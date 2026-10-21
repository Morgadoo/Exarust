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
//!
//! Uncompressed colour pixels are stored red first (R, G, B and R, G, B, A),
//! in both storage modes: the original copies unpacked pixels byte for byte
//! and uploads both formats as RGB/RGBA, never BGR/BGRA.

use std::borrow::Cow;

use crate::{Error, Result, pixel, read_u32};

pub const RFI_MAGIC: u32 = 0x1D2D_3DC6;
const HEADER_LEN: usize = 32;
const STORAGE_PACKED: u32 = 0x5000_0000;

/// The header flag of an image that stores a complete mip chain.
pub const FLAG_MIP_CHAIN: u32 = 0x0008_0000;
/// The header flags that repeat the image across its width, height and depth;
/// without them it clamps to the edge.
pub const FLAG_REPEAT_S: u32 = 0x0001_0000;
pub const FLAG_REPEAT_T: u32 = 0x0002_0000;
pub const FLAG_REPEAT_R: u32 = 0x0004_0000;
/// The texture detail class (0–3), in bits 22 and 23.
pub const FLAG_DETAIL_CLASS: u32 = 0x00C0_0000;

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
    /// Stored as R, G, B.
    Rgb8,
    /// Stored as R, G, B, A.
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

impl RfiHeader {
    /// Whether the file stores a complete mip chain after the base level.
    pub fn has_mip_chain(&self) -> bool {
        self.flags & FLAG_MIP_CHAIN != 0
    }

    /// The texture's detail class (0–3), which decides how many leading mip
    /// levels a low `TextureDetail` setting may skip.
    pub fn detail_class(&self) -> u32 {
        (self.flags & FLAG_DETAIL_CLASS) >> 22
    }

    /// Whether the image repeats (`true`) or clamps on each axis.
    pub fn repeats(&self) -> [bool; 3] {
        [
            self.flags & FLAG_REPEAT_S != 0,
            self.flags & FLAG_REPEAT_T != 0,
            self.flags & FLAG_REPEAT_R != 0,
        ]
    }

