//! Descriptor-relative access prevents renamed ancestors from redirecting a scan.

use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags, Stat};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io,
    os::unix::ffi::OsStrExt,
    path::Path,
    sync::atomic::AtomicBool,
};

pub(super) struct Directory(pub File);

pub(super) struct Info {
    pub identity: (u64, u64),
    pub size: u64,
    pub links: u64,
    pub folder: bool,
    pub symlink: bool,
}

// The native stat field types differ between Darwin and Linux.
#[allow(clippy::unnecessary_cast)]
fn info(stat: Stat) -> io::Result<Info> {
    let blocks =
        u64::try_from(stat.st_blocks).map_err(|_| io::Error::other("Negative allocation"))?;
    let kind = FileType::from_raw_mode(stat.st_mode as _);
    Ok(Info {
        identity: (stat.st_dev as u64, stat.st_ino as u64),
        size: blocks
            .checked_mul(512)
            .ok_or_else(|| io::Error::other("Allocation exceeds supported range"))?,
        links: stat.st_nlink as u64,
        folder: kind == FileType::Directory,
        symlink: kind == FileType::Symlink,
    })
}

const FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

impl Directory {
    pub fn open_root(path: &Path) -> io::Result<File> {
        Ok(fs::open(path, FLAGS, Mode::empty())?.into())
    }

    pub fn info(&self) -> io::Result<Info> {
        info(fs::fstat(&self.0)?)
    }

    pub fn child_info(&self, name: &OsStr) -> io::Result<Info> {
        info(fs::statat(&self.0, name, AtFlags::SYMLINK_NOFOLLOW)?)
    }

    pub fn open_child(&self, name: &OsStr, expected: (u64, u64)) -> io::Result<Self> {
        let directory = Self(fs::openat(&self.0, name, FLAGS, Mode::empty())?.into());
        if directory.info()?.identity != expected {
            return Err(io::Error::other("Directory changed during scan"));
        }
        Ok(directory)
    }

    pub fn children(
        &self,
        cancelled: &AtomicBool,
        skipped: &mut usize,
    ) -> io::Result<std::vec::IntoIter<OsString>> {
        let entries = Dir::read_from(&self.0)?.filter_map(|entry| match entry {
            Ok(entry) => {
                let bytes = entry.file_name().to_bytes();
                if bytes == b"." || bytes == b".." {
                    None
                } else {
                    Some(Ok(OsStr::from_bytes(bytes).to_owned()))
                }
            }
            Err(error) => Some(Err(error.into())),
        });
        super::collect_names(entries, cancelled, skipped)
    }
}
