//! The engine's byte stream: a growable block with a logical size, a
//! capacity and a cursor, and the primitives every writer and reader of the
//! original's binary files uses (raw bytes, u8, u16, u32, f32, a text line,
//! length-prefixed strings, and Latin-1 text stored as UTF-8).
//!
//! The rules are the original's (`docs/formats/SETTINGS.md`, Byte stream):
//! - every fixed-size write sets the size to *cursor + bytes written*, so a
//!   write in the middle of the stream cuts it there;
//! - the block grows only when that size passes the capacity, to the size
//!   rounded up to a multiple of 512;
//! - the UTF-8 string write only ever raises the size, and grows the block to
//!   exactly what it needs.
//!
//! Reads that would run past the stream's size are errors here; the original
//! reads whatever memory follows.

use crate::{Error, Result};

/// The step the block's capacity grows in.
pub const GROW_STEP: usize = 512;

/// A byte stream over an owned block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataStream {
    /// The block; its length is the capacity.
    block: Vec<u8>,
    /// The logical size.
    size: usize,
    /// The cursor, an offset into the block.
    pos: usize,
}

impl DataStream {
    /// An empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// A stream over `bytes`, its cursor at the start.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let size = bytes.len();
        Self {
            block: bytes,
            size,
            pos: 0,
        }
    }

    /// The logical contents.
    pub fn bytes(&self) -> &[u8] {
        &self.block[..self.size.min(self.block.len())]
    }

    /// The logical contents, taken out of the stream.
    pub fn into_bytes(mut self) -> Vec<u8> {
        self.block.truncate(self.size);
        self.block
    }

    /// The logical size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The block's capacity.
    pub fn capacity(&self) -> usize {
        self.block.len()
    }

    /// The cursor.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Moves the cursor.
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Resizes the block to exactly `size` bytes, size and capacity alike,
    /// keeping the cursor's offset.
    pub fn set_size(&mut self, size: usize) {
        self.block.resize(size, 0);
        self.size = size;
    }

    /// Makes room for `n` bytes at the cursor: the size becomes cursor + `n`,
    /// and the block grows to that rounded up to 512 when it is too small.
    pub fn reserve_at_cursor(&mut self, n: usize) {
        self.size = self.pos + n;
        if self.block.len() < self.size {
            let capacity = self.size.div_ceil(GROW_STEP) * GROW_STEP;
            self.block.resize(capacity, 0);
        }
    }

    /// Writes `bytes` at the cursor.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.reserve_at_cursor(bytes.len());
        self.block[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
    }

    /// Writes a byte.
    pub fn write_u8(&mut self, v: u8) {
        self.write_bytes(&[v]);
    }

    /// Writes a little-endian u16.
    pub fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Writes a little-endian u32.
    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Writes a little-endian f32.
    pub fn write_f32(&mut self, v: f32) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Writes a line of text and the CR LF after it.
    pub fn write_line(&mut self, text: &[u8]) {
        self.write_bytes(text);
        self.write_u16(0x0a0d);
    }

    /// Writes a string as its u32 length and, when it has any, its bytes.
    pub fn write_string(&mut self, text: &[u8]) {
        self.write_u32(text.len() as u32);
        if !text.is_empty() {
            self.write_bytes(text);
        }
    }

    /// Writes Latin-1 text as its UTF-8 length (u32) and its UTF-8 bytes.
    /// The size only ever rises; when the block is too small it is resized
    /// to exactly the end of the string.
    pub fn write_utf8(&mut self, latin1: &[u8]) {
        let len = utf8_len(latin1);
        let end = self.pos + len + 4;
        if self.size < end {
            if self.block.len() < end {
                self.set_size(end);
            } else {
                self.size = end;
            }
        }
        self.block[self.pos..self.pos + 4].copy_from_slice(&(len as u32).to_le_bytes());
        self.pos += 4;
        let encoded = latin1_to_utf8(latin1);
        self.block[self.pos..self.pos + len].copy_from_slice(&encoded);
        self.pos += len;
    }

    /// Reads `n` raw bytes at the cursor.
    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.size);
        let Some(end) = end else {
            return Err(Error::Truncated {
                offset: self.pos,
                needed: n,
                len: self.size,
            });
        };
        let start = self.pos;
        self.pos = end;
        Ok(&self.block[start..end])
    }

    /// Reads a little-endian u32.
    pub fn read_u32(&mut self) -> Result<u32> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Reads a length-prefixed string into `text`. A zero length leaves
    /// `text` as it was, as the original does.
    pub fn read_string_into(&mut self, text: &mut Vec<u8>) -> Result<()> {
        let n = self.read_u32()? as usize;
        if n != 0 {
            let bytes = self.read_bytes(n)?.to_vec();
            *text = bytes;
        }
        Ok(())
    }

    /// Reads a length-prefixed string.
    pub fn read_string(&mut self) -> Result<Vec<u8>> {
        let mut text = Vec::new();
        self.read_string_into(&mut text)?;
        Ok(text)
    }

    /// Reads a length-prefixed UTF-8 string back into Latin-1 text. A zero
    /// length gives empty text.
    pub fn read_utf8(&mut self) -> Result<Vec<u8>> {
        let n = self.read_u32()? as usize;
        if n == 0 {
            return Ok(Vec::new());
        }
        let bytes = self.read_bytes(n)?;
        Ok(utf8_to_latin1(bytes))
    }
}

