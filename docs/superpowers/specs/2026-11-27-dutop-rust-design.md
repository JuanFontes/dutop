# Dutop: native Rust CLI and incremental open-source release

Status: first-delivery scope approved by the user on 2026-11-27.

## Intent

Turn the existing personal `dutop` command into an independent open-source
terminal application. Users should see which immediate folders and files occupy
the most disk space, with the complete directory total below two ranked panels.
Installing a prebuilt release must not require Python or a Rust toolchain.

The user requested Rust, a `dev` branch, configurable top counts, help, improved
output, multiple operating systems, Docker/Compose builds, documentation and
licensing. The first usable delivery is the local Rust replacement. Distribution
and visual redesign follow in separate deliveries.

## Existing implementation

- Repository: `JuanFontes/dutop`, initially containing only `README.md`.
- Python source and six behavioral tests: `~/.local/share/dutop/`.
- Installed command: `~/.local/bin/dutop`, a symbolic link to the Python source.
- Shell integration: `alias dutop="$HOME/.local/bin/dutop"` in `~/.zshrc`.
- Scanner: macOS `/usr/bin/du --libxo json`, plus immediate-entry metadata.
- Output: two top-ten panels; panels stack below 90 terminal columns.
- Allocated space is reported, not apparent file length. Hidden paths are
  included, nested symbolic links are not followed, and incomplete scans are
  identified explicitly.

## Approaches

1. Package the existing Python implementation. Least migration work, but keeps
   Python and the current macOS-specific scanner. The user's distribution goal
   led them to request Rust instead.
2. Port the presentation to Rust while invoking native `du`. Removes Python but
   retains command-specific adapters and output parsing across operating systems.
3. Implement scanning and presentation in Rust. Recommended and consistent with
   the requested rewrite: a native CLI without Python or an external `du` command.
   This requires explicit tests for disk accounting and filesystem edge cases.

## Platform decision

First delivery: macOS and Linux, targeting ARM64 and x86_64. Windows is
deferred under the approved first-delivery scope.
Windows allocation measurement and packaging require separate implementation and
validation; do not silently report apparent sizes as allocated space there.

The supported OS-version baseline must be recorded when packaging is designed.
No claim of working on every Linux distribution is made. Linux container builds
produce Linux artifacts; macOS artifacts require an appropriate macOS build host.

## First delivery: local Rust replacement

Preserve the existing two-panel presentation as the visual reference. Improve
the visual design later, after the user chooses a direction through Product
Design. This delivery adds only the controls needed for a usable CLI:

```text
dutop [OPTIONS] [DIRECTORY]

DIRECTORY             Defaults to the current directory
-n, --top <N>          Number of entries per panel (1–99); defaults to 10
-h, --help            Show usage, size semantics and examples
-V, --version         Show the application version
```

`--top 4`, `--top 40` and `--top=4` are valid. Zero, negative values, non-numeric
values and values too large for the supported integer type produce a concise
argument error. `-- top 4` is not the flag: a bare `--` ends option parsing.
Follow-up approved on 2026-11-27: reject values above 99 with an argument error,
and defensively cap the renderer at 99. The top count applies independently to
each panel. Never allocate or loop solely
according to an untrusted huge requested count; limit rendering to actual entries,
with at most the existing ten-row minimum padding for visual continuity.

### Scan and accounting contract

- Traverse recursively to measure complete folder contents, but display only
  immediate children of the selected directory.
- Include hidden paths in rankings and the complete total.
- Measure allocated blocks using supported native filesystem metadata, including
  directory metadata in totals. Never read file contents to calculate sizes.
- Account for hard-linked regular files once in the complete total, keyed by
  filesystem device and inode. Immediate file rows show each entry's allocation,
  preserving the existing distinction between row sums and the total.
- Attribute shared hard-linked allocation deterministically to the first visited
  entry using a stable traversal order. Folder totals follow that assignment;
  document that independent scans of subfolders may differ.
- Do not follow child symbolic links. Show immediate links in the files panel
  with a link marker. An explicitly selected directory symlink may resolve to its
  target, matching the existing behavior.
- Include mounted filesystems by default, as the current command does. Prevent
  repeated traversal of an already visited directory identity where applicable.
- Rank largest first; ties use a deterministic filename order. Invalid UTF-8
  filenames must not crash scanning or inject terminal escape sequences.
- Keep scanning separate from rendering. Bound retained state to traversal state,
  immediate results and identities needed to prevent duplicate accounting; do not
  retain every nested file or its contents. Do not spawn a thread per path.
- Cancellation must stop the scan promptly and return exit status 130.
- Partial scan: print available results with an explicit partial total and return
  1. An unreadable or invalid root fails clearly. Argument errors return 2.
- No network requests, services, automatic privilege escalation or deletion.

APFS clones, snapshots and other shared storage mean allocated totals are not a
guarantee of the space that deleting an entry would free. Preserve this limitation
in help/documentation without adding noise to every row.

### Output contract

