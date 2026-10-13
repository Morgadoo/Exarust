//! `.rfi` images.
//!
//! 32-byte header, all `u32` little endian:
//!
//! | offset | field |
//! |---|---|
//! | 0 | magic `C6 3D 2D 1D` |
//! | 4 | width |
//! | 8 | height |
//! | 12 | depth (1; 0 on a handful of entries) |
//! | 16 | format word, see [`PixelFormat::from_word`] |
//! | 20 | flags (sampler/usage bits, not yet decoded) |
//! | 24 | storage: `0x1000_0000` raw, `0x5000_0000` packed |
//! | 28 | size of the unpacked pixel data (all mip levels) |
//!
//! Raw images store the mip chain, largest level first, right after the
//! header. Packed images run-length encode that same chain (see [`unpack`]);
//! only uncompressed pixel formats are packed.

use std::borrow::Cow;

use crate::{Error, Result, read_u32};

pub const RFI_MAGIC: u32 = 0x1D2D_3DC6;
const HEADER_LEN: usize = 32;
const STORAGE_PACKED: u32 = 0x5000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// RGB, 8 bytes per 4x4 block.
    Bc1,
    /// RGBA, 16 bytes per block.
    Bc3,
    /// One channel (luminance), 8 bytes per block.
    Bc4,
    /// Two channels (normal map X/Y), 16 bytes per block.
    Bc5,
    /// Stored as B, G, R.
    Rgb8,
    /// Stored as B, G, R, A.
    Rgba8,
    L8,
    La8,
    A8,
    Unknown(u32),
}

impl PixelFormat {
    /// The format word reads as bit fields: the top byte has bit 7 set for
    /// block compression; bit 6 of the third byte selects 16-byte blocks; the
    /// second byte is a channel mask in which `0x20` means alpha, `0x42` one
    /// luminance channel, `0x84` two channels and `0xC6` RGB. The mapping was
    /// derived from data sizes across every shipped image.
    pub fn from_word(word: u32) -> PixelFormat {
        let channels = (word >> 8) & 0xFF;
        let compressed = word & 0x8000_0000 != 0;
        let wide_blocks = word & 0x0040_0000 != 0;
        match (compressed, wide_blocks, channels) {
            (true, false, 0xC6) => PixelFormat::Bc1,
            (true, false, 0x42) => PixelFormat::Bc4,
            (true, true, 0x84) => PixelFormat::Bc5,
            (true, true, 0xE6 | 0xA4) => PixelFormat::Bc3,
            (false, _, 0xC6) => PixelFormat::Rgb8,
            (false, _, 0xE6) => PixelFormat::Rgba8,
            (false, _, 0x42) => PixelFormat::L8,
            (false, _, 0x62) => PixelFormat::La8,
            (false, _, 0x20) => PixelFormat::A8,
            _ => PixelFormat::Unknown(word),
        }
    }

    /// Bytes per pixel of uncompressed formats.
    pub fn bytes_per_pixel(self) -> Option<usize> {
        match self {
            PixelFormat::Rgb8 => Some(3),
            PixelFormat::Rgba8 => Some(4),
            PixelFormat::La8 => Some(2),
            PixelFormat::L8 | PixelFormat::A8 => Some(1),
            _ => None,
        }
    }

