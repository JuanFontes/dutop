# Contributing to dutop

Thanks for helping make disk usage easier to understand.

You can contribute a bug report, a clearer example, a filesystem edge case, a
platform check, or a code change. Small improvements are welcome; you do not need
to propose a large feature to help.

## Start with the problem

For a bug, open an issue with:

- The command you ran and the output you expected.
- What happened, including the exit code when relevant.
- `dutop --version`, operating system and CPU architecture.
- Filesystem type for accounting issues; terminal, width and color settings for
  rendering issues.
- A small reproduction using sample files, or a screenshot when the layout matters.

A temporary directory with a few files is usually more useful than a report from
an entire home directory. If the issue involves sparse files, hard links, symlinks
or permissions, include that detail.

For a new feature, describe the job it helps someone do and an example of the
desired command or output. Discuss changes to accounting, default behavior,
platform support or dependencies before investing in a large implementation.

## Set up a checkout

Fork and clone the repository, then create a branch for your change. Development
currently happens on `dev`; base contributions on that branch during this preview.

```sh
git switch dev
git switch -c feat/short-description
cargo run --locked -- --help
```

Run the local binary against a small directory while iterating:

```sh
cargo run --locked -- --top 4 ./src
```

Use `--release` when checking scan performance. Debug-build timings are not a
meaningful comparison with installed release binaries.

### Toolchain

- macOS or Linux with Rust and Cargo.
- Current development checks use **Rust 1.100.0**.
- **Rust 1.85** is the tested minimum for the locked dependency graph. CI keeps
  this check separate from builds with the release compiler.
- Python 3.12+ is used by packaging and optional terminal tests. It is not a
  runtime dependency of `dutop`.
- The scanner integration tests compare a controlled fixture with `/usr/bin/du`.
  The application itself does not invoke `du`.

Keep `Cargo.lock` checked in. Use `--locked` for builds and tests so dependency
changes are deliberate. Add `--offline` only when the required dependencies are
already cached.

## Find your way around

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | Command execution, terminal detection, progress, cancellation and exit handling. |
| `src/cli.rs` | Arguments, validation, help and examples. |
| `src/lib.rs` | Public modules and the shared top-count limit. |
| `src/report.rs` | Report and entry data types. |
| `src/scan/mod.rs` | Traversal, allocation accounting, ranking and partial results. |
| `src/scan/unix.rs` | Native metadata and descriptor-relative filesystem access. |
| `src/render.rs` | Header, panel layout and report summary. |
| `src/render/panel.rs` | Ranked rows, numeric columns, filename suffixes and heat colors. |
| `src/render/text.rs` | Safe text, terminal-cell widths, clipping and ANSI styling. |
| `tests/scan.rs` | Filesystem and accounting integration tests. |
| `tests/render.rs` | Layout, percentages, colors, Unicode and clipping tests. |
| `tests/cli.rs` | Public command behavior and exit codes. |
| `tests/terminal.py` | Real pseudo-terminal checks for colors, layout and interruption. |

The scanner produces a report; rendering consumes it. Keep filesystem work out of
the renderer and presentation decisions out of the scanner. Put small helpers
beside the behavior they support.

## Test the change you make

Start with the affected behavior and its direct dependents. **Do not run the full
suite automatically for every edit or documentation change.** Expand the checks
when the change crosses boundaries, and explain that impact in the pull request.

| If you change… | Start with… |
| --- | --- |
| Layout, colors, Unicode or clipping | `cargo test --locked --test render` |
| Arguments, defaults, help or exit behavior | `cargo test --locked --test cli` |
| Traversal or allocation accounting | `cargo test --locked --lib scan::tests` and `cargo test --locked --test scan` |
| Shared report data or scan-to-output behavior | The directly affected scanner, renderer and CLI tests above. |
| Documentation only | Review the diff, links and examples; run a documented command only when its behavior needs verification. |

For a specific regression, narrow the command further:

```sh
cargo test --locked --test render clipped_symlink_preserves_compound_extension_and_marker_together
```

For a bug fix, reproduce the bug with a focused test before changing the code,
then confirm that the test passes with the fix. Prefer real temporary files and
real output over mocks. Derive expected values independently of the implementation.

