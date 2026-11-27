#!/usr/bin/env python3
"""Exercise an installed or unpacked executable, without a build toolchain."""
import argparse
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile


def check(binary, version):
    binary = str(binary.resolve(strict=True))
    assert subprocess.check_output([binary, '--version'], text=True).strip() == f'dutop {version}'
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        (root / 'folder').mkdir()
        (root / 'folder/data').write_bytes(b'x' * 8192)
        (root / 'file').write_bytes(b'x' * 4096)
        (root / '.hidden').write_bytes(b'x' * 4096)
        (root / 'link').symlink_to(root / 'folder')
        output = subprocess.check_output([binary, '--top', '4', temp], text=True,
                                         env=dict(os.environ, NO_COLOR='1', COLUMNS='120'))
        for expected in ('FOLDERS 1 / 1', 'FILES 3 / 3', 'TOTAL', 'folder/', '.hidden', '[LINK]'):
            assert expected in output, (expected, output)
        assert '\x1b' not in output
        invalid = subprocess.run([binary, '--top', '100'], capture_output=True, text=True)
        assert invalid.returncode == 2
        assert 'between 1 and 99' in invalid.stderr
    print(f'Package smoke passed: {binary}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--binary', type=Path)
    source.add_argument('--archive', type=Path)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    if args.archive:
        with tempfile.TemporaryDirectory() as temp:
            with tarfile.open(args.archive) as archive:
                archive.extractall(temp, filter='data')
            extracted = list(Path(temp).iterdir())
            assert len(extracted) == 1
            root = extracted[0]
            assert (root / 'LICENSE').is_file()
            assert (root / 'licenses/INDEX.txt').is_file()
            assert (root / 'licenses/rust-standard-library/COPYRIGHT-library.html').is_file()
            if 'linux' in args.archive.name:
                assert (root / 'licenses/musl-COPYRIGHT').is_file()
            check(root / 'dutop', args.version)
    else:
        check(args.binary, args.version)
