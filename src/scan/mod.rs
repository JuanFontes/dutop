//! Native allocated-space accounting with descriptor-relative traversal.
mod unix;

use crate::report::{Entry, EntryKind, Report};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs, io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use unix::Directory;

struct Frame {
    directory: Directory,
    name: OsString,
    children: std::vec::IntoIter<OsString>,
    total: u64,
}

fn check_cancelled(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Scan cancelled.",
        ))
    } else {
        Ok(())
    }
}

fn collect_names(
    entries: impl Iterator<Item = io::Result<OsString>>,
    cancelled: &AtomicBool,
    skipped: &mut usize,
) -> io::Result<std::vec::IntoIter<OsString>> {
    let mut names = Vec::new();
    for entry in entries {
        check_cancelled(cancelled)?;
        match entry {
            Ok(name) => names.push(name),
            Err(_) => *skipped += 1,
        }
    }
    names.sort_unstable();
    Ok(names.into_iter())
}

fn add(total: &mut u64, bytes: u64) -> io::Result<()> {
    *total = total
        .checked_add(bytes)
        .ok_or_else(|| io::Error::other("Total exceeds supported range"))?;
    Ok(())
}

/// Retain immediate results and traversal state, never nested file contents.
/// Cancellation is checked between filesystem operations; the OS can still block I/O.
pub fn scan(path: &Path, cancelled: &AtomicBool) -> io::Result<Report> {
    check_cancelled(cancelled)?;
    let path = fs::canonicalize(path)?;
    scan_opened(&path, Directory::open_root(&path)?, cancelled)
}

fn scan_opened(path: &Path, root: fs::File, cancelled: &AtomicBool) -> io::Result<Report> {
    check_cancelled(cancelled)?;
    let directory = Directory(root);
    let metadata = directory.info()?;
    if !metadata.folder {
        return Err(io::ErrorKind::NotADirectory.into());
    }
    let mut report = Report {
        path: path.to_path_buf(),
        directories: Vec::new(),
        files: Vec::new(),
        total: 0,
        partial: false,
        skipped: 0,
    };
    let mut directories = HashSet::from([metadata.identity]);
    let mut links = HashSet::new();
    let mut stack = vec![Frame {
        children: directory.children(cancelled, &mut report.skipped)?,
        directory,
        name: OsString::new(),
        total: metadata.size,
    }];
    while let Some(frame) = stack.last_mut() {
        check_cancelled(cancelled)?;
        let Some(name) = frame.children.next() else {
            let finished = stack.pop().expect("frame exists");
            if let Some(parent) = stack.last_mut() {
                add(&mut parent.total, finished.total)?;
                if stack.len() == 1 {
                    report.directories.push(Entry {
                        name: finished.name,
                        size: finished.total,
                        kind: EntryKind::Folder,
                    });
                }
            } else {
                report.total = finished.total;
            }
            continue;
        };
        let metadata = match frame.directory.child_info(&name) {
            Ok(metadata) => metadata,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        if metadata.folder {
            let first_visit = directories.insert(metadata.identity);
            let opened = if first_visit {
                frame
                    .directory
                    .open_child(&name, metadata.identity)
                    .and_then(|directory| {
                        let children = directory.children(cancelled, &mut report.skipped)?;
                        Ok((directory, children))
                    })
            } else {
                // A repeated directory is already accounted for; don't descend again.
                if stack.len() == 1 {
                    report.directories.push(Entry {
                        name,
                        size: 0,
                        kind: EntryKind::Folder,
                    });
                }
                continue;
            };
            match opened {
                Ok((directory, children)) => stack.push(Frame {
                    directory,
                    name,
                    children,
                    total: metadata.size,
                }),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
                Err(_) => {
                    report.skipped += 1;
                    add(&mut frame.total, metadata.size)?;
                    if stack.len() == 1 {
                        report.directories.push(Entry {
                            name,
                            size: metadata.size,
                            kind: EntryKind::Folder,
                        });
                    }
                }
            }
        } else {
            if metadata.links <= 1 || links.insert(metadata.identity) {
                add(&mut frame.total, metadata.size)?;
            }
            if stack.len() == 1 {
                report.files.push(Entry {
                    name,
                    size: metadata.size,
                    kind: if metadata.symlink {
                        EntryKind::Link
                    } else {
                        EntryKind::File
                    },
                });
            }
        }
    }
    check_cancelled(cancelled)?;
    let rank = |a: &Entry, b: &Entry| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name));
    report.directories.sort_unstable_by(rank);
    report.files.sort_unstable_by(rank);
    report.partial = report.skipped != 0;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_iterator_error_keeps_readable_siblings_and_marks_partial() {
        let entries = [
            Ok(OsString::from("z")),
            Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            Ok(OsString::from("a")),
        ];
        let mut skipped = 0;
        let names =
            collect_names(entries.into_iter(), &AtomicBool::new(false), &mut skipped).unwrap();
        assert_eq!(names.collect::<Vec<_>>(), ["a", "z"]);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn opened_directory_stays_anchored_when_its_path_becomes_a_symlink() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("original");
        let outside = temp.path().join("outside");
        fs::create_dir(&path).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(path.join("inside"), b"inside").unwrap();
        fs::write(outside.join("foreign"), b"outside").unwrap();
        let opened = fs::File::open(&path).unwrap();
        fs::rename(&path, temp.path().join("moved")).unwrap();
        symlink(outside, &path).unwrap();
        let report = scan_opened(&path, opened, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            report
                .files
                .iter()
                .map(|e| e.name.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["inside"]
        );
    }
}
