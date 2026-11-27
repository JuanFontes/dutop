# Bundled native-library notices

`musl-COPYRIGHT` is the unmodified musl 1.2.5 notice from:
https://git.musl-libc.org/cgit/musl/plain/COPYRIGHT?h=v1.2.5

The release compiler is Rust 1.100.0. Its musl target libraries use musl 1.2.5
with upstream security patches; see the pinned compiler's build scripts:
https://github.com/rust-lang/rust/blob/1.100.0/src/ci/docker/scripts/musl.sh
https://github.com/rust-lang/rust/blob/1.100.0/src/ci/docker/scripts/musl-toolchain.sh

The notice is copied into every Linux archive and Debian package. Crate notices
come from the locked dependency sources; Rust standard library attribution comes
from the release compiler distribution. Review these sources when upgrading the
compiler. The package builder deliberately rejects a different release compiler.
