//! The tag/size chunk container used by `.rfc` files.
//!
//! A file is a 4-byte magic followed by chunks. Each chunk is `tag: u32`,
//! `size: u32`, then `size` payload bytes. List chunks (`0x3D00_0000` meshes,
//! `0x0000_BA00` materials) start their payload with a `u32` count followed by
//! child chunks; other payloads mix fixed fields and nested chunks, so this
//! module only offers the framing and leaves interpretation to callers.

use crate::{Error, Result, read_u32};

pub const RFC_MAGIC: u32 = 0x3D23_AFCF;

/// Well-known tags observed across the shipped data.
pub mod tag {
    pub const MATERIAL_LIST: u32 = 0x0000_BA00;
    pub const MATERIAL: u32 = 0x0000_BA01;
    pub const MESH_LIST: u32 = 0x3D00_0000;
    pub const MESH: u32 = 0x0000_3D03;
    pub const MESH_V1: u32 = 0x0000_3D01;
    pub const MESH_V6: u32 = 0x0000_3D06;
    /// A voxel light node: light added straight into the GI volume, not a
    /// mesh (`rfc::NODE_VOXEL_LIGHT`).
    pub const VOXEL_LIGHT: u32 = 0x0000_3D0C;
    /// Tile-grid level header (wall/floor set names and the cell grid).
    pub const LEVEL_GRID: u32 = 0x0000_3DED;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub tag: u32,
    /// Offset of the payload within the slice that was iterated.
    pub offset: usize,
    pub data: &'a [u8],
}

/// Iterates sibling chunks over `b[start..]`.
pub struct Chunks<'a> {
    b: &'a [u8],
    pos: usize,
    failed: bool,
}

impl<'a> Chunks<'a> {
    pub fn new(b: &'a [u8], start: usize) -> Self {
        Self {
            b,
            pos: start,
            failed: false,
        }
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Result<Chunk<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.pos >= self.b.len() {
            return None;
        }
        let item = (|| {
            let tag = read_u32(self.b, self.pos)?;
            let size = read_u32(self.b, self.pos + 4)? as usize;
            let offset = self.pos + 8;
            let data = self.b.get(offset..offset + size).ok_or(Error::Truncated {
                offset,
                needed: size,
                len: self.b.len(),
            })?;
            self.pos = offset + size;
            Ok(Chunk { tag, offset, data })
        })();
        self.failed = item.is_err();
        Some(item)
    }
}

/// Top-level chunks of an `.rfc` file.
pub fn rfc_chunks(b: &[u8]) -> Result<Chunks<'_>> {
    let magic = read_u32(b, 0)?;
    if magic != RFC_MAGIC && magic != 0x3D21_AFCF {
        return Err(Error::BadMagic {
            found: magic,
            expected: RFC_MAGIC,
        });
    }
    Ok(Chunks::new(b, 4))
}

/// Children of a list chunk: a `u32` count followed by exactly that many
/// chunks filling the payload. Returns `None` when the payload is not shaped
/// like a list.
pub fn list_children<'a>(chunk: &Chunk<'a>) -> Option<(u32, Vec<Chunk<'a>>)> {
    let count = read_u32(chunk.data, 0).ok()?;
    let children: Result<Vec<_>> = Chunks::new(chunk.data, 4).collect();
    let children = children.ok()?;
    (children.len() == count as usize).then_some((count, children))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn walks_lists() {
        let mut list = 2u32.to_le_bytes().to_vec();
        list.extend(chunk(tag::MESH, b"a"));
        list.extend(chunk(tag::MESH, b"bc"));
        let mut file = RFC_MAGIC.to_le_bytes().to_vec();
        file.extend(chunk(tag::MESH_LIST, &list));
        let top: Vec<_> = rfc_chunks(&file).unwrap().map(Result::unwrap).collect();
        assert_eq!(top.len(), 1);
        let (n, kids) = list_children(&top[0]).unwrap();
        assert_eq!(n, 2);
        assert_eq!(kids[1].data, b"bc");
    }

    #[test]
    fn stops_on_truncation() {
        let mut file = RFC_MAGIC.to_le_bytes().to_vec();
        file.extend(chunk(tag::MESH, b"abcd"));
        file.truncate(file.len() - 1);
        let items: Vec<_> = rfc_chunks(&file).unwrap().collect();
        assert_eq!(items.len(), 1);
        assert!(items[0].is_err());
    }
}
