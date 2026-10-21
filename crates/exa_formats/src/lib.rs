//! Engine-independent parsers for the formats stored in Exanima packs.
//!
//! Nothing here depends on Bevy: every parser takes a byte slice and returns
//! plain data, so the same code serves the runtime, `exa_tool` and tests.
//! Formats whose layout is only partly understood expose the raw fields they
//! read; see `docs/formats/` for what is known about each.

pub mod arena_save;
pub mod character_stats;
pub mod characters;
pub mod chunk;
pub mod containers;
pub mod data_stream;
pub mod encumbrance;
pub mod environment;
pub mod factory;
pub mod file_ref;
pub mod flb;
pub mod hw_config;
pub mod key_track;
pub mod kind;
pub mod level;
pub mod material;
pub mod morph;
pub mod objdb;
pub mod objstrings;
pub mod palette;
pub mod pixel;
pub mod power_tree;
pub mod procedural;
pub mod races;
pub mod rdb;
pub mod rfc;
pub mod rfc_write;
pub mod rfi;
pub mod rigid_body;
pub mod roles;
pub mod rsq;
pub mod save_chunk;
pub mod save_package;
pub mod settings_ini;
pub mod settings_store;
pub mod terrain;
pub mod trade_record;
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
