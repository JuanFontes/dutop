# Dutop first packaged release

## Goal and constraints
Make dutop installable through JuanFontes/homebrew-tap; verify the existing
release checklist. Preserve the approved CLI and presentation. No RTK wrappers.
Python is build/test tooling only, never an installed runtime dependency.

## Decisions
- Release candidate: 0.1.0-beta.1, MIT.
- Native archives for macOS ARM64/x86_64 (deployment target 13; CI on 15),
  Linux ARM64/x86_64 with static musl linking. Debian packages for both Linux CPUs.
- Follow the existing tap's prebuilt archive model, not Homebrew bottle DSL.
- Rust 1.100.0 release compiler; actually check the declared Rust 1.85 minimum.
- Docker is an optional Linux build/export tool, not a runtime service.
- Prepare and test privately before requesting the final visibility change.

## Task 1: package artifacts and validate compiler compatibility
Write packaging tests for archive layout, executable mode, complete licensing,
checksums and rejection of mismatched versions/unsupported targets. Observe the
new tests fail, then implement scripts/package.py. Build the current CLI with
Rust 1.85 and 1.100; fix proven compatibility failures only. Package .deb via
native dpkg-deb, never handwritten archive internals. Native package smoke tests
must inspect version, fixture output, invalid flags and install/uninstall.

## Task 2: automated builds and checks
Add CI for all four native platforms and MSRV. Check formatting and Clippy once;
test release binaries through existing PTY checks. Release workflow validates tag
against Cargo version, packages all four architectures, gathers checksums and
creates a draft prerelease only after checks pass. Pin third-party actions.
Add an optional multi-stage Dockerfile and reproducible Linux build instructions.

## Task 3: Homebrew integration and documentation
Generate a formula from actual release checksums, preserving existing Curlimp.
Test formula install, brew test and uninstall on supported platforms. Prepare
README installation commands and maintainer release instructions. Review dependency
notices and tracked files. Update the tap through scoped approved filesystem writes.

## Task 4: review and publish
One fresh independent review of packaging, CI and distribution boundaries; fix
important defects with focused regressions. Commit/push the prepared development
branch for native CI. When artifacts and changes are reviewable, confirm the private
repository can become public. Publish the release and tap, then install from the
actual public tap and verify the installed executable. Report any unverified target
or blocked publication explicitly; local artifacts are not a public installation.

## Review focus
Cross-platform packaging, filename/version safety, formula checksums and platform
selection, license attribution, release/tag consistency, workflow permissions,
non-root filesystem tests, and manual binaries shadowing Homebrew on PATH.
