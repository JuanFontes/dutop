The first native Rust preview of **dutop**: big folders, big files, one clear view.

- Separate ranked panels for immediate folders and files; full allocated-space totals.
- `--top 1` through `--top 99`, responsive layouts, safe Unicode and relative heat colors.
- Hidden entries included, child symlink targets excluded, deterministic hard-link accounting.
- Explicit partial results, shell-friendly output and Ctrl+C cancellation.
- Prebuilt macOS and Linux archives for ARM64 and x86_64, with dependency notices and SHA256 checksums.
- Debian packages for ARM64 and AMD64; installation through `JuanFontes/tap`.

This is a beta. macOS binaries target macOS 13 or newer and are tested on macOS 15.
Linux binaries are statically linked with musl and tested on Ubuntu 24.04.
Windows is not supported. Allocated space is not a guarantee of space freed by deletion.

Install after the corresponding tap formula is published:

```sh
brew install juanfontes/tap/dutop
```

For manual downloads, verify the archive or Debian package against `SHA256SUMS`.
For `.deb` files, use `sudo apt install ./dutop_0.1.0-beta.1_amd64.deb`
(or the `arm64` package). See the README for usage and accounting details.