    /// Bytes for one mip level of `w` x `h`.
    pub fn level_size(self, w: u32, h: u32) -> Option<usize> {
        let blocks = || (w.div_ceil(4) * h.div_ceil(4)) as usize;
        let px = (w * h) as usize;
        Some(match self {
            PixelFormat::Bc1 | PixelFormat::Bc4 => blocks() * 8,
            PixelFormat::Bc3 | PixelFormat::Bc5 => blocks() * 16,
            PixelFormat::Rgb8 => px * 3,
            PixelFormat::Rgba8 => px * 4,
            PixelFormat::La8 => px * 2,
            PixelFormat::L8 | PixelFormat::A8 => px,
            PixelFormat::Unknown(_) => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RfiHeader {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub format_word: u32,
    pub flags: u32,
    pub storage: u32,
    pub data_size: u32,
}

#[derive(Debug, Clone)]
pub struct Rfi<'a> {
    pub header: RfiHeader,
    pub format: PixelFormat,
    /// Number of mip levels present (1 when only the base level is stored).
    pub mip_count: u32,
    /// Raw pixel data, or the packed stream when [`Rfi::is_packed`].
    pub data: &'a [u8],
}

/// Decodes the run-length stream of a packed image into `out_len` bytes.
///
/// A control byte `c` below the threshold (64 for one byte per pixel, 63
/// otherwise) is followed by `c + 1` literal pixels; any other value is
/// followed by one pixel repeated `c - 61` times.
pub fn unpack(stream: &[u8], bytes_per_pixel: usize, out_len: usize) -> Result<Vec<u8>> {
    let threshold = if bytes_per_pixel == 1 { 64 } else { 63 };
    let mut out = Vec::with_capacity(out_len);
    let mut pos = 0usize;
    let truncated = |pos: usize, needed: usize| Error::Truncated {
        offset: HEADER_LEN + pos,
        needed,
        len: HEADER_LEN + stream.len(),
    };
    while out.len() < out_len {
        let c = *stream.get(pos).ok_or_else(|| truncated(pos, 1))? as usize;
        pos += 1;
        if c < threshold {
            let n = (c + 1) * bytes_per_pixel;
            let lit = stream.get(pos..pos + n).ok_or_else(|| truncated(pos, n))?;
            out.extend_from_slice(lit);
            pos += n;
        } else {
            let px = stream
                .get(pos..pos + bytes_per_pixel)
                .ok_or_else(|| truncated(pos, bytes_per_pixel))?;
            for _ in 0..c - 61 {
                out.extend_from_slice(px);
            }
            pos += bytes_per_pixel;
        }
    }
    if out.len() != out_len {
        return Err(Error::Malformed(format!(
            "rle overran: {} bytes for {out_len}",
            out.len()
        )));
    }
    Ok(out)
}

impl<'a> Rfi<'a> {
    pub fn parse(b: &'a [u8]) -> Result<Self> {
        let magic = read_u32(b, 0)?;
        if magic != RFI_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: RFI_MAGIC,
            });
        }
        let header = RfiHeader {
            width: read_u32(b, 4)?,
            height: read_u32(b, 8)?,
            depth: read_u32(b, 12)?,
            format_word: read_u32(b, 16)?,
            flags: read_u32(b, 20)?,
            storage: read_u32(b, 24)?,
            data_size: read_u32(b, 28)?,
        };
        let format = PixelFormat::from_word(header.format_word);
        let mip_count = mip_count_for(format, header.width, header.height, header.data_size);
        Ok(Self {
            header,
            format,
            mip_count,
            data: &b[HEADER_LEN..],
        })
    }

    pub fn is_packed(&self) -> bool {
        self.header.storage == STORAGE_PACKED
    }

    /// The full mip chain as raw pixel data, unpacking if needed.
    pub fn pixels(&self) -> Result<Cow<'a, [u8]>> {
        if !self.is_packed() {
            return Ok(Cow::Borrowed(self.data));
        }
        let bpp = self
            .format
            .bytes_per_pixel()
            .ok_or_else(|| Error::Unsupported(format!("packed {:?}", self.format)))?;
        unpack(self.data, bpp, self.header.data_size as usize).map(Cow::Owned)
    }

    /// Bytes of mip level `level` within `pixels` (from [`Rfi::pixels`]).
    pub fn level<'p>(&self, pixels: &'p [u8], level: u32) -> Result<&'p [u8]> {
        let mut offset = 0usize;
        for l in 0..=level {
            let (w, h) = level_dims(self.header.width, self.header.height, l);
            let size = self
                .format
                .level_size(w, h)
                .ok_or_else(|| Error::Unsupported(format!("{:?}", self.format)))?;
            if l == level {
                return pixels.get(offset..offset + size).ok_or(Error::Truncated {
                    offset: HEADER_LEN + offset,
                    needed: size,
                    len: HEADER_LEN + pixels.len(),
                });
            }
            offset += size;
        }
        unreachable!()
    }

    /// Decodes the base level to tightly packed RGBA8.
    pub fn decode_rgba8(&self) -> Result<Vec<u8>> {
        let (w, h) = (self.header.width as usize, self.header.height as usize);
        let pixels = self.pixels()?;
        let src = self.level(&pixels, 0)?;
        let mut out = vec![0u8; w * h * 4];
        match self.format {
            PixelFormat::Bc1 | PixelFormat::Bc3 | PixelFormat::Bc4 | PixelFormat::Bc5 => {
                decode_blocks(self.format, src, w, h, &mut out)
            }
            PixelFormat::Rgb8 => expand(src, &mut out, 3, |p| [p[2], p[1], p[0], 255]),
            PixelFormat::Rgba8 => expand(src, &mut out, 4, |p| [p[2], p[1], p[0], p[3]]),
            PixelFormat::L8 => expand(src, &mut out, 1, |p| [p[0], p[0], p[0], 255]),
            PixelFormat::La8 => expand(src, &mut out, 2, |p| [p[0], p[0], p[0], p[1]]),
            PixelFormat::A8 => expand(src, &mut out, 1, |p| [255, 255, 255, p[0]]),
            PixelFormat::Unknown(w) => {
                return Err(Error::Unsupported(format!("rfi format word {w:#010x}")));
            }
        }
        Ok(out)
    }
}

