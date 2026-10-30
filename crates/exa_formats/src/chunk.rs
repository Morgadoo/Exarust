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

/// A cursor over a chunk payload that can step over a nested chunk: the
/// original's reader keeps a stack of chunk ends, opens a chunk by reading its
/// tag and size, reads the chunk's leading fields, and closes it to move the
/// cursor past its trailing sub-chunks.
///
/// Opening an innermost chunk remembers the enclosing chunk's end, so chunks
/// nest to any depth. `peek` is the original's "open, look at the tag, rewind
/// the cursor over the header" idiom: the end stays known, so closing it later
/// still skips the whole chunk.
#[derive(Debug, Clone)]
pub struct ChunkCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// The end offset of the innermost open chunk; zero when none is open.
    end: usize,
    /// The ends of the enclosing chunks, innermost last.
    ends: Vec<usize>,
}

impl<'a> ChunkCursor<'a> {
    /// A cursor at the start of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            end: 0,
            ends: Vec::new(),
        }
    }

    /// A cursor at `position`, which must lie within `bytes`.
    pub fn at(bytes: &'a [u8], position: usize) -> Self {
        Self {
            pos: position.min(bytes.len()),
            ..Self::new(bytes)
        }
    }

    /// The cursor's offset within its bytes.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Moves the cursor to `position`, clamped to the bytes.
    pub fn seek(&mut self, position: usize) {
        self.pos = position.min(self.bytes.len());
    }

    /// Moves the cursor back over the last header it read, so the next reads
    /// start at the chunk's tag again. The chunk's end stays known.
    pub fn rewind_header(&mut self) {
        self.pos = self.pos.saturating_sub(8);
    }

    /// Opens the chunk at the cursor: reads its tag and size, moves the cursor
    /// past them and remembers where the chunk ends. A truncated header or a
    /// size that runs past the bytes leaves the cursor unchanged.
    pub fn open(&mut self) -> Result<u32> {
        let tag = read_u32(self.bytes, self.pos)?;
        let size = read_u32(self.bytes, self.pos + 4)? as usize;
        let start = self.pos + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(Error::Truncated {
                offset: start,
                needed: size,
                len: self.bytes.len(),
            })?;
        self.ends.push(self.end);
        self.end = end;
        self.pos = start;
        Ok(tag)
    }

    /// Closes the innermost open chunk: moves the cursor to its end and
    /// restores the enclosing chunk's end. Closing with nothing open does
    /// nothing.
    pub fn close(&mut self) {
        if let Some(enclosing) = self.ends.pop() {
            self.pos = self.end;
            self.end = enclosing;
        }
    }

    /// Reads the next `u32` and moves past it.
    pub fn read_u32(&mut self) -> Result<u32> {
        let value = read_u32(self.bytes, self.pos)?;
        self.pos += 4;
        Ok(value)
    }

    /// Reads the next `n` bytes and moves past them.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.bytes.len(),
        })?;
        let slice = self.bytes.get(self.pos..end).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.bytes.len(),
        })?;
        self.pos = end;
        Ok(slice)
    }

    /// The bytes still to come in the innermost open chunk, or to the end of
    /// the bytes when none is open.
    pub fn remaining(&self) -> &'a [u8] {
        let end = if self.ends.is_empty() && self.end == 0 {
            self.bytes.len()
        } else {
            self.end
        };
        self.bytes.get(self.pos..end).unwrap_or(&[])
    }

    /// Reads the rest of the innermost open chunk as one slice and moves the
    /// cursor to its end, as the original copies a character's trailing saved
    /// chunks.
    pub fn take_remaining(&mut self) -> &'a [u8] {
        let rest = self.remaining();
        self.pos += rest.len();
        rest
    }
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

    /// The skipper's nest: an outer chunk whose payload is a `u32` then an
    /// inner chunk. Opening the outer, reading the word, opening and closing
    /// the inner, and closing the outer leaves the cursor after both; the
    /// enclosing end is restored so a third level would still close correctly.
    #[test]
    fn nests_chunk_ends() {
        let inner = chunk(0x1111, &7u32.to_le_bytes());
        let mut payload = 42u32.to_le_bytes().to_vec();
        payload.extend(&inner);
        let outer = chunk(0x2222, &payload);
        let mut bytes = outer.clone();
        bytes.extend(chunk(0x3333, &[]));

        let mut cursor = ChunkCursor::new(&bytes);
        assert_eq!(cursor.open().unwrap(), 0x2222);
        assert_eq!(cursor.read_u32().unwrap(), 42);
        assert_eq!(cursor.open().unwrap(), 0x1111);
        assert_eq!(cursor.read_u32().unwrap(), 7);
        cursor.close();
        // The inner chunk's end is the outer payload's end.
        assert_eq!(cursor.position(), outer.len());
        cursor.close();
        assert_eq!(cursor.position(), outer.len());
        // A sibling chunk follows.
        assert_eq!(cursor.open().unwrap(), 0x3333);
        cursor.close();
        assert_eq!(cursor.position(), bytes.len());
    }

    /// `peek` is open + rewind: the header can be read again, and closing
    /// still skips the whole chunk.
    #[test]
    fn peek_rewinds_the_header_and_still_skips() {
        let mut bytes = 99u32.to_le_bytes().to_vec();
        bytes.extend(5u32.to_le_bytes());
        bytes.extend([0xAAu8; 5]);
        let mut cursor = ChunkCursor::new(&bytes);
        assert_eq!(cursor.open().unwrap(), 99);
        cursor.rewind_header();
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.open().unwrap(), 99);
        assert_eq!(cursor.take_remaining(), [0xAA; 5]);
        cursor.close();
        cursor.close();
        assert_eq!(cursor.position(), bytes.len());
    }

    /// A chunk whose size runs past the buffer, or whose header is cut, is an
    /// error and leaves the cursor where it was.
    #[test]
    fn a_truncated_chunk_leaves_the_cursor() {
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend(9u32.to_le_bytes());
        bytes.extend([0u8; 3]);
        let mut cursor = ChunkCursor::new(&bytes);
        assert!(cursor.open().is_err());
        assert_eq!(cursor.position(), 0);

        let mut cut = 1u32.to_le_bytes().to_vec();
        cut.extend([0u8; 2]);
        let mut cursor = ChunkCursor::new(&cut);
        assert!(cursor.open().is_err());
        assert_eq!(cursor.position(), 0);
        // Closing with nothing open is harmless.
        cursor.close();
        assert_eq!(cursor.position(), 0);
    }
}
