#!/usr/bin/env python3
"""Build native release archives, Debian packages, and a checksummed tap formula.

Developer tooling only. Installed dutop binaries have no Python dependency.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = (
    'aarch64-apple-darwin', 'x86_64-apple-darwin',
    'aarch64-unknown-linux-musl', 'x86_64-unknown-linux-musl',
)


def validate(version, target):
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?', version):
        raise ValueError('expected a release version, for example 0.1.0-beta.1')
    if target not in TARGETS:
        raise ValueError(f'unsupported release target: {target}')


def archive(stage, output, version, target):
    validate(version, target)
    name = f'dutop-{version}-{target}'
    path = output / f'{name}.tar.gz'
    def normalize(info):
        info.uid = info.gid = 0
        info.uname = info.gname = 'root'
        info.mode = 0o755 if info.isdir() or info.name == f'{name}/dutop' else 0o644
        return info
    with tarfile.open(path, 'w:gz') as stream:
        stream.add(stage, arcname=name, filter=normalize)
    return path


def artifact_names(version):
    validate(version, TARGETS[0])
    return {f'dutop-{version}-{target}.tar.gz' for target in TARGETS} | {
        f'dutop_{version}_{arch}.deb' for arch in ('arm64', 'amd64')}


def checksums(files, output):
    path = output / 'SHA256SUMS'
    path.write_text(''.join(
        f'{hashlib.sha256(item.read_bytes()).hexdigest()}  {item.name}\n'
        for item in sorted(files)))
    return path


def notices(destination, target):
    """Retain licenses for this target's normal and build dependency graph."""
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--offline', '--filter-platform', target,
         '--format-version', '1'], cwd=ROOT))
    tree = subprocess.check_output(
        ['cargo', 'tree', '--locked', '--offline', '--target', target,
         '--edges', 'normal,build', '--prefix', 'none', '--format', '{p}'],
        cwd=ROOT, text=True)
    included = {tuple(line.split()[:2]) for line in tree.splitlines()}
    destination.mkdir()
    index = ['Third-party dependency notices', '',
             'Normal and build dependencies for the packaged target are listed.', '']
    for dependency in sorted(metadata['packages'], key=lambda item: item['name']):
        if dependency['name'] == 'dutop' or (dependency['name'], 'v' + dependency['version']) not in included:
            continue
        source = Path(dependency['manifest_path']).parent
        name = f"{dependency['name']}-{dependency['version']}"
        found = [p for p in source.iterdir()
                 if p.is_file() and p.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE', 'COPYRIGHT'))]
        if not found:
            raise ValueError(f'missing upstream license files: {name}')
        (destination / name).mkdir()
        for path in found:
            shutil.copy2(path, destination / name / path.name)
        index.append(f"{name}: {dependency['license']}")
    sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
    candidates = [sysroot / 'share/doc/rust', sysroot / 'share/doc/rustc']
    rust_docs = next((p for p in candidates if (p / 'COPYRIGHT-library.html').is_file()), None)
    if rust_docs is None:
        raise ValueError('Rust standard library notices missing from compiler distribution')
    rust_destination = destination / 'rust-standard-library'
    rust_destination.mkdir()
    shutil.copy2(rust_docs / 'COPYRIGHT-library.html', rust_destination)
    if (rust_docs / 'licenses').is_dir():
        shutil.copytree(rust_docs / 'licenses', rust_destination / 'licenses')
    if 'linux' in target:
        shutil.copy2(ROOT / 'packaging/musl-COPYRIGHT', destination / 'musl-COPYRIGHT')
        index.extend(['', 'musl 1.2.5 (Rust 1.100.0 target distribution): musl-COPYRIGHT'])
    index.extend(['', subprocess.check_output(['rustc', '--version'], text=True).strip(),
                  'Rust standard library attribution: rust-standard-library/COPYRIGHT-library.html'])
    (destination / 'INDEX.txt').write_text('\n'.join(index) + '\n')


def debian(stage, output, version, target):
    architecture = 'arm64' if target.startswith('aarch64') else 'amd64'
    deb_version = version.replace('-', '~', 1)
    path = output / f'dutop_{version}_{architecture}.deb'
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        root.chmod(0o755)
        (root / 'usr/bin').mkdir(parents=True)
        shutil.copy2(stage / 'dutop', root / 'usr/bin/dutop')
        documentation = root / 'usr/share/doc/dutop'
        documentation.mkdir(parents=True)
        shutil.copy2(stage / 'LICENSE', documentation / 'copyright')
        shutil.copytree(stage / 'licenses', documentation / 'licenses')
        (root / 'DEBIAN').mkdir()
        (root / 'DEBIAN/control').write_text(
            f'Package: dutop\nVersion: {deb_version}\nArchitecture: {architecture}\n'
            'Maintainer: Juan Fontes <juanfontes@users.noreply.github.com>\n'
            'Section: utils\nPriority: optional\n'
            f'Installed-Size: {sum(p.stat().st_size for p in root.rglob("*") if p.is_file()) // 1024 + 1}\n'
            'Homepage: https://github.com/JuanFontes/dutop\n'
            'Description: Ranked folder and file disk usage in your terminal\n'
            ' Native allocated-space scanner with complete totals and responsive panels.\n')
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(root), str(path)], check=True)
    return path