    /// The first mip level the loader uploads as level 0: the detail class
    /// less the detail setting, plus one, floored at zero, then raised while
    /// the width and then the height shifted by it are larger than
    /// `max_size` (the device's largest texture side). Skipping a level
    /// halves the stored resolution; the format never changes.
    pub fn first_mip_level(&self, texture_detail: u32, max_size: u32) -> u32 {
        let class = self.detail_class();
        let skipped = class as i64 - texture_detail as i64 + 1;
        let mut level = skipped.clamp(0, 32) as u32;
        while level < 32 && (self.width >> level) > max_size {
            level += 1;
        }
        while level < 32 && (self.height >> level) > max_size {
            level += 1;
        }
        level
    }
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
        let pixels = self.pixels()?;
        self.decode_level_rgba8(&pixels, 0)
    }

    /// Decodes every stored mip level to RGBA8, base first, back to back.
    pub fn decode_mips_rgba8(&self) -> Result<Vec<u8>> {
        let pixels = self.pixels()?;
        let mut out = Vec::new();
        for level in 0..self.mip_count {
            out.extend(self.decode_level_rgba8(&pixels, level)?);
        }
        Ok(out)
    }

    /// The registered pixel format whose converter reads this image: the
    /// stored word when the original registers it, else the plain format of
    /// its layout.
    fn converter_format(&self) -> u32 {
        let word = self.header.format_word;
        if pixel::lookup(word).is_some() {
            return word;
        }
        match self.format {
            PixelFormat::L8 => pixel::id::LUM8,
            PixelFormat::La8 => pixel::id::LUMA8,
            PixelFormat::A8 => pixel::id::ALPHA8,
            _ => pixel::id::RGB8,
        }
    }

    /// Decodes mip level `level` of `pixels` (from [`Rfi::pixels`]).
    pub fn decode_level_rgba8(&self, pixels: &[u8], level: u32) -> Result<Vec<u8>> {
        let (w, h) = level_dims(self.header.width, self.header.height, level);
        let (w, h) = (w as usize, h as usize);
        let src = self.level(pixels, level)?;
        let mut out = vec![0u8; w * h * 4];
        match self.format {
            PixelFormat::Bc1 | PixelFormat::Bc3 | PixelFormat::Bc4 | PixelFormat::Bc5 => {
                decode_blocks(self.format, src, w, h, &mut out)
            }
            // Already the target layout: the converters only run between
            // different layouts.
            PixelFormat::Rgba8 => out.copy_from_slice(&src[..w * h * 4]),
            PixelFormat::Rgb8 | PixelFormat::L8 | PixelFormat::La8 | PixelFormat::A8 => {
                // The original's converter of the stored format, to RGBA8.
                let source = self.converter_format();
                if !pixel::convert(source, pixel::id::RGBA8, src, w * h, &mut out)? {
                    return Err(Error::Unsupported(format!(
                        "rfi format word {source:#010x} to RGBA8"
                    )));
                }
            }
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
        assert_eq!(PixelFormat::from_word(0x0100_C600), PixelFormat::Rgb8);
        assert_eq!(PixelFormat::from_word(0x1111_C600), PixelFormat::Rgb8);
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

    /// A 32-byte header followed by `body`.
    fn image(w: u32, h: u32, word: u32, storage: u32, size: u32, body: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [RFI_MAGIC, w, h, 1, word, 0, storage, size] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(body);
        b
    }

    #[test]
    fn uncompressed_colour_is_stored_red_first() {
        // An ember orange and a sky blue, raw.
        let raw = image(
            2,
            1,
            0x0100_C600,
            0x1000_0000,
            6,
            &[200, 90, 20, 30, 120, 220],
        );
        let rfi = Rfi::parse(&raw).unwrap();
        assert_eq!(
            rfi.decode_rgba8().unwrap(),
            [200, 90, 20, 255, 30, 120, 220, 255]
        );
        // The same pixels packed: a literal of two, then a run of three.
        let packed = image(
            5,
            1,
            0x0100_C600,
            0x5000_0000,
            15,
            &[1, 200, 90, 20, 30, 120, 220, 64, 1, 2, 3],
        );
        let rfi = Rfi::parse(&packed).unwrap();
        assert!(rfi.is_packed());
        assert_eq!(
            rfi.decode_rgba8().unwrap(),
            [
                200, 90, 20, 255, 30, 120, 220, 255, 1, 2, 3, 255, 1, 2, 3, 255, 1, 2, 3, 255
            ]
        );
        // RGBA keeps its bytes, raw and packed (a run of two).
        let raw = image(1, 1, 0x0100_E608, 0x1000_0000, 4, &[200, 90, 20, 128]);
        assert_eq!(
            Rfi::parse(&raw).unwrap().decode_rgba8().unwrap(),
            [200, 90, 20, 128]
        );
        let packed = image(2, 1, 0x0100_E608, 0x5000_0000, 8, &[63, 200, 90, 20, 128]);
        assert_eq!(
            Rfi::parse(&packed).unwrap().decode_rgba8().unwrap(),
            [200, 90, 20, 128, 200, 90, 20, 128]
        );
        // A data-flagged RGB8 normal map (z in the third byte) stays x, y, z.
        let normal = image(1, 1, 0x1111_C600, 0x1000_0000, 3, &[128, 128, 255]);
        assert_eq!(
            Rfi::parse(&normal).unwrap().decode_rgba8().unwrap(),
            [128, 128, 255, 255]
        );
    }

    #[test]
    fn decodes_every_stored_mip_level() {
        // A 2x2 luminance image with its 1x1 level.
        let data = [10, 20, 30, 40, 99];
        let rfi = Rfi {
            header: RfiHeader {
                width: 2,
                height: 2,
                depth: 1,
                format_word: 0,
                flags: 0,
                storage: 0,
                data_size: data.len() as u32,
            },
            format: PixelFormat::L8,
            mip_count: 2,
            data: &data,
        };
        let mips = rfi.decode_mips_rgba8().unwrap();
        assert_eq!(mips.len(), 2 * 2 * 4 + 4);
        assert_eq!(&mips[16..], &[99, 99, 99, 255]);
        assert_eq!(rfi.decode_rgba8().unwrap(), mips[..16]);
    }

    #[test]
    fn mip_chain_detection() {
        // 1024x1024 BC1 with a full chain is 699064 bytes.
        assert_eq!(mip_count_for(PixelFormat::Bc1, 1024, 1024, 699_064), 11);
        assert_eq!(mip_count_for(PixelFormat::Bc1, 1024, 1024, 524_288), 1);
    }

    #[test]
    fn the_flag_word_carries_detail_class_wrap_and_the_chain() {
        // A shipped diffuse: class 2, stored chain, repeat on both axes.
        let header = RfiHeader {
            width: 1024,
            height: 1024,
            depth: 1,
            format_word: 0x813B_C600,
            flags: 0x20AB_0000,
            storage: 0x1000_0000,
            data_size: 699_064,
        };
        assert!(header.has_mip_chain());
        assert_eq!(header.detail_class(), 2);
        assert_eq!(header.repeats(), [true, true, false]);
        // A data map that clamps on both axes: 0x20A80000.
        let clamped = RfiHeader {
            flags: 0x20A8_0000,
            ..header
        };
        assert_eq!(clamped.repeats(), [false, false, false]);
        // No chain, class 0.
        let plain = RfiHeader {
            flags: 0x0001_0000,
            ..header
        };
        assert!(!plain.has_mip_chain());
        assert_eq!(plain.detail_class(), 0);
        assert_eq!(plain.repeats(), [true, false, false]);
    }

    #[test]
    fn the_detail_setting_chooses_the_first_uploaded_level() {
        const NO_LIMIT: u32 = 65_536;
        let header = |width, height, class: u32| RfiHeader {
            width,
            height,
            depth: 1,
            format_word: 0x813B_C600,
            flags: class << 22,
            storage: 0x1000_0000,
            data_size: 0,
        };
        // class − detail + 1, floored at zero: the shrink table.
        assert_eq!(header(1024, 1024, 0).first_mip_level(3, NO_LIMIT), 0);
        assert_eq!(header(1024, 1024, 1).first_mip_level(3, NO_LIMIT), 0);
        assert_eq!(header(1024, 1024, 2).first_mip_level(3, NO_LIMIT), 0);
        assert_eq!(header(1024, 1024, 3).first_mip_level(3, NO_LIMIT), 1);
        assert_eq!(header(1024, 1024, 3).first_mip_level(2, NO_LIMIT), 2);
        assert_eq!(header(1024, 1024, 2).first_mip_level(2, NO_LIMIT), 1);
        assert_eq!(header(1024, 1024, 2).first_mip_level(4, NO_LIMIT), 0);
        // With no detail setting every class above zero skips a level.
        assert_eq!(header(1024, 1024, 0).first_mip_level(0, NO_LIMIT), 1);
        assert_eq!(header(131_072, 65_537, 0).first_mip_level(0, NO_LIMIT), 1);
        assert_eq!(header(262_144, 1024, 0).first_mip_level(0, NO_LIMIT), 2);
        // The limit is the driver's largest side, which is what replaces the
        // static 65536 on a real device.
        assert_eq!(header(32_768, 16, 0).first_mip_level(0, 16_384), 1);
    }
}
