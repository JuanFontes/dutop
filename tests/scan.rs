#![cfg(unix)]
mod support;

use dutop::{report::EntryKind, scan::scan};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    process::Command,
    sync::atomic::AtomicBool,
};
use support::file;

#[test]
fn first_level_rankings_include_hidden_and_total_matches_native_du() {
    let root = tempfile::tempdir().unwrap();
    file(root.path(), "large/deep/payload", 262_144);
    file(root.path(), "small/payload", 4096);
    file(root.path(), ".hidden/payload", 65536);
    file(root.path(), "visible", 8192);
    file(root.path(), ".secret", 32768);
    let r = scan(root.path(), &AtomicBool::new(false)).unwrap();
    assert!(!r.partial);
    assert_eq!(
        r.directories
            .iter()
            .map(|e| e.name.to_str().unwrap())
            .collect::<Vec<_>>(),
        ["large", ".hidden", "small"]
    );
    assert_eq!(
        r.files
            .iter()
            .map(|e| e.name.to_str().unwrap())
            .collect::<Vec<_>>(),
        [".secret", "visible"]
    );
    let output = Command::new("/usr/bin/du")
        .args(["-sk", "."])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let du_kib: u64 = String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(r.total.div_ceil(1024), du_kib);
}

#[test]
fn total_includes_entries_outside_top_ten() {
    let root = tempfile::tempdir().unwrap();
    for i in 0..14 {
        file(root.path(), &format!("file-{i:02}"), (i + 1) * 8192);
    }
    let r = scan(root.path(), &AtomicBool::new(false)).unwrap();
    assert_eq!(r.files.len(), 14);
    assert_eq!(r.files[0].name, "file-13");
    assert!(r.total > r.files[..10].iter().map(|e| e.size).sum::<u64>());
}

#[test]
fn symlinks_never_traverse_targets_but_explicit_root_can_be_a_link() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    file(outside.path(), "large", 1_048_576);
    symlink(outside.path(), root.path().join("outside")).unwrap();
    symlink(root.path(), root.path().join("cycle")).unwrap();
    let r = scan(root.path(), &AtomicBool::new(false)).unwrap();
    assert!(r.directories.is_empty());
    assert_eq!(r.files.len(), 2);
    assert!(r.files.iter().all(|e| e.kind == EntryKind::Link));
    assert!(r.total < 1_048_576);
    let linked = scan(&root.path().join("outside"), &AtomicBool::new(false)).unwrap();
    assert_eq!(linked.files[0].name, "large");
}

#[test]
fn hardlinks_are_counted_once_in_total_across_folders() {
    let root = tempfile::tempdir().unwrap();
    file(root.path(), "a/file", 65536);
    fs::create_dir(root.path().join("b")).unwrap();
    fs::hard_link(root.path().join("a/file"), root.path().join("b/shared")).unwrap();
    fs::hard_link(root.path().join("a/file"), root.path().join("z-file")).unwrap();
    let r = scan(root.path(), &AtomicBool::new(false)).unwrap();
    let expected = ["", "a", "b", "a/file"]
        .iter()
        .map(|p| fs::symlink_metadata(root.path().join(p)).unwrap().blocks() * 512)
        .sum::<u64>();
    assert_eq!(r.total, expected);
    let a = r.directories.iter().find(|e| e.name == "a").unwrap();
    let b = r.directories.iter().find(|e| e.name == "b").unwrap();
    assert!(a.size > b.size);
    assert_eq!(
        r.files[0].size,
        fs::metadata(root.path().join("z-file")).unwrap().blocks() * 512
    );
}

#[test]
fn sparse_files_use_allocation_not_apparent_length() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sparse");
    fs::File::create(&path).unwrap().set_len(1 << 30).unwrap();
    let r = scan(root.path(), &AtomicBool::new(false)).unwrap();
    assert_eq!(r.files[0].size, fs::metadata(path).unwrap().blocks() * 512);
    assert!(r.files[0].size < 1 << 30);
}

#[test]
fn unreadable_child_produces_a_partial_report() {
    let root = tempfile::tempdir().unwrap();
    file(root.path(), "locked/payload", 8192);
    let path = root.path().join("locked");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let unreadable = fs::read_dir(&path).is_err();
    let result = scan(root.path(), &AtomicBool::new(false));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    if unreadable {
        let r = result.unwrap();
        assert!(r.partial);
        assert!(r.skipped >= 1);
    } else {
        eprintln!("permission fixture skipped: runner can bypass filesystem permissions");
    }
}

#[test]
fn invalid_root_and_pre_cancelled_scan_return_errors() {
    let root = tempfile::tempdir().unwrap();
    assert!(scan(&root.path().join("missing"), &AtomicBool::new(false)).is_err());
    file(root.path(), "file", 1);
    assert!(scan(&root.path().join("file"), &AtomicBool::new(false)).is_err());
    assert_eq!(
        scan(root.path(), &AtomicBool::new(true))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Interrupted
    );
}
