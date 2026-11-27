# Releasing dutop

Release executables do not require Rust or Python. Python 3.12+ is used only by
packaging and terminal checks. The release compiler is Rust 1.100.0; CI separately
checks the declared Rust 1.85 minimum.

## Supported artifacts

| Platform | Target | Archive | Debian |
| --- | --- | --- | --- |
| macOS ARM64 | `aarch64-apple-darwin` | `.tar.gz` | — |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` | — |
| Linux ARM64 | `aarch64-unknown-linux-musl` | `.tar.gz` | `arm64.deb` |
| Linux AMD64 | `x86_64-unknown-linux-musl` | `.tar.gz` | `amd64.deb` |

macOS has deployment target 13.0; native CI runs on macOS 15, not every older
macOS version. Linux is statically linked with musl; native CI runs on Ubuntu
24.04. Static linking avoids a host glibc version dependency, but does not promise
compatibility with every kernel or filesystem.

## Validate a candidate

1. Update Cargo version and lockfile together, and edit `docs/release-notes.md`.
2. Open a development PR. CI tests all four native targets, Rust 1.85, terminal
   behavior, archive contents, Debian install/uninstall and Homebrew install/test/
   uninstall. Permission checks run as unprivileged users.
3. Review the source, dependency changes, license notices and CI evidence.
4. Integrate the reviewed commit and create its exact version tag, for example
   `v0.1.0`. Do not move a published tag or replace an existing artifact.
5. Dispatch **Draft release** at that tag, with the same tag as its input. It checks
   the version and commit, runs the package matrix, verifies checksums, then creates
   a draft release (marked as a prerelease only for a prerelease tag). Nothing is automatically published to the public tap.
6. Download and inspect the draft assets. The generated `dutop.rb` uses all four
   verified archive SHA256 values. Publish the reviewed release, then update
   `Formula/dutop.rb` in `JuanFontes/homebrew-tap` and run its focused package checks.
7. Verify a fresh install from the actual published tap:
   `brew install juanfontes/tap/dutop`, `brew test juanfontes/tap/dutop`, and
   `dutop --version`. Check `type -a dutop` for a prior manual installation that
   shadows Homebrew. Preserve that manual binary before changing it.

For the first release, the source repository must become public before publishing
a formula for anonymous downloads. Review visibility separately from a private CI
run. A draft or a formula pointing at private assets is not a working public install.

## Local package build

On a matching native host, build the target with Rust 1.100.0, then:

```sh
python3 scripts/package.py build \
  --target aarch64-apple-darwin \
  --binary target/aarch64-apple-darwin/release/dutop
```

Use `MACOSX_DEPLOYMENT_TARGET=13.0` for macOS release builds. Linux packaging needs
`dpkg-deb`, `musl-tools` and the matching Rust musl target. Package generation checks
the binary's version and includes dependency and Rust standard library notices.
Archive checksums cover every byte, including license documents.

`prepare_tap.py` accepts the separate artifact directories downloaded from CI,
checks every manifest against its file, and rejects duplicate or missing archives:

```sh
python3 scripts/prepare_tap.py --artifacts artifacts --version 0.1.0 > dutop.rb
```

`--local` changes URLs to verified local files for an isolated Homebrew candidate
test. Never publish that local formula.

## Optional Docker build

Docker is for producing Linux packages, not for scanning your host. No daemon,
Compose service or Docker socket mount is required by dutop itself.

```sh
docker build --target artifacts --output type=local,dest=dist/linux-arm64 \
  --platform linux/arm64 .
docker build --target artifacts --output type=local,dest=dist/linux-amd64 \
  --platform linux/amd64 .
```

The image pins the Rust base digest and Cargo.lock pins Rust dependencies. OS
build-tool packages still follow the base distribution's repositories; this is
not a claim of bit-for-bit reproducibility. Builds and filesystem tests run as an
unprivileged builder. Use native CI results as the release evidence; emulation is
useful for a local check but does not replace testing on the intended architecture.

Docker cannot produce the macOS artifacts. Build them on the native macOS runners.

## Homebrew terminology

The tap installs our native release archives. These are **not Homebrew bottles**:
bottles are a separate Homebrew build and relocation format. Neither approach
requires end users to install Rust, but their publishing workflows differ.