pub fn level_dims(w: u32, h: u32, level: u32) -> (u32, u32) {
    ((w >> level).max(1), (h >> level).max(1))
}

fn mip_count_for(format: PixelFormat, w: u32, h: u32, data_size: u32) -> u32 {
    let mut total = 0usize;
    let mut level = 0;
    loop {
        let (lw, lh) = level_dims(w, h, level);
        let Some(size) = format.level_size(lw, lh) else {
            return 1;
        };
        total += size;
        level += 1;
        if total >= data_size as usize || (lw == 1 && lh == 1) {
            return level;
        }
    }
}

fn expand(src: &[u8], out: &mut [u8], bpp: usize, f: impl Fn(&[u8]) -> [u8; 4]) {
    for (px, dst) in src.chunks_exact(bpp).zip(out.as_chunks_mut::<4>().0) {
        dst.copy_from_slice(&f(px));
    }
}

fn decode_blocks(format: PixelFormat, src: &[u8], w: usize, h: usize, out: &mut [u8]) {
    let block_len = match format {
        PixelFormat::Bc1 | PixelFormat::Bc4 => 8,
        _ => 16,
    };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let mut tile = [0u8; 4 * 4 * 4];
    let mut narrow = [0u8; 4 * 4 * 2];
    for by in 0..bh {
        for bx in 0..bw {
            let i = (by * bw + bx) * block_len;
            let block = &src[i..i + block_len];
            match format {
                PixelFormat::Bc1 => bcdec_rs::bc1(block, &mut tile, 16),
                PixelFormat::Bc3 => bcdec_rs::bc3(block, &mut tile, 16),
                PixelFormat::Bc4 => {
                    bcdec_rs::bc4(block, &mut narrow, 4, false);
                    for p in 0..16 {
                        let l = narrow[p];
                        tile[p * 4..p * 4 + 4].copy_from_slice(&[l, l, l, 255]);
                    }
                }
                PixelFormat::Bc5 => {
                    bcdec_rs::bc5(block, &mut narrow, 8, false);
                    for p in 0..16 {
                        let (x, y) = (narrow[p * 2], narrow[p * 2 + 1]);
                        let nx = x as f32 / 127.5 - 1.0;
                        let ny = y as f32 / 127.5 - 1.0;
                        let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
                        let z = ((nz + 1.0) * 127.5) as u8;
                        tile[p * 4..p * 4 + 4].copy_from_slice(&[x, y, z, 255]);
                    }
                }
                _ => unreachable!(),
            }
            for row in 0..4 {
                let y = by * 4 + row;
                if y >= h {
                    break;
                }
                let cols = (w - bx * 4).min(4);
                let dst = (y * w + bx * 4) * 4;
                out[dst..dst + cols * 4].copy_from_slice(&tile[row * 16..row * 16 + cols * 4]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_words_from_shipped_data() {
        assert_eq!(PixelFormat::from_word(0x813B_C600), PixelFormat::Bc1);
        assert_eq!(PixelFormat::from_word(0x823B_C600), PixelFormat::Bc1);
        assert_eq!(PixelFormat::from_word(0x813B_4200), PixelFormat::Bc4);
        assert_eq!(PixelFormat::from_word(0x927B_8400), PixelFormat::Bc5);
        assert_eq!(PixelFormat::from_word(0x817B_E608), PixelFormat::Bc3);
        assert_eq!(PixelFormat::from_word(0x0100_E608), PixelFormat::Rgba8);
        assert_eq!(PixelFormat::from_word(0x0100_6208), PixelFormat::La8);
        assert_eq!(PixelFormat::from_word(0x0100_2008), PixelFormat::A8);
    }

    #[test]
    fn rle_literals_and_runs() {
        // 1 byte/pixel: literal of 2, then a run of 64 - 61 = 3.
        assert_eq!(unpack(&[1, 7, 8, 64, 9], 1, 5).unwrap(), [7, 8, 9, 9, 9]);
        // 2 bytes/pixel: threshold 63, run of 2.
        assert_eq!(unpack(&[63, 1, 2], 2, 4).unwrap(), [1, 2, 1, 2]);
        assert!(unpack(&[5, 1], 1, 6).is_err());
    }

    #[test]
    fn mip_chain_detection() {
        // 1024x1024 BC1 with a full chain is 699064 bytes.
        assert_eq!(mip_count_for(PixelFormat::Bc1, 1024, 1024, 699_064), 11);
        assert_eq!(mip_count_for(PixelFormat::Bc1, 1024, 1024, 524_288), 1);
    }
}
