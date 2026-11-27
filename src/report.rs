use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Folder,
    File,
    Link,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: OsString,
    pub size: u64,
    pub kind: EntryKind,
}

#[derive(Debug)]
pub struct Report {
    pub path: PathBuf,
    pub directories: Vec<Entry>,
    pub files: Vec<Entry>,
    pub total: u64,
    pub partial: bool,
    pub skipped: usize,
}
