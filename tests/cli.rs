mod support;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
};
use support::file;

fn run(root: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dutop"))
        .current_dir(root)
        .args(args)
        .env("TERM", "xterm-256color")
        .env("COLUMNS", "120")
        .env_remove("NO_COLOR")
        .output()
        .unwrap()
}

#[test]
fn default_directory_and_top_ten_work_without_ansi_when_piped() {
    let root = tempfile::tempdir().unwrap();
    for i in 0..14 {
        file(root.path(), &format!("file-{i:02}"), (i + 1) * 8192);
    }
    let result = run(root.path(), &[]);
    assert!(result.status.success());
    let out = String::from_utf8(result.stdout).unwrap();
    assert!(out.contains("FILES 10 / 14"));
    assert!(out.contains("file-13"));
    assert!(out.contains("file-04"));
    assert!(!out.contains("file-03"));
    assert!(out.contains("14 files/links"));
    assert!(!out.contains('\x1b'));
    assert!(result.stderr.is_empty());
}

#[test]
fn top_supports_space_equals_and_short_forms() {
    let root = tempfile::tempdir().unwrap();
    for i in 0..45 {
        file(root.path(), &format!("file-{i:02}"), (i + 1) * 8192);
    }
    for args in [vec!["--top", "4"], vec!["--top=4"], vec!["-n", "4"]] {
        let result = run(root.path(), &args);
        assert!(result.status.success());
        let out = String::from_utf8(result.stdout).unwrap();
        assert!(out.contains("file-41"));
        assert!(!out.contains("file-40"));
    }
    let result = run(root.path(), &["--top", "40"]);
    let out = String::from_utf8(result.stdout).unwrap();
    assert!(out.contains("file-05"));
    assert!(!out.contains("file-04"));
}

#[test]
fn invalid_counts_flags_and_missing_values_return_argument_errors() {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec!["--top", "0"],
        vec!["--top", "-1"],
        vec!["--top", "many"],
        vec!["--top"],
        vec!["--top", "999999999999999999999999"],
        vec!["--unknown"],
        vec!["--", "top", "4"],
    ] {
        let result = run(root.path(), &args);
        assert_eq!(result.status.code(), Some(2), "{args:?}");
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
}

#[test]
fn help_and_version_do_not_require_a_valid_scan_target() {
    let root = tempfile::tempdir().unwrap();
    let result = run(root.path(), &["--help", "/nonexistent-dutop-test"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("--top"));
    assert!(help.contains("--version"));
    assert!(help.contains("allocated"));
    assert!(help.contains("--top 4"));
    let result = run(root.path(), &["--version"]);
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        concat!("dutop ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn invalid_roots_fail_clearly_without_a_panic() {
    let root = tempfile::tempdir().unwrap();
    file(root.path(), "file", 1);
    for path in ["missing", "file"] {
        let result = run(root.path(), &[path]);
        assert_eq!(result.status.code(), Some(1));
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("dutop:"));
        assert!(!error.contains("panicked"));
    }
}

#[test]
fn partial_scan_prints_results_and_returns_failure() {
    let root = tempfile::tempdir().unwrap();
    file(root.path(), "locked/file", 8192);
    let path = root.path().join("locked");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let unreadable = fs::read_dir(&path).is_err();
    let result = run(root.path(), &[]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    if unreadable {
        assert_eq!(result.status.code(), Some(1));
        assert!(
            String::from_utf8(result.stdout)
                .unwrap()
                .contains("PARTIAL TOTAL")
        );
    } else {
        eprintln!("permission fixture skipped: runner can bypass filesystem permissions");
    }
}

#[test]
fn argument_errors_escape_user_control_sequences() {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec![".", "bad\rname"],
        vec!["--bad\u{202e}flag"],
        vec!["--bad\u{00ad}flag"],
    ] {
        let result = run(root.path(), &args);
        assert_eq!(result.status.code(), Some(2));
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(!error.contains(['\r', '\u{202e}', '\u{00ad}']));
    }
}

#[test]
fn top_accepts_99_and_rejects_100_before_scanning() {
    let root = tempfile::tempdir().unwrap();
    for i in 0..100 {
        file(root.path(), &format!("item-{i:03}"), (i + 1) * 4096);
    }
    let result = run(root.path(), &["--top", "99"]);
    assert!(result.status.success());
    let out = String::from_utf8(result.stdout).unwrap();
    assert!(out.contains("FILES 99 / 100"));
    assert!(out.contains("item-001"));
    assert!(!out.contains("item-000"));
    let result = run(root.path(), &["--top", "100", "missing"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(error.contains("99"));
}
