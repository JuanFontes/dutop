# D1 verification — 2026-11-27

The first local Rust delivery is implemented on `dev`, without commits or pushes.
The terminal layout remains the two existing panels; the visual redesign and
distribution packages are separate deliveries.

## Verified behavior

- Native filesystem accounting; no Python or external `du` process at runtime.
- `dutop [directory]`, `--top N`/`-n N` (default 10), help and version.
- Immediate-child rankings, hidden entries, complete allocated totals, sparse
  files, deterministic hard-link accounting and partial-read status.
- Descriptor-relative traversal with no-follow directory opens. An already opened
  directory cannot be redirected by replacing its original pathname with a symlink.
- Narrow/wide panels, bounded empty padding, escaped filename controls and invalid
  UTF-8, grapheme-aware clipping, optional tty colors and uncolored piped output.
- SIGINT stops scanning or a blocked output pipe with status 130.

## Evidence

| Check | Result |
|---|---|
| macOS ARM64 Rust tests | 24 passed: 2 unit, 7 scan, 8 render, 7 CLI |
| Linux ARM64 Rust tests | The same 24 passed, including non-root permission fixtures |
| macOS PTY checks | 6 passed: colors, NO_COLOR, TERM=dumb, narrow layout, scan cancellation, blocked-pipe cancellation |
| `cargo fmt --check` | Passed |
| `cargo clippy --offline --all-targets -- -D warnings` | Passed |
| Native and Linux release builds | Passed |
| Installed version | `dutop 0.1.0-dev.1` |
| Installed file | Native ARM64 Mach-O, 878448 bytes; release hash matches |
| Fresh interactive zsh | No dutop alias; command resolves to `~/.local/bin/dutop`; fixture scan passed |

Linux was tested in the official `rust:1.100.0-slim` container, digest
`sha256:3b199fd6db7d709274df56565b8f4003f9e30d7bfcff9d53148c86053b7855e2`, as
UID 1000 with networking disabled, no capabilities, and read-only host mounts.
Source and vendored dependencies were copied to disposable container storage.
No existing containers were modified.

The six PTY checks used the optional Python developer harness in `tests/terminal.py`;
Python is not used by the installed application. A native interactive terminal
report was also inspected as text. No Ghostty screenshot comparison was performed.

The first implementation exposed five review findings. Regression tests reproduced
each before the fixes: pathname replacement, emoji clipping, blocked output SIGINT,
excess empty rows in stacked panels, and unescaped format controls/CLI arguments.
The affected Rust tests passed on both operating systems after the fixes.

## Installation and rollback

- Installed executable: `~/.local/bin/dutop` (regular file, not a build-directory link).
- Removed exactly one alias line from `~/.zshrc`; shell syntax verified.
- Original Python source remains unchanged in `~/.local/share/dutop/dutop.py`.
- Backup: `~/.local/share/dutop/backups/20261127T065312/` contains the previous
  `.zshrc`, a `dutop-python` link and an installation manifest with hashes.
- Already-open shells may keep their alias in memory. Run `unalias dutop 2>/dev/null`
  followed by `rehash`, or open a new shell. The old alias points to the same path
  and therefore also reaches the newly installed binary in the meantime.

## Deferred and unverified

- Visual redesign, additional flags, Dockerfile/Compose, CI, full README, license
  files, `.deb`, portable release archives and Homebrew packages belong to D2–D4.
- x86_64 builds, minimum OS versions and the declared Rust 1.85 compiler floor
  have not been executed in D1. Validation used Rust 1.100.0 on ARM64.
- Windows is deferred; AppImage still requires a product decision.
- Minor review findings deferred: long link names may truncate their `[LINK]`
  marker, and CLI help does not yet explain differing hard-link allocation when a
  subfolder is scanned independently. The design document specifies that behavior.

## Reproduce the focused checks

```sh
cargo test --locked --lib --test scan --test render --test cli
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
python3 tests/terminal.py target/debug/dutop
cargo build --release --locked
```