/// The UTF-8 length of Latin-1 text: one byte below 0x80, two above.
pub fn utf8_len(latin1: &[u8]) -> usize {
    latin1.iter().map(|&b| if b < 0x80 { 1 } else { 2 }).sum()
}

/// Latin-1 text as UTF-8: bytes below 0x80 as they are, 0x80–0xBF as
/// `C2 b`, 0xC0–0xFF as `C3 (b − 0x40)`.
pub fn latin1_to_utf8(latin1: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(utf8_len(latin1));
    for &b in latin1 {
        if b < 0x80 {
            out.push(b);
        } else if b < 0xc0 {
            out.extend_from_slice(&[0xc2, b]);
        } else {
            out.extend_from_slice(&[0xc3, b - 0x40]);
        }
    }
    out
}

/// The Latin-1 length of UTF-8 text by the same walk as
/// [`utf8_to_latin1`]: one for a byte below 0x80 and for a `C2`/`C3`
/// sequence, nothing for any other sequence.
pub fn latin1_len(utf8: &[u8]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < utf8.len() {
        let b = utf8[i];
        if b < 0x80 {
            count += 1;
            i += 1;
        } else if b < 0xe0 {
            if b == 0xc2 || b == 0xc3 {
                count += 1;
            }
            i += 2;
        } else if b < 0xf0 {
            i += 3;
        } else {
            i += 4;
        }
    }
    count
}

/// UTF-8 back to Latin-1: a byte below 0x80 is kept; a two-byte sequence
/// gives a character only when it starts with `C2` (the next byte) or `C3`
/// (the next byte + 0x40); three- and four-byte sequences are dropped. A
/// sequence cut off by the end reads its missing byte as 0 (the original
/// reads past the string).
pub fn utf8_to_latin1(utf8: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < utf8.len() {
        let b = utf8[i];
        if b < 0x80 {
            out.push(b);
            i += 1;
        } else if b < 0xe0 {
            let next = utf8.get(i + 1).copied().unwrap_or(0);
            if b == 0xc3 {
                out.push(next.wrapping_add(0x40));
            } else if b == 0xc2 {
                out.push(next);
            }
            i += 2;
        } else if b < 0xf0 {
            i += 3;
        } else {
            i += 4;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_cut_the_size_at_the_cursor_and_grow_in_512_steps() {
        let mut s = DataStream::new();
        s.write_u32(0x6440_a401);
        assert_eq!((s.size(), s.capacity()), (4, 512));
        s.write_bytes(&[7; 600]);
        assert_eq!((s.size(), s.capacity()), (604, 1024));
        // A write in the middle cuts the stream after it.
        s.seek(2);
        s.write_u16(0xbeef);
        assert_eq!(s.size(), 4);
        assert_eq!(s.bytes(), &[0x01, 0xa4, 0xef, 0xbe]);
        assert_eq!(s.capacity(), 1024);
        s.write_u8(9);
        s.write_f32(1.0);
        assert_eq!(&s.bytes()[4..], &[9, 0, 0, 0x80, 0x3f]);
    }

    #[test]
    fn a_line_ends_in_cr_lf_and_a_string_has_its_length() {
        let mut s = DataStream::new();
        s.write_line(b"GUISize = 1");
        assert_eq!(s.bytes(), b"GUISize = 1\r\n");
        let mut s = DataStream::new();
        s.write_string(b"");
        s.write_string(b"ab");
        assert_eq!(s.bytes(), &[0, 0, 0, 0, 2, 0, 0, 0, b'a', b'b']);
        s.seek(0);
        let mut text = b"kept".to_vec();
        s.read_string_into(&mut text).unwrap();
        assert_eq!(text, b"kept");
        assert_eq!(s.read_string().unwrap(), b"ab");
        assert!(s.read_u32().is_err());
    }

    #[test]
    fn latin1_round_trips_through_utf8() {
        // é (0xE9) is C3 A9; the section sign (0xA7) is C2 A7.
        let text = [b'C', 0xe9, 0xa7];
        assert_eq!(utf8_len(&text), 5);
        assert_eq!(latin1_to_utf8(&text), [b'C', 0xc3, 0xa9, 0xc2, 0xa7]);
        let mut s = DataStream::new();
        s.write_utf8(&text);
        // Exactly the string's end: 4 + 5 bytes, no 512 rounding.
        assert_eq!((s.size(), s.capacity()), (9, 9));
        s.seek(0);
        assert_eq!(s.read_utf8().unwrap(), text);
        // Three- and four-byte sequences and other lead bytes are dropped.
        assert_eq!(
            utf8_to_latin1(&[0xe2, 0x82, 0xac, b'x', 0xc4, 0x80, 0xf0, 1, 2, 3]),
            b"x"
        );
        assert_eq!(
            latin1_len(&[0xe2, 0x82, 0xac, b'x', 0xc4, 0x80, 0xf0, 1, 2, 3]),
            1
        );
        assert_eq!(latin1_len(&[b'C', 0xc3, 0xa9, 0xc2, 0xa7]), 3);
        // A lead cut off by the end still counts, as the conversion writes it.
        assert_eq!(latin1_len(&[0xc3]), 1);
        assert_eq!(utf8_to_latin1(&[0xc3]), [0x40]);
    }

    #[test]
    fn the_utf8_write_only_raises_the_size() {
        let mut s = DataStream::from_bytes(vec![0; 20]);
        s.write_utf8(b"ab");
        assert_eq!((s.size(), s.capacity()), (20, 20));
        assert_eq!(&s.bytes()[..6], &[2, 0, 0, 0, b'a', b'b']);
    }
}
