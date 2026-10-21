//! `exa-tool saves`: read a save package and check it rebuilds.
//!
//! The file is only ever read. Nothing is written back, so it is safe to
//! point at the folder the game keeps its own saves in.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use exa_formats::save_package::{
    ARENA_SIGNATURE, Flavour, NAME_LEN, PLAYER_DATA, PackageImage, PlayerData, PlayerHeader,
};

/// A section name without its padding zeros.
fn text(name: &[u8; NAME_LEN]) -> String {
    let end = name.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
    String::from_utf8_lossy(&name[..end]).into_owned()
}

/// Lists a save's sections and player-data chunks, and reports whether the
/// package and the player data rebuild to the same bytes.
pub fn saves(path: &Path) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let package = PackageImage::read(&bytes)
        .map_err(|e| anyhow::anyhow!("{}: not a save package: {e:?}", path.display()))?;
    println!(
        "{}: {} bytes, magic {:#010x}, {} sections",
        path.display(),
        bytes.len(),
        package.magic,
        package.entries().len()
    );
    for entry in package.entries() {
        println!(
            "  {:#08x} {:>9} bytes  {}",
            entry.offset,
            entry.data.len(),
            text(&entry.name)
        );
    }
    let rebuilt = package.to_bytes() == bytes;
    println!("package rebuilds byte for byte: {rebuilt}");
    let mut sound = rebuilt;
    if let Some(section) = package.section(PLAYER_DATA) {
        if section.len() < 4 {
            bail!("the player data is shorter than its signature");
        }
        let signature = u32::from_le_bytes(section[0..4].try_into().unwrap());
        let flavour = if signature == ARENA_SIGNATURE {
            Flavour::Arena
        } else {
            Flavour::Story
        };
        let data = PlayerData::parse(section, flavour)
            .map_err(|e| anyhow::anyhow!("player data: {e:?}"))?;
        match &data.header {
            PlayerHeader::Story(h) => println!(
                "story save in {}: clock {} s, seed {:#x}",
                text(&h.locale),
                h.clock,
                h.seed
            ),
            PlayerHeader::Arena(h) => println!(
                "arena save in {}: day {}, {} copper, data version {:#x}{}",
                text(&h.locale),
                h.day,
                h.money,
                h.data_version,
                if data.arena_data_is_old() {
                    " (old)"
                } else {
                    ""
                }
            ),
        }
        println!(
            "  placement {}",
            data.character
                .as_ref()
                .map_or("none".to_string(), |c| format!("{} bytes", c.len()))
        );
        for chunk in &data.chunks {
            println!("  chunk {:#010x} {:>8} bytes", chunk.id, chunk.data.len());
        }
        let same = data.write() == section;
        println!("player data rewrites byte for byte: {same}");
        sound &= same;
    }
    if !sound {
        bail!("{} does not rebuild unchanged", path.display());
    }
    Ok(())
}
