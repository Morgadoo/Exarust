//! Exanima pack archives (`.rpk`), the project file (`Exanima.rfp`) and a
//! read-only virtual file system over an installation.
//!
//! The same table-of-contents layout is reused by several other extensions
//! inside `Resource.rpk` (`.rml` material libraries, `.flb` shader libraries,
//! `.fds` manual pages, `actors.rpk`), so [`Archive::parse`] works on any byte
//! slice, including an entry of another archive.

mod archive;
mod mem_archive;
mod project;
mod vfs;

pub use archive::{Archive, Entry, MappedArchive, RPK_MAGIC};
pub use mem_archive::{MemArchive, MemEntry};
pub use project::{PROJECT_MAGIC, Project};
pub use vfs::{GameData, PackKind, locate_install};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error on {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("bad magic {found:#010x}, expected {expected:#010x}")]
    BadMagic { found: u32, expected: u32 },
    #[error("truncated data: needed {needed} bytes at offset {offset}, have {len}")]
    Truncated {
        offset: usize,
        needed: usize,
        len: usize,
    },
    #[error("entry {name:?} points outside the archive ({offset}+{size} > {len})")]
    EntryOutOfBounds {
        name: String,
        offset: usize,
        size: usize,
        len: usize,
    },
    #[error("{0}")]
    Malformed(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn read_u32(b: &[u8], offset: usize) -> Result<u32> {
    b.get(offset..offset + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("slice of 4")))
        .ok_or(Error::Truncated {
            offset,
            needed: 4,
            len: b.len(),
        })
}
