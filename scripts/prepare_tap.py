#!/usr/bin/env python3
"""Verify exactly six release artifacts, then generate the tap or release bundle."""
import argparse
import hashlib
from pathlib import Path
import re
import shutil

from package import artifact_names, checksums, formula


def verified_artifacts(directory, version):
    paths = {}
    for manifest in sorted(directory.rglob('SHA256SUMS')):
        for line in manifest.read_text().splitlines():
            match = re.fullmatch(r'([0-9a-f]{64})  ([^/\s]+)', line)
            if not match:
                raise ValueError(f'invalid checksum in {manifest}')
            digest, name = match.groups()
            if name in paths:
                raise ValueError(f'duplicate artifact: {name}')
            artifact = manifest.parent / name
            if not artifact.is_file() or artifact.is_symlink():
                raise ValueError(f'missing or symbolic artifact: {artifact}')
            if hashlib.sha256(artifact.read_bytes()).hexdigest() != digest:
                raise ValueError(f'checksum mismatch: {artifact}')
            paths[name] = artifact
    if set(paths) != artifact_names(version):
        raise ValueError('release artifact set must contain the four archives and two Debian packages')
    found = {p.absolute() for p in directory.rglob('*')
             if p.name.endswith(('.tar.gz', '.deb'))}
    if found != {p.absolute() for p in paths.values()}:
        raise ValueError('unlisted release artifact')
    return paths


def prepare(directory, version, local=False):
    paths = verified_artifacts(directory, version)
    lines = [f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {name}'
             for name, path in sorted(paths.items())]
    result = formula(version, '\n'.join(lines))
    if local:
        for name, path in paths.items():
            result = result.replace(f'https://github.com/JuanFontes/dutop/releases/download/v{version}/{name}',
                                    path.resolve().as_uri())
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--local', action='store_true')
    parser.add_argument('--bundle', type=Path, help='Create a new verified release directory')
    args = parser.parse_args()
    result = prepare(args.artifacts, args.version, args.local)
    if args.bundle:
        if args.local:
            parser.error('--bundle cannot be used with --local')
        paths = verified_artifacts(args.artifacts, args.version)
        args.bundle.mkdir(exist_ok=False)
        for name, path in paths.items():
            shutil.copy2(path, args.bundle / name)
        checksums([args.bundle / name for name in paths], args.bundle)
        verified_artifacts(args.bundle, args.version)
        (args.bundle / 'dutop.rb').write_text(result)
    else:
        print(result, end='')
