# D2 verification — 2026-11-27

## Approved follow-up: limit and relative heat colors (dev.3)

The user approved the D2 appearance and then requested these adjustments:

- `--top` accepts 1–99; larger values fail before scanning. The renderer also
  caps its input at 99, resolving the earlier three-digit-rank minor.
- Dotted row separators removed. Borders and the column-heading rule retained.
- With `--top > 3`, sizes and percentages use the same relative heat color:
  strongest red for the largest entry of each panel, smoothly through yellow
  toward the existing green. Entries at or below 20% of that panel maximum keep
  their original green size / cyan percentage. The yellow anchor is 55% of the
  panel maximum. Numeric percentages still refer to the complete directory total.
- With `--top <= 3`, colors remain unchanged. NO_COLOR and pipes remain uncolored.

Validation: 17 renderer + 8 CLI tests, 4 presentation PTY checks, fmt, clippy and
release build pass. Boundary failures at top100 and the heat-color regression
were observed before implementation. Installed release verified with a PTY at
top3/4/99, plus rejection of top100 and absence of dotted separator rows.

Current version: `0.1.0-dev.3`, 911,728 bytes; SHA-256
`5ce44cf074cdb8e8193f050359166dadcb677c9401acaaa3f9f9ac5295451858`.
Previous dev.2 backup:
`/Users/juanfontes/.local/share/dutop/backups/20261127T073628-d2/dutop`.
No README edit, commit, push or runtime dependency added. The following sections
record the original dev.2 delivery; its UI-capture limitation still applies.

## Original D2 delivery (dev.2)

Implemented the user-approved terminal design in `design/dutop-approved.png`:
rank/name/size/percentage columns, shown/complete counts, cyan headings, light
names, green sizes, muted borders, grouped rows and a complete summary.
The scanner and disk-accounting contract are unchanged. No dependencies added.

## Checks

- `cargo test --offline --test render --test cli`: 15 renderer + 7 CLI checks pass.
- Presentation-specific PTY checks: 4 pass (ANSI, NO_COLOR, TERM=dumb, 48 columns).
- `cargo fmt --check` and `cargo clippy --offline --lib --bin dutop -- -D warnings` pass.
- `cargo build --release --offline --locked` passes on macOS ARM64.
- Actual CLI output captured at 48 and 160 cells using a controlled temporary
  directory, then inspected as text. No content from the user's Downloads used.
- Installed release matches the built binary by SHA-256. Its `--top 4`, counts,
  percentages, uncolored pipe output and 48/160-column layouts were verified.

Tests first failed on missing D2 behaviors. One independent read-only review
then found three issues, each reproduced in a failing regression and fixed:

1. Ten-cell size strings such as `1004.0 KiB` now retain column/border alignment.
2. Unicode ligatures use consistent whole-substring widths when clipping/padding.
3. Long symlink names preserve a short compound extension and `[LINK]` together.

Existing test bounds were adapted to the additional heading/separator lines;
the long-filename fixture was enlarged to actually overflow at 160 columns.

## Local installation

- Version: `0.1.0-dev.2`, native ARM64 executable, 911,712 bytes.
- Path: `/Users/juanfontes/.local/bin/dutop`.
- SHA-256: `cfeee7de8c23a1646945d5aa65aa81fbdbaf6d05a1247c53e1bc7b0646cbe8f9`.
- Previous binary: `/Users/juanfontes/.local/share/dutop/backups/20261127T072738-d2/dutop`.
- This update changed only the installed binary and created its backup; no shell
  or font configuration changed. Source remains uncommitted on `dev`.

## Limits and follow-up

- Pixel comparison in Ghostty is **not verified**: computer-use access to Ghostty
  was denied and the browser preview host was unavailable. Terminal geometry and
  ANSI output were checked; appearance in the user's font still needs inspection.
- Minor deferred: when showing 100+ entries, ranks are padded to three digits
  (`001`) instead of two digits aligned with spaces. Ordering is unaffected.
- Linux D2 execution, x86_64 and Rust 1.85 were not rerun in this output-only
  delivery. D1's Linux results remain recorded separately, not claimed as D2 tests.
- `--json`, extra size/filter flags, Docker/Compose, packaging and public release
  remain separate deliveries. No README rewrite, commit or publication occurred.
