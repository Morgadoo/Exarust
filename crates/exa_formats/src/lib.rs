//! Engine-independent parsers for the formats stored in Exanima packs.
//!
//! Nothing here depends on Bevy: every parser takes a byte slice and returns
//! plain data, so the same code serves the runtime, `exa_tool` and tests.
//! Formats whose layout is only partly understood expose the raw fields they
//! read; see `docs/formats/` for what is known about each.

pub mod characters;
pub mod chunk;
pub mod environment;
pub mod factory;
pub mod kind;
pub mod level;
pub mod material;
pub mod objdb;
pub mod races;
pub mod rdb;
pub mod rfc;
pub mod rfi;
pub mod rsq;
pub mod terrain;
pub mod weapon_record;
pub mod weapon_stats;

pub use kind::FileKind;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad magic {found:#010x}, expected {expected:#010x}")]
    BadMagic { found: u32, expected: u32 },
    #[error("truncated: needed {needed} bytes at offset {offset}, have {len}")]
    Truncated {
        offset: usize,
        needed: usize,
        len: usize,
    },
    #[error("unsupported: {0}")]
    Unsupported(String),
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