def formula(version, manifest):
    validate(version, TARGETS[0])
    entries = {}
    for line in manifest.splitlines():
        match = re.fullmatch(r'([0-9a-f]{64})  ([^/\s]+)', line)
        if not match:
            raise ValueError('invalid SHA256SUMS line')
        digest, name = match.groups()
        if name in entries:
            raise ValueError(f'duplicate checksum: {name}')
        entries[name] = digest

    def artifact(target, indent):
        name = f'dutop-{version}-{target}.tar.gz'
        if name not in entries:
            raise ValueError(f'missing checksum: {name}')
        return (f'{indent}url "https://github.com/JuanFontes/dutop/releases/download/v{version}/{name}"\n'
                f'{indent}sha256 "{entries[name]}"\n')

    return (f'''class Dutop < Formula
  desc "Big folders, big files, and complete disk usage in two ranked panels"
  homepage "https://github.com/JuanFontes/dutop"
  version "{version}"
  license "MIT"

  on_macos do
    depends_on macos: :ventura
    on_arm do
''' + artifact(TARGETS[0], '      ') + '''    end
    on_intel do
''' + artifact(TARGETS[1], '      ') + '''    end
  end

  on_linux do
    on_arm do
''' + artifact(TARGETS[2], '      ') + '''    end
    on_intel do
''' + artifact(TARGETS[3], '      ') + '''    end
  end

  def install
    bin.install "dutop"
    pkgshare.install "licenses"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dutop --version")
    (testpath/"sample/folder").mkpath
    (testpath/"sample/folder/data").write("x" * 8192)
    (testpath/"sample/file").write("x" * 4096)
    output = shell_output("#{bin}/dutop --top 4 #{testpath}/sample")
    assert_match "FOLDERS 1 / 1", output
    assert_match "FILES 1 / 1", output
    assert_match "TOTAL", output
    assert_match "folder/", output
    assert_match "between 1 and 99", shell_output("#{bin}/dutop --top 100 2>&1", 2)
  end
end
''')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    build = commands.add_parser('build')
    build.add_argument('--target', choices=TARGETS, required=True)
    build.add_argument('--binary', type=Path, required=True)
    build.add_argument('--output', type=Path, default=ROOT / 'dist')
    tap = commands.add_parser('formula')
    tap.add_argument('--version', required=True)
    tap.add_argument('--checksums', type=Path, required=True)
    args = parser.parse_args()
    if args.command == 'formula':
        print(formula(args.version, args.checksums.read_text()), end='')
        return
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--offline', '--no-deps', '--format-version', '1'], cwd=ROOT))
    version = next(p['version'] for p in metadata['packages'] if p['name'] == 'dutop')
    validate(version, args.target)
    compiler = subprocess.check_output(['rustc', '--version'], text=True)
    if not compiler.startswith('rustc 1.100.0 '):
        raise ValueError('release packages require Rust 1.100.0 and its corresponding notices')
    binary = args.binary.resolve(strict=True)
    expected = f'dutop {version}'
    actual = subprocess.check_output([str(binary), '--version'], text=True).strip()
    if actual != expected:
        raise ValueError(f'binary version mismatch: expected {expected}, got {actual}')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temp:
        stage = Path(temp)
        shutil.copy2(binary, stage / 'dutop')
        (stage / 'dutop').chmod(0o755)
        for name in ('LICENSE', 'README.md'):
            shutil.copy2(ROOT / name, stage / name)
        notices(stage / 'licenses', args.target)
        files = [archive(stage, output, version, args.target)]
        if 'linux' in args.target:
            files.append(debian(stage, output, version, args.target))
        checksums(files, output)
        for path in files:
            print(path)


if __name__ == '__main__':
    main()