When touching Rust code, also check formatting and lint the application:

```sh
cargo fmt --check
cargo clippy --locked --lib --bin dutop -- -D warnings
```

Run `cargo fmt` to apply formatting. If you change dependencies, review the
`Cargo.toml` and `Cargo.lock` diffs together and explain why the dependency is needed.

### Terminal behavior

After building the binary, these checks cover presentation in an actual
pseudo-terminal:

```sh
cargo build --locked
python3 tests/terminal.py target/debug/dutop \
  TerminalTests.test_tty_has_colors_and_no_alternate_screen \
  TerminalTests.test_no_color_disables_colored_report \
  TerminalTests.test_dumb_terminal_has_no_escape_sequences \
  TerminalTests.test_narrow_terminal_stacks_panels
```

When changing cancellation or output handling, select the relevant interruption
checks as well:

```sh
python3 tests/terminal.py target/debug/dutop \
  TerminalTests.test_ctrl_c_cancels_scan_with_status_130 \
  TerminalTests.test_sigint_interrupts_a_blocked_output_pipe
```

Permission tests need an unprivileged user to exercise unreadable paths. A test
running with elevated filesystem access may skip that condition; report the skip
instead of treating it as verified. Claim a platform result only after checking
that platform.

### Packaging and release changes

Run `python3 -m unittest discover -s tests/packaging -v` for artifact/formula
logic, then exercise the actual changed package with `tests/package_smoke.py`.
Review workflow changes with actionlint. A compatibility or release change needs
native CI evidence for the affected targets; local emulation is not that evidence.

See [releasing.md](docs/releasing.md) for native targets, Docker builds, license
notices, draft releases and tap maintenance. The CI matrix checks install and
uninstall in disposable runners. Do not run those uninstall steps against a
contributor's working installation without intending to remove it.

## Preserve the contracts

Changes should keep these behaviors unless an explicitly discussed feature
changes them:

- **Allocation, not logical length.** Use filesystem allocation metadata without
  reading file contents. Keep hard-link accounting deterministic.
- **First-level rankings, complete totals.** Hidden entries and entries outside
  the displayed top count contribute to the total.
- **Bounded display.** `--top` is 1–99 per panel, defaulting to 10. A count controls
  the presentation, not the depth or completeness of the scan.
- **Explicit partial results.** Read errors must not turn incomplete data into a
  successful-looking complete total.
- **Correct link handling.** Do not follow child symlink targets or let a renamed
  ancestor redirect traversal. An explicit root symlink may resolve to its target.
- **Readable terminal output.** Escape filename controls and invalid bytes. Measure
  display cells consistently, keep numeric columns aligned, and preserve meaningful
  name endings when space allows.
- **Honest color semantics.** Heat colors compare against the panel maximum;
  printed percentages use the full directory total, or the known partial total.
- **Shell-friendly behavior.** Honor NO_COLOR, TERM=dumb and piped stdout. Keep the
  report on stdout, progress/errors on stderr, and cancellation at exit code 130.

Rust code forbids `unsafe` in this package. New dependencies and platform-specific
behavior need a concrete reason and checks for the affected paths.

## Open a focused pull request

Target `dev` during the current preview. Describe:

1. The problem and a concrete before/after example.
2. The resulting behavior and any compatibility or accounting impact.
3. The exact checks you ran, with the operating system and architecture.
4. Any failures, skipped conditions or behavior you could not verify.

Include wide and narrow output examples for visual changes. Use sample data so
reviewers can reproduce the result. Update help or documentation when users need
to learn new behavior, and keep unrelated refactors separate.

Release tags, publishing, package uploads and repository visibility are handled
separately from a contribution. A local build or a passing test does not by itself
mean a release has been published.

## License and collaboration

`dutop` uses the [MIT license](LICENSE). Contributions are intended to be
distributed under the same license; keep applicable attribution and license
notices when incorporating third-party material.

Be respectful, explain tradeoffs, and make it easy for someone else to reproduce
what you found. Clear bug reports and thoughtful reviews count as contributions too.
