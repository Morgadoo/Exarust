use crate::read_u32;

/// Format identified from an entry's leading bytes. Entries in the main packs
/// carry no extension, so the magic is the only reliable discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileKind {
    /// `.rpk` / `.rml` / `.flb` / `.fds`: table-of-contents archive.
    Pack,
    /// `.rfc`: chunk tree holding meshes, materials, skins and levels.
    ChunkTree,
    /// Older chunk-tree variant seen on a few character meshes.
    ChunkTreeAlt,
    /// `.rfi`: image (block-compressed or packed).
    Image,
    /// `.rft`: terrain.
    Terrain,
    /// `.ftb`: terrain material table.
    TerrainTable,
    /// `.rdb`: indexed record database.
    Database,
    /// `.rsq`: skeleton and motion sequences.
    Sequences,
    /// `.pwr`: power (thaumaturgy) definition.
    Power,
    /// Weapon assembly recipe from `Factories.rpk`.
    Factory,
    /// Actor definition from `actors.rpk`.
    Actor,
    /// RIFF/WAVE audio.
    Wave,
    /// DirectDraw Surface.
    Dds,
    Unknown(u32),
}

impl FileKind {
    pub fn detect(b: &[u8]) -> FileKind {
        let Ok(m) = read_u32(b, 0) else {
            return FileKind::Unknown(0);
        };
        match m {
            0xAFBF_0C01 => FileKind::Pack,
            0x3D23_AFCF => FileKind::ChunkTree,
            0x3D21_AFCF => FileKind::ChunkTreeAlt,
            0x1D2D_3DC6 => FileKind::Image,
            0x3EEF_AD01 => FileKind::Terrain,
            0x3EEF_BD02 | 0x3EEF_BD03 => FileKind::TerrainTable,
            0xAFCE_01CE => FileKind::Power,
            0x4646_4952 => FileKind::Wave,
            0x2053_4444 => FileKind::Dds,
            0x3D00_0000 => FileKind::Sequences,
            _ if m & 0xFFFF_0000 == 0xDBCB_0000
                || m & 0xFFFF_0000 == 0xDBAE_0000
                || m & 0xFFFF_0000 == 0xDBAC_0000 =>
            {
                FileKind::Database
            }
            _ if m & 0xFFFF_00FF == 0xAFCE_000F => FileKind::Factory,
            _ if m & 0xFFFF_FF00 == 0x00CD_AC00 => FileKind::Actor,
            _ => FileKind::Unknown(m),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            FileKind::Pack => "rpk",
            FileKind::ChunkTree | FileKind::ChunkTreeAlt => "rfc",
            FileKind::Image => "rfi",
            FileKind::Terrain => "rft",
            FileKind::TerrainTable => "ftb",
            FileKind::Database => "rdb",
            FileKind::Sequences => "rsq",
            FileKind::Power => "pwr",
            FileKind::Factory => "fac",
            FileKind::Actor => "act",
            FileKind::Wave => "wav",
            FileKind::Dds => "dds",
            FileKind::Unknown(_) => "bin",
        }
    }
}