- Preserve two panels, descending sizes, complete total and entry counts.
- Stack panels in narrow terminals. Clip safely even at very small widths.
- Escape control characters in filenames and calculate terminal display widths.
- Respect `NO_COLOR`, `TERM=dumb` and non-terminal stdout.
- Send progress/cancellation information to stderr; keep stdout for the report.
- Keep the command a one-shot report that exits. No interactive application or
  continuous monitoring is included in this first delivery.

### Shell migration

Removing the alias alone does not remove the old executable from `PATH`. After
the Rust executable passes the focused checks:

1. Preserve the current Python source and existing command target for rollback.
2. Remove only the exact `dutop` alias definition from `.zshrc`, with a backup.
3. Replace the command at `~/.local/bin/dutop` with a self-contained release
   executable, installed atomically. Do not point to an ephemeral build directory.
4. Verify the installed command and a fresh interactive zsh resolve to Rust.
5. Tell the user to run `unalias dutop; rehash` in already-open shells, or open a
   new shell. Another process cannot remove an alias from an existing shell.

Changes under the user's home outside this repository require the execution
environment's approval. Prepare and verify the binary before requesting that
specific installation/configuration write. Do not remove unrelated aliases.

## Source ownership

Use one Cargo package, not a workspace or a framework:

```text
Cargo.toml / Cargo.lock
src/main.rs          Thin executable and exit handling
src/lib.rs           Public application modules
src/cli.rs           Arguments, help and version
src/scan/            Traversal, native metadata and accounting
src/report.rs        Report data and ranking
src/render.rs        Terminal width, escaping and panel presentation
tests/               Behavioral CLI and filesystem integration checks
docs/                Design, development and release instructions
packaging/           Debian and Homebrew definitions, in a later delivery
.github/workflows/   Focused CI and release builds, in later deliveries
```

Small helpers live beside their owning domain. Dependencies should be few and
purposeful; Rust dependencies compiled into the executable are acceptable.
Exact crates, versions and internal type names are implementation choices, to be
checked against current primary documentation before use.

## Small deliveries and verification

### D0 — This design and development branch

Create and select `dev`. Record scope, decisions, open questions and deliveries.
Review the document diff; no application tests for documentation alone.
No commits or remote publication are implied by creating the branch.

### D1 — Native CLI for local testing

Implement the first-delivery contract, port relevant behavioral coverage and
install the verified local executable in place of the Python entry point.

Focused checks: CLI defaults/invalid counts/help/version, rankings and complete
totals beyond N, hidden entries, sparse files, hard links, symlinks, special names,
partial reads, cancellation and narrow/no-color output. Compare controlled fixture
totals with native `du` where semantics align. Use deterministic unit coverage for
permission failure plus an actual non-root check when the environment permits.
Run formatting and Clippy for the new package. Verify an actual terminal rendering
separately from automated assertions. Linux is not marked verified until Linux
checks run; local macOS results alone are insufficient.

### D2 — Output design and optional power-user flags

Use Product Design to propose terminal-realistic visual directions grounded in
the current command, then implement the selected direction. Review real rendered
output at wide and narrow sizes; do not promise image effects a terminal cannot
render. Suggested later flags: `--json`, `--no-color`/`--color`, `--apparent-size`
and `--one-file-system`. These are proposals, not approved D1 scope.

Focused checks: only changed rendering/argument behavior and directly affected
scan semantics. JSON, if selected, needs a documented output contract.

### D3 — Reproducible Linux builds and CI

Add a multi-stage Dockerfile, Compose build service, `.dockerignore` and build
instructions. Build as an unprivileged user where practical; keep generated
artifacts out of source and avoid root-owned output on the host. A Docker socket
mount is unnecessary. Pin build inputs, keep `Cargo.lock`, and choose Linux libc
and OS baselines explicitly.

Add macOS/Linux checks for the supported targets. Docker validates the Linux
path; native macOS jobs validate macOS. Run checks for packaging/build changes
and the relevant CLI smoke test, not unrelated application suites.

### D4 — Open-source documentation and packages

Prepare an English README with real output, installation/usage, limitations,
contributing instructions and MIT license (proposed). Audit dependency licenses
and include required notices.

Prepare versioned archives and checksums; `.deb` packages for supported Linux
architectures; a Homebrew tap with a formula and prebuilt bottles. Clarify whether
the user also wants AppImage before adding that format. Test fresh install,
`--version`, a fixture scan and uninstall in the relevant environment. A source
build requires Rust; using a compatible prebuilt package does not.

Publication, tags, commits and pushes require explicit authorization at the
appropriate delivery. Preparing local artifacts does not count as publication.

## Decisions still needed from the user

1. Visual direction in D2 and AppImage requirement before D4; neither blocks the
   basic macOS/Linux scanner design.

## Implementation discretion

The implementer chooses maintained Rust crates, internal APIs, test fixture
organization, exact progress wording and build caching. These choices must not
alter the accounting, output, platform or installation contracts above. Performance
improvements require measurements rather than promises that Rust is always faster.
