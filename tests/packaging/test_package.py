import hashlib
import importlib.util
from pathlib import Path
import tarfile
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[2] / 'scripts' / 'package.py'
spec = importlib.util.spec_from_file_location('package', SCRIPT)
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)
sys.path.insert(0, str(SCRIPT.parent))
from prepare_tap import prepare


class PackageTests(unittest.TestCase):
    def test_archive_contains_executable_and_all_notices(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            stage = root / 'stage'
            stage.mkdir()
            (stage / 'dutop').write_bytes(b'executable')
            (stage / 'dutop').chmod(0o755)
            (stage / 'LICENSE').write_text('MIT notice')
            (stage / 'licenses').mkdir()
            (stage / 'licenses' / 'dependency.txt').write_text('dependency notice')
            archive = package.archive(stage, root, '0.1.0-beta.1', 'aarch64-apple-darwin')
            with tarfile.open(archive) as stream:
                members = {item.name: item for item in stream.getmembers()}
                prefix = 'dutop-0.1.0-beta.1-aarch64-apple-darwin/'
                self.assertTrue(members[prefix + 'dutop'].mode & 0o111)
                self.assertIn(prefix + 'LICENSE', members)
                for member in members.values():
                    self.assertEqual((member.uid, member.gid), (0, 0))
                    if member.isdir():
                        self.assertEqual(member.mode, 0o755)
                self.assertEqual(stream.extractfile(prefix + 'licenses/dependency.txt').read(), b'dependency notice')
            manifest = package.checksums([archive], root)
            self.assertEqual(manifest.read_text(), hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + archive.name + '\n')

    def test_rejects_unsupported_targets_and_path_like_versions(self):
        for version, target in [('0.1/../../escape', 'aarch64-apple-darwin'), ('0.1.0', 'windows')]:
            with self.assertRaises(ValueError):
                package.validate(version, target)

    def test_formula_selects_all_four_verified_checksums(self):
        manifest = '\n'.join(f'{str(index) * 64}  dutop-0.1.0-beta.1-{target}.tar.gz' for index, target in enumerate(package.TARGETS, 1))
        formula = package.formula('0.1.0-beta.1', manifest)
        self.assertIn('class Dutop < Formula', formula)
        for index, target in enumerate(package.TARGETS, 1):
            self.assertIn(target + '.tar.gz', formula)
            self.assertIn(f'sha256 "{str(index) * 64}"', formula)
        self.assertIn('bin.install "dutop"', formula)
        self.assertNotIn('depends_on "rust"', formula)
        with self.assertRaises(ValueError):
            package.formula('0.1.0-beta.1', manifest.splitlines()[0])
        with self.assertRaises(ValueError):
            package.formula('0.1.0-beta.1', manifest + '\n' + manifest.splitlines()[0])

    def test_wrong_binary_version_is_not_packaged(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / 'dutop'
            binary.write_text('#!/bin/sh\necho "dutop 99.0.0"\n')
            binary.chmod(0o755)
            result = subprocess.run([sys.executable, str(SCRIPT), 'build',
                '--target', 'aarch64-apple-darwin', '--binary', str(binary),
                '--output', str(root / 'output')], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('binary version mismatch', result.stderr)
            self.assertFalse((root / 'output').exists())

    def test_tap_checks_real_bytes_and_rejects_tampering(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            paths = []
            for target in package.TARGETS:
                directory = root / target
                directory.mkdir()
                path = directory / f'dutop-0.1.0-beta.1-{target}.tar.gz'
                path.write_bytes(target.encode())
                package.checksums([path], directory)
                paths.append(path)
            for target, arch in [('aarch64-unknown-linux-musl', 'arm64'), ('x86_64-unknown-linux-musl', 'amd64')]:
                directory = root / target
                deb = directory / f'dutop_0.1.0-beta.1_{arch}.deb'
                deb.write_bytes(arch.encode())
                package.checksums([directory / f'dutop-0.1.0-beta.1-{target}.tar.gz', deb], directory)
            generated = prepare(root, '0.1.0-beta.1', local=True)
            for path in paths:
                self.assertIn(path.resolve().as_uri(), generated)
            paths[0].write_bytes(b'tampered')
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                prepare(root, '0.1.0-beta.1')

    def test_release_rejects_missing_debs_and_unlisted_artifacts(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            files = []
            for target in package.TARGETS:
                path = root / f'dutop-0.1.0-beta.1-{target}.tar.gz'
                path.write_bytes(b'archive')
                files.append(path)
            package.checksums(files, root)
            with self.assertRaisesRegex(ValueError, 'artifact set'):
                prepare(root, '0.1.0-beta.1')
            for arch in ('arm64', 'amd64'):
                path = root / f'dutop_0.1.0-beta.1_{arch}.deb'
                path.write_bytes(b'deb')
                files.append(path)
            package.checksums(files, root)
            (root / 'unlisted.deb').write_bytes(b'extra')
            with self.assertRaisesRegex(ValueError, 'unlisted'):
                prepare(root, '0.1.0-beta.1')

    def test_linux_packages_include_the_static_musl_copyright(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'licenses'
            package.notices(output, 'aarch64-unknown-linux-musl')
            text = (output / 'musl-COPYRIGHT').read_text()
            self.assertIn('Rich Felker', text)
            self.assertIn('THE SOFTWARE IS PROVIDED', text)

    def test_release_names_are_safe_for_github_uploads(self):
        names = package.artifact_names('0.1.0-beta.1')
        self.assertIn('dutop_0.1.0-beta.1_arm64.deb', names)
        self.assertIn('dutop_0.1.0-beta.1_amd64.deb', names)
        self.assertEqual(len(names), 6)
        self.assertTrue(all('~' not in name for name in names))


if __name__ == '__main__':
    unittest.main()
