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
/// The header flags that make an image a cube map of six faces. Each face
/// stores its own complete mip chain, face after face: `genref01r` is six
/// 256² RGB8 chains in one file. The header's `data_size` is one face's.
pub const FLAG_CUBE: u32 = 0x4000_0000;
/// The mask the cube test applies to the header flags: the image is a cube
/// when the masked bits read exactly [`FLAG_CUBE`].
pub const FLAG_CUBE_MASK: u32 = 0x7000_0000;
/// The format-word bits two words must share to be the same pixel layout
/// for the image conversion (the flag and data-kind bits are left out).
const LAYOUT_MASK: u32 = 0x8F0C_FFFF;

/// The number of bytes one level of `w` × `h` takes in the format `word`,
/// as the original computes it from the word's fields:
///
/// - bit 31: block compression. The block's bytes are 1 + bits 19–23
///   (`0x8` for BC1/BC4, `0x10` for BC3/BC5); the block side is 1 + bits
///   16–18 (4), and a level is whole blocks across and down.
/// - bit 18: bit-packed (uncompressed). The bits per pixel are bits 6–12
///   plus bits 0–5 (24 for RGB8), and a level is `ceil(w·h·bits / 8)`.
/// - otherwise byte-aligned pixels: `w·h·ceil(bits / 8)`.
pub fn format_level_size(word: u32, w: u32, h: u32) -> Option<usize> {
    let bits = ((word >> 6) & 0x7F) + (word & 0x3F);
    let size = if word & 0x8000_0000 != 0 {
        let block = u64::from(((word & 0x00F8_0000) >> 19) + 1);
        let side = u64::from(((word & 0x0007_0000) >> 16) + 1);
        let across = u64::from(w).div_ceil(side);
        let down = u64::from(h).div_ceil(side);
        block.checked_mul(across)?.checked_mul(down)?
    } else if word & 0x0004_0000 != 0 {
        let total = u64::from(bits)
            .checked_mul(u64::from(w))?
            .checked_mul(u64::from(h))?;
        (total + 7) >> 3
    } else {
        let bytes = u64::from(bits).div_ceil(8);
        bytes.checked_mul(u64::from(w))?.checked_mul(u64::from(h))?
    };
    usize::try_from(size).ok()
}

/// The halved sides of each level after the base: while `w + h > 2`, each
/// side above one halves. The original's chain loops all walk it.
fn chain_levels(mut w: u32, mut h: u32) -> impl Iterator<Item = (u32, u32)> {
    std::iter::from_fn(move || {
        if u64::from(w) + u64::from(h) <= 2 {
            return None;
        }
        if w > 1 {
            w >>= 1;
        }
        if h > 1 {
            h >>= 1;
        }
        Some((w, h))
    })
}

/// The bytes one face of an image takes, as the original's size routine
/// computes it: the base level in `word` ([`format_level_size`]), plus every
/// halved level when `flags` carries [`FLAG_MIP_CHAIN`]. A cube's faces are
/// not counted (its callers multiply by six).
pub fn image_size(word: u32, flags: u32, w: u32, h: u32) -> Option<usize> {
    let mut total = format_level_size(word, w, h)?;
    if flags & FLAG_MIP_CHAIN != 0 {
        for (lw, lh) in chain_levels(w, h) {
            total = total.checked_add(format_level_size(word, lw, lh)?)?;
        }
    }
    Some(total)
}

/// The pixel count an image's converters walk: the base `w·h`, plus every
/// halved level when `flags` carries [`FLAG_MIP_CHAIN`], times six for a
/// cube ([`FLAG_CUBE`]).
pub fn image_pixels(flags: u32, w: u32, h: u32) -> u64 {
    let mut pixels = u64::from(w) * u64::from(h);
    if flags & FLAG_MIP_CHAIN != 0 {
        for (lw, lh) in chain_levels(w, h) {
            pixels += u64::from(lw) * u64::from(lh);
        }
    }
    if flags & FLAG_CUBE_MASK == FLAG_CUBE {
        pixels *= 6;
    }
    pixels
}
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
    /// BC1 with one-bit transparency (the block's second mode stores black
    /// with alpha 0), 8 bytes per block.
    Bc1Alpha,
    /// RGBA, 16 bytes per block.
    Bc3,
    /// One channel (luminance), 8 bytes per block.
    Bc4,
    /// One channel stored as alpha only (`ALPHATC`), 8 bytes per block;
    /// decoded to RGBA as the alpha plane under white.
    Bc4Alpha,
    /// Luminance-alpha `LATC`, 16 bytes per block. The original's converter
    /// serves only an RGB target, from the colour block in the second half
    /// of each block; the first half is never read.
    Latc,
    /// Two channels (normal map X/Y), 16 bytes per block.
    Bc5,
    /// Stored as R, G, B.
    Rgb8,
    /// Stored as R, G, B, A.
    Rgba8,
    /// Stored as B, G, R.
    Bgr8,
    /// Stored as B, G, R, A.
    Bgra8,
    /// Stored as Z, Y, X (a normal map whose red and blue are swapped).
    Zyx8,
    /// Stored as Z, Y, X, V.
    Zyxv8,
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
    /// derived from data sizes across every shipped image. The words the
    /// original registers in its own tables are matched whole first, because
    /// several of them share a channel mask but decode differently.
    pub fn from_word(word: u32) -> PixelFormat {
        match word {
            0x813B_C600 | 0x823B_C600 => return PixelFormat::Bc1,
            0x813B_E601 => return PixelFormat::Bc1Alpha,
            0x817B_E608 | 0x827B_E608 | 0x827B_A408 => return PixelFormat::Bc3,
            0x813B_4200 => return PixelFormat::Bc4,
            0x813B_2008 => return PixelFormat::Bc4Alpha,
            0x817B_6208 => return PixelFormat::Latc,
            0x927B_8400 => return PixelFormat::Bc5,
            0x0200_C600 => return PixelFormat::Bgr8,
            0x0200_E608 => return PixelFormat::Bgra8,
            0x1201_C600 => return PixelFormat::Zyx8,
            0x1201_E608 => return PixelFormat::Zyxv8,
            _ => {}
        }
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
            PixelFormat::Rgb8 | PixelFormat::Bgr8 | PixelFormat::Zyx8 => Some(3),
            PixelFormat::Rgba8 | PixelFormat::Bgra8 | PixelFormat::Zyxv8 => Some(4),
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
            PixelFormat::Bc1 | PixelFormat::Bc1Alpha | PixelFormat::Bc4 | PixelFormat::Bc4Alpha => {
                blocks() * 8
            }
            PixelFormat::Bc3 | PixelFormat::Latc | PixelFormat::Bc5 => blocks() * 16,
            PixelFormat::Rgb8 | PixelFormat::Bgr8 | PixelFormat::Zyx8 => px * 3,
            PixelFormat::Rgba8 | PixelFormat::Bgra8 | PixelFormat::Zyxv8 => px * 4,
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

    /// Whether the image is a cube map of six faces, each with its own
    /// complete mip chain.
    pub fn is_cube(&self) -> bool {
        self.flags & FLAG_CUBE_MASK == FLAG_CUBE
    }

    /// How many faces the image stores ([`RfiHeader::is_cube`]).
    pub fn faces(&self) -> u32 {
        if self.is_cube() { 6 } else { 1 }
    }

    /// Bytes of one level of this image's format word, from the word's own
    /// fields ([`format_level_size`]). A hand-built header without a format
    /// word falls back on the decoded [`PixelFormat`].
    pub fn level_size(&self, w: u32, h: u32) -> Option<usize> {
        format_level_size(self.format_word, w, h)
            .filter(|&size| size > 0)
            .or_else(|| PixelFormat::from_word(self.format_word).level_size(w, h))
    }

    /// Bytes of one face as the original sizes it ([`image_size`]): the
    /// base level, and the chain when the header says it is stored.
    pub fn image_size(&self) -> Option<usize> {
        image_size(self.format_word, self.flags, self.width, self.height)
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
        let mip_count = mip_count_for(
            format,
            header.format_word,
            header.width,
            header.height,
            header.data_size,
        );
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

    /// The pixel stream as stored, or unpacked. A packed stream unpacks to
    /// `data_size` bytes, as the original's decoder does (one face's worth;
    /// no shipped cube is packed).
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
                .header
                .level_size(w, h)
                .or_else(|| self.format.level_size(w, h))
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

    /// The number of mip levels of one face the image stores.
    pub fn mip_count(&self) -> u32 {
        self.mip_count
    }

    /// Bytes of one face's stored chain, faces excluded: every level
    /// [`Rfi::mip_count`] counts.
    pub fn face_chain_len(&self) -> Result<usize> {
        let mut total = 0usize;
        for level in 0..self.mip_count {
            let (w, h) = level_dims(self.header.width, self.header.height, level);
            let size = self.stored_level_size(w, h).ok_or_else(|| {
                Error::Unsupported(format!("rfi format word {:#010x}", self.header.format_word))
            })?;
            total = total.checked_add(size).ok_or_else(|| {
                Error::Unsupported(format!("rfi format word {:#010x}", self.header.format_word))
            })?;
        }
        Ok(total)
    }

    /// One level's bytes from the stored word, else from the decoded layout
    /// (a hand-built header carries no word).
    fn stored_level_size(&self, w: u32, h: u32) -> Option<usize> {
        self.header
            .level_size(w, h)
            .or_else(|| self.format.level_size(w, h))
    }

    /// The raw (unpacked) bytes of one face's mip chain. A cube stores its
    /// six faces back to back, one `data_size` apart (the stride the
    /// original's image copy steps by).
    pub fn face_chain(&self, face: u32) -> Result<Cow<'a, [u8]>> {
        let face_len = self.face_chain_len()?;
        let start = face as usize * self.header.data_size as usize;
        let end = start + face_len;
        let pixels = self.pixels()?;
        match &pixels {
            Cow::Borrowed(all) => all
                .get(start..end)
                .map(Cow::Borrowed)
                .ok_or(Error::Truncated {
                    offset: HEADER_LEN + start,
                    needed: face_len,
                    len: HEADER_LEN + all.len(),
                }),
            Cow::Owned(all) => {
                all.get(start..end)
                    .map(|s| Cow::Owned(s.to_vec()))
                    .ok_or(Error::Truncated {
                        offset: HEADER_LEN + start,
                        needed: face_len,
                        len: HEADER_LEN + all.len(),
                    })
            }
        }
    }

    /// Decodes one face's whole stored mip chain to RGBA8, base level first.
    pub fn decode_face_mips_rgba8(&self, face: u32) -> Result<Vec<u8>> {
        let chain = self.face_chain(face)?;
        let mut out = Vec::new();
        let mut offset = 0usize;
        for level in 0..self.mip_count {
            let (w, h) = level_dims(self.header.width, self.header.height, level);
            let size = self
                .header
                .level_size(w, h)
                .or_else(|| self.format.level_size(w, h))
                .ok_or_else(|| Error::Unsupported(format!("{:?}", self.format)))?;
            let src = chain.get(offset..offset + size).ok_or(Error::Truncated {
                offset: HEADER_LEN + offset,
                needed: size,
                len: HEADER_LEN + chain.len(),
            })?;
            self.decode_level_into_rgba8(src, w as usize, h as usize, &mut out)?;
            offset += size;
        }
        Ok(out)
    }

    /// Decodes every face's mip chain to RGBA8; a plane has one face.
    pub fn decode_faces_mips_rgba8(&self) -> Result<Vec<Vec<u8>>> {
        (0..self.header.faces())
            .map(|face| self.decode_face_mips_rgba8(face))
            .collect()
    }

    /// The base level of each face, tightly packed RGBA8.
    pub fn decode_faces_rgba8(&self) -> Result<Vec<Vec<u8>>> {
        let (w, h) = (self.header.width as usize, self.header.height as usize);
        let faces = self.decode_faces_mips_rgba8()?;
        Ok(faces.into_iter().map(|f| f[..w * h * 4].to_vec()).collect())
    }

    /// The base level of the first face, tightly packed RGBA8.
    pub fn decode_rgba8(&self) -> Result<Vec<u8>> {
        let chain = self.face_chain(0)?;
        let (w, h) = (self.header.width as usize, self.header.height as usize);
        let mut out = Vec::new();
        let size = self
            .header
            .level_size(self.header.width, self.header.height)
            .or_else(|| {
                self.format
                    .level_size(self.header.width, self.header.height)
            })
            .ok_or_else(|| Error::Unsupported(format!("{:?}", self.format)))?;
        let src = chain.get(..size).ok_or(Error::Truncated {
            offset: HEADER_LEN,
            needed: size,
            len: HEADER_LEN + chain.len(),
        })?;
        self.decode_level_into_rgba8(src, w, h, &mut out)?;
        Ok(out)
    }

    /// Decodes every stored mip level of the first face to RGBA8, base
    /// first, back to back.
    pub fn decode_mips_rgba8(&self) -> Result<Vec<u8>> {
        self.decode_face_mips_rgba8(0)
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
            PixelFormat::Bgr8 => pixel::id::BGR8,
            PixelFormat::Bgra8 => pixel::id::BGRA8,
            PixelFormat::Zyx8 => pixel::id::ZYX8,
            PixelFormat::Zyxv8 => pixel::id::ZYXV8,
            _ => pixel::id::RGB8,
        }
    }

    /// Decodes mip level `level` of `pixels` (from [`Rfi::pixels`]).
    pub fn decode_level_rgba8(&self, pixels: &[u8], level: u32) -> Result<Vec<u8>> {
        let (w, h) = level_dims(self.header.width, self.header.height, level);
        let src = self.level(pixels, level)?;
        let mut out = Vec::new();
        self.decode_level_into_rgba8(src, w as usize, h as usize, &mut out)?;
        Ok(out)
    }

    /// Appends mip level pixels `src` (`w` × `h`) as tightly packed RGBA8 to
    /// `out`.
    fn decode_level_into_rgba8(
        &self,
        src: &[u8],
        w: usize,
        h: usize,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        let start = out.len();
        out.resize(start + w * h * 4, 0);
        let out = &mut out[start..];
        match self.format {
            PixelFormat::Bc1
            | PixelFormat::Bc1Alpha
            | PixelFormat::Bc3
            | PixelFormat::Bc4
            | PixelFormat::Bc4Alpha
            | PixelFormat::Latc
            | PixelFormat::Bc5 => decode_blocks(self.format, src, w, h, out),
            // Already the target layout: the converters only run between
            // different layouts.
            PixelFormat::Rgba8 => {
                let src = src.get(..w * h * 4).ok_or(Error::Truncated {
                    offset: HEADER_LEN,
                    needed: w * h * 4,
                    len: HEADER_LEN + src.len(),
                })?;
                out.copy_from_slice(src);
            }
            PixelFormat::Rgb8
            | PixelFormat::Bgr8
            | PixelFormat::Bgra8
            | PixelFormat::Zyx8
            | PixelFormat::Zyxv8
            | PixelFormat::L8
            | PixelFormat::La8
            | PixelFormat::A8 => {
                // The original's converter of the stored format, to RGBA8.
                let source = self.converter_format();
                if !pixel::convert(source, pixel::id::RGBA8, src, w * h, out)? {
                    return Err(Error::Unsupported(format!(
                        "rfi format word {source:#010x} to RGBA8"
                    )));
                }
            }
            PixelFormat::Unknown(w) => {
                return Err(Error::Unsupported(format!("rfi format word {w:#010x}")));
            }
        }
        Ok(())
    }
}

/// An image whose pixels are owned: the original's image copies and the
/// blank images its icon renderer and noise generators create. The header
/// keeps a cube's per-face `data_size`; `pixels` holds every face.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageData {
    pub header: RfiHeader,
    pub format: PixelFormat,
    pub pixels: Vec<u8>,
}

/// Storage word of an image whose pixels follow its header in one block.
const STORAGE_INLINE: u32 = 0x1000_0000;
/// Storage word of an image whose pixels are a separate allocation.
const STORAGE_SEPARATE: u32 = 0x2000_0000;
/// The detail class a render target gets when its flags name none (2).
const TARGET_DETAIL_CLASS: u32 = 0x0080_0000;

impl ImageData {
    /// A copy of an image, as the original's image copy makes it with its
    /// keep-chain argument set: a run-length encoded source is unpacked
    /// first (the original logs "Source image is run length encoded."), and
    /// `data_size` bytes of each face are copied, one `data_size` apart.
    pub fn copy_of(image: &Rfi<'_>) -> Result<Self> {
        let face = image.header.data_size as usize;
        let faces = image.header.faces() as usize;
        let source = image.pixels()?;
        let len = face
            .checked_mul(faces)
            .ok_or_else(|| Error::Unsupported(format!("image size {face}")))?;
        let pixels = source.get(..len).ok_or(Error::Truncated {
            offset: HEADER_LEN,
            needed: len,
            len: HEADER_LEN + source.len(),
        })?;
        Ok(Self {
            header: RfiHeader {
                storage: STORAGE_INLINE,
                ..image.header
            },
            format: image.format,
            pixels: pixels.to_vec(),
        })
    }

    /// A copy of an image's base level only, as the original's image copy
    /// makes it with its keep-chain argument clear: the copy's chain flag is
    /// cleared, its `data_size` is the base level's ([`image_size`] without
    /// the chain), and each face's base is taken from the source one source
    /// `data_size` apart.
    pub fn copy_base(image: &Rfi<'_>) -> Result<Self> {
        let mut header = RfiHeader {
            storage: STORAGE_INLINE,
            ..image.header
        };
        header.flags &= !FLAG_MIP_CHAIN;
        let base = header.image_size().ok_or_else(|| {
            Error::Unsupported(format!("rfi format word {:#010x}", header.format_word))
        })?;
        let stride = image.header.data_size as usize;
        let source = image.pixels()?;
        let faces = image.header.faces() as usize;
        let mut pixels = Vec::with_capacity(base.saturating_mul(faces));
        for face in 0..faces {
            let start = face * stride;
            let slice = source.get(start..start + base).ok_or(Error::Truncated {
                offset: HEADER_LEN + start,
                needed: base,
                len: HEADER_LEN + source.len(),
            })?;
            pixels.extend_from_slice(slice);
        }
        header.data_size =
            u32::try_from(base).map_err(|_| Error::Unsupported(format!("image size {base}")))?;
        Ok(Self {
            header,
            format: image.format,
            pixels,
        })
    }

    /// An image of zeroes, `data_size` per face ([`image_size`], so a
    /// chain flag in `flags` sizes the chain), six faces for a cube.
    fn zeroed(header: RfiHeader) -> Result<Self> {
        let size = header.image_size().ok_or_else(|| {
            Error::Unsupported(format!("rfi format word {:#010x}", header.format_word))
        })?;
        let len = size
            .checked_mul(header.faces() as usize)
            .ok_or_else(|| Error::Unsupported(format!("image size {size}")))?;
        Ok(Self {
            header: RfiHeader {
                data_size: u32::try_from(size)
                    .map_err(|_| Error::Unsupported(format!("image size {size}")))?,
                ..header
            },
            format: PixelFormat::from_word(header.format_word),
            pixels: vec![0; len],
        })
    }

    /// A blank image, as the original's create call makes it: depth 1, the
    /// flags and format word as given, `data_size` from the size routine
    /// (the chain included when `flags` carries it) and a separately
    /// allocated pixel store of six faces for a cube.
    pub fn blank(width: u16, height: u16, format_word: u32, flags: u32) -> Result<Self> {
        Self::zeroed(RfiHeader {
            width: u32::from(width),
            height: u32::from(height),
            depth: 1,
            format_word,
            flags,
            storage: STORAGE_SEPARATE,
            data_size: 0,
        })
    }

    /// The blank image the original makes for a render target (its icon
    /// renderer's): as [`ImageData::blank`], but flags with no detail class
    /// get class 2, and the pixels follow the header in one block.
    pub fn render_target(width: u16, height: u16, format_word: u32, flags: u32) -> Result<Self> {
        let flags = if flags & FLAG_DETAIL_CLASS == 0 {
            flags | TARGET_DETAIL_CLASS
        } else {
            flags
        };
        Self::zeroed(RfiHeader {
            width: u32::from(width),
            height: u32::from(height),
            depth: 1,
            format_word,
            flags,
            storage: STORAGE_INLINE,
            data_size: 0,
        })
    }

    /// Resizes the pixel store, as the original's resize does: `data_size`
    /// becomes `size`, and the store holds six times that for a cube.
    /// Existing bytes stay, new ones are zero; shrinking drops the tail.
    pub fn resize(&mut self, size: usize) -> Result<()> {
        self.header.data_size =
            u32::try_from(size).map_err(|_| Error::Unsupported(format!("image size {size}")))?;
        let len = size
            .checked_mul(self.header.faces() as usize)
            .ok_or_else(|| Error::Unsupported(format!("image size {size}")))?;
        self.pixels.resize(len, 0);
        Ok(())
    }

    /// Frees the pixel store; the header (its `data_size` too) stays, as
    /// the original's release leaves it.
    pub fn release(&mut self) {
        self.pixels = Vec::new();
    }

    /// Converts the image to the format word `requested` (0: the source
    /// format's registered fallback), as the original's conversion does:
    ///
    /// - the same word: nothing to do;
    /// - a requested word with bit 20 set and bit 31 clear whose layout
    ///   (under `0x8F0C_FFFF`) is the image's: the word changes, the pixels
    ///   do not;
    /// - otherwise the first registered format whose id is the image's word
    ///   ([`pixel::lookup`]) converts it: a zero request takes that format's
    ///   fallback; an image word with bit 20 set and bit 31 clear and the
    ///   target's layout only changes its word; else the converter writes
    ///   every pixel of every face and stored level ([`image_pixels`]) in
    ///   the target word, and `data_size` becomes the target's
    ///   [`image_size`]. The chain flag stays.
    ///
    /// Returns `false` when the image's word is not a registered format or
    /// its converter declines the target. Exarust's table holds the 14
    /// eight-bit and 6 float formats; the original also registers the
    /// block-compressed ones, which do not convert here.
    pub fn convert(&mut self, requested: u32) -> Result<bool> {
        let word = self.header.format_word;
        if requested == word {
            return Ok(true);
        }
        let same_layout = |a: u32, b: u32| a & LAYOUT_MASK == b & LAYOUT_MASK;
        let plain = |w: u32| w & 0x8010_0000 == 0x0010_0000;
        if plain(requested) && same_layout(word, requested) {
            self.header.format_word = requested;
            return Ok(true);
        }
        let Some(source) = pixel::lookup(word) else {
            return Ok(false);
        };
        let target = if requested == 0 {
            source.fallback
        } else {
            requested
        };
        if plain(word) && same_layout(word, target) {
            self.header.format_word = target;
            return Ok(true);
        }
        let (w, h, flags) = (self.header.width, self.header.height, self.header.flags);
        let size = image_size(target, flags, w, h)
            .ok_or_else(|| Error::Unsupported(format!("rfi format word {target:#010x}")))?;
        let faces = self.header.faces() as usize;
        let mut out = vec![0u8; size.saturating_mul(faces)];
        let count = usize::try_from(image_pixels(flags, w, h))
            .map_err(|_| Error::Unsupported(format!("image of {w} x {h}")))?;
        if !pixel::convert(word, target, &self.pixels, count, &mut out)? {
            return Ok(false);
        }
        self.header.format_word = target;
        self.header.data_size =
            u32::try_from(size).map_err(|_| Error::Unsupported(format!("image size {size}")))?;
        self.pixels = out;
        self.format = PixelFormat::from_word(target);
        Ok(true)
    }

    /// The image as a borrowed [`Rfi`], for decoding.
    pub fn as_rfi(&self) -> Rfi<'_> {
        Rfi {
            header: self.header,
            format: self.format,
            mip_count: mip_count_for(
                self.format,
                self.header.format_word,
                self.header.width,
                self.header.height,
                self.header.data_size,
            ),
            data: &self.pixels,
        }
    }
}

pub fn level_dims(w: u32, h: u32, level: u32) -> (u32, u32) {
    ((w >> level).max(1), (h >> level).max(1))
}

fn mip_count_for(format: PixelFormat, word: u32, w: u32, h: u32, data_size: u32) -> u32 {
    let mut total = 0usize;
    let mut level = 0;
    loop {
        let (lw, lh) = level_dims(w, h, level);
        let Some(size) = format_level_size(word, lw, lh).or_else(|| format.level_size(lw, lh))
        else {
            return 1;
        };
        total += size;
        level += 1;
        if total >= data_size as usize || (lw == 1 && lh == 1) {
            return level;
        }
    }
}

/// Expands a 5-6-5 colour word the way the original's decoder does: each
/// channel keeps its top bits in the high bits of the byte and repeats its
/// top two or three bits in the low ones (`v << 3 | v >> 2` for five bits,
/// `v << 2 | v >> 4` for six).
fn expand_565(word: u16) -> [u8; 3] {
    let r = (word >> 11) & 0x1F;
    let g = (word >> 5) & 0x3F;
    let b = word & 0x1F;
    [
        ((r << 3) | (r >> 2)) as u8,
        ((g << 2) | (g >> 4)) as u8,
        ((b << 3) | (b >> 2)) as u8,
    ]
}

/// The four colours of a colour block: the two endpoints expanded, then
/// either the two third-point blends (when the first endpoint is larger) or
/// one half blend and transparent black. Divides truncate, as the original's
/// integer `DIV` does.
fn colour_palette(first: u16, second: u16, transparent: bool) -> [[u8; 4]; 4] {
    let a = expand_565(first);
    let b = expand_565(second);
    let mut palette = [[0u8; 4]; 4];
    for c in 0..3 {
        palette[0][c] = a[c];
        palette[1][c] = b[c];
        if first > second {
            palette[2][c] = ((u16::from(a[c]) * 2 + u16::from(b[c])) / 3) as u8;
            palette[3][c] = ((u16::from(a[c]) + u16::from(b[c]) * 2) / 3) as u8;
        } else {
            palette[2][c] = ((u16::from(a[c]) + u16::from(b[c])) >> 1) as u8;
            palette[3][c] = 0;
        }
    }
    palette[0][3] = 255;
    palette[1][3] = 255;
    palette[2][3] = 255;
    palette[3][3] = if first > second || !transparent {
        255
    } else {
        0
    };
    palette
}

/// Decodes one 8-byte colour block to 16 RGBA texels, row by row.
fn decode_colour_block(block: &[u8], out: &mut [[u8; 4]; 16], transparent: bool) {
    let first = u16::from_le_bytes([block[0], block[1]]);
    let second = u16::from_le_bytes([block[2], block[3]]);
    let palette = colour_palette(first, second, transparent);
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    for (pixel, texel) in out.iter_mut().enumerate() {
        *texel = palette[((indices >> (2 * pixel)) & 3) as usize];
    }
}

/// The eight values of a BC4/BC3 alpha block. The six interpolants divide by
/// seven or by five with a truncating integer division, without the rounding
/// term a GPU adds.
fn alpha_palette(first: u8, second: u8) -> [u8; 8] {
    let (a, b) = (u16::from(first), u16::from(second));
    let mut palette = [0u8; 8];
    palette[0] = first;
    palette[1] = second;
    if first > second {
        for (i, value) in palette.iter_mut().enumerate().skip(2) {
            let w = i as u16 - 1;
            *value = (((7 - w) * a + w * b) / 7) as u8;
        }
    } else {
        palette[2] = ((4 * a + b) / 5) as u8;
        palette[3] = ((3 * a + 2 * b) / 5) as u8;
        palette[4] = ((2 * a + 3 * b) / 5) as u8;
        palette[5] = ((a + 4 * b) / 5) as u8;
        palette[6] = 0;
        palette[7] = 255;
    }
    palette
}

/// Decodes one 8-byte alpha block to 16 values, row by row.
fn decode_alpha_block(block: &[u8], out: &mut [u8; 16]) {
    let palette = alpha_palette(block[0], block[1]);
    let mut bits = 0u64;
    for (i, &byte) in block[2..8].iter().enumerate() {
        bits |= u64::from(byte) << (8 * i);
    }
    for (pixel, value) in out.iter_mut().enumerate() {
        *value = palette[((bits >> (3 * pixel)) & 7) as usize];
    }
}

/// Decodes a two-channel block pair to X and Y.
fn decode_pair(first: &[u8], second: &[u8]) -> ([u8; 16], [u8; 16]) {
    let mut x = [0u8; 16];
    let mut y = [0u8; 16];
    decode_alpha_block(first, &mut x);
    decode_alpha_block(second, &mut y);
    (x, y)
}

fn decode_blocks(format: PixelFormat, src: &[u8], w: usize, h: usize, out: &mut [u8]) {
    let block_len = match format {
        PixelFormat::Bc1 | PixelFormat::Bc1Alpha | PixelFormat::Bc4 | PixelFormat::Bc4Alpha => 8,
        _ => 16,
    };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let mut tile = [[0u8; 4]; 16];
    let mut alpha = [0u8; 16];
    for by in 0..bh {
        for bx in 0..bw {
            let i = (by * bw + bx) * block_len;
            let Some(block) = src.get(i..i + block_len) else {
                return;
            };
            match format {
                PixelFormat::Bc1 | PixelFormat::Bc1Alpha => {
                    decode_colour_block(block, &mut tile, format == PixelFormat::Bc1Alpha);
                }
                PixelFormat::Bc3 => {
                    // The alpha block comes first, the colour block second.
                    decode_colour_block(&block[8..], &mut tile, false);
                    decode_alpha_block(block, &mut alpha);
                    for (texel, a) in tile.iter_mut().zip(alpha) {
                        texel[3] = a;
                    }
                }
                PixelFormat::Bc4 => {
                    decode_alpha_block(block, &mut alpha);
                    for (texel, l) in tile.iter_mut().zip(alpha) {
                        *texel = [l, l, l, 255];
                    }
                }
                PixelFormat::Bc4Alpha => {
                    // The alpha plane through the alpha-to-RGBA converter.
                    decode_alpha_block(block, &mut alpha);
                    for (texel, a) in tile.iter_mut().zip(alpha) {
                        *texel = [255, 255, 255, a];
                    }
                }
                PixelFormat::Latc => decode_colour_block(&block[8..], &mut tile, false),
                PixelFormat::Bc5 => {
                    let (x, y) = decode_pair(block, &block[8..]);
                    for ((texel, x), y) in tile.iter_mut().zip(x).zip(y) {
                        *texel = [x, y, pixel::rebuild_z(x, y), 255];
                    }
                }
                _ => return,
            }
            let mut row_bytes = [0u8; 16];
            for row in 0..4 {
                let y = by * 4 + row;
                if y >= h {
                    break;
                }
                let cols = (w - bx * 4).min(4);
                for column in 0..cols {
                    row_bytes[column * 4..column * 4 + 4].copy_from_slice(&tile[row * 4 + column]);
                }
                let dst = (y * w + bx * 4) * 4;
                out[dst..dst + cols * 4].copy_from_slice(&row_bytes[..cols * 4]);
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
        assert_eq!(PixelFormat::from_word(0x813B_E601), PixelFormat::Bc1Alpha);
        assert_eq!(PixelFormat::from_word(0x813B_4200), PixelFormat::Bc4);
        assert_eq!(PixelFormat::from_word(0x813B_2008), PixelFormat::Bc4Alpha);
        assert_eq!(PixelFormat::from_word(0x817B_6208), PixelFormat::Latc);
        assert_eq!(PixelFormat::from_word(0x927B_8400), PixelFormat::Bc5);
        assert_eq!(PixelFormat::from_word(0x817B_E608), PixelFormat::Bc3);
        assert_eq!(PixelFormat::from_word(0x827B_E608), PixelFormat::Bc3);
        assert_eq!(PixelFormat::from_word(0x827B_A408), PixelFormat::Bc3);
        assert_eq!(PixelFormat::from_word(0x0100_C600), PixelFormat::Rgb8);
        assert_eq!(PixelFormat::from_word(0x1111_C600), PixelFormat::Rgb8);
        assert_eq!(PixelFormat::from_word(0x0100_E608), PixelFormat::Rgba8);
        assert_eq!(PixelFormat::from_word(0x0100_6208), PixelFormat::La8);
        assert_eq!(PixelFormat::from_word(0x0100_2008), PixelFormat::A8);
        assert_eq!(PixelFormat::from_word(0x0200_C600), PixelFormat::Bgr8);
        assert_eq!(PixelFormat::from_word(0x0200_E608), PixelFormat::Bgra8);
        assert_eq!(PixelFormat::from_word(0x1201_C600), PixelFormat::Zyx8);
        assert_eq!(PixelFormat::from_word(0x1201_E608), PixelFormat::Zyxv8);
    }

    /// A 4x4 image of one block.
    fn block_image(word: u32, block: &[u8]) -> Vec<u8> {
        let size = block.len() as u32;
        image(4, 4, word, 0x1000_0000, size, block)
    }

    fn rgba_attexels(decoded: &[u8]) -> Vec<[u8; 4]> {
        decoded
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2], p[3]])
            .collect()
    }

    #[test]
    fn bc1_expands_endpoints_and_blends_with_truncating_division() {
        // Red and blue endpoints (first larger): the third-point blends are
        // (2·255)/3 = 170 and 255/3 = 85, and the indices are 0, 1, 2, 3.
        let mut block = [0u8; 8];
        block[..4].copy_from_slice(&[0x00, 0xF8, 0x1F, 0x00]);
        block[4] = 0b1110_0100;
        let texels = rgba_attexels(
            &Rfi::parse(&block_image(0x813B_C600, &block))
                .unwrap()
                .decode_rgba8()
                .unwrap(),
        );
        assert_eq!(texels[0], [255, 0, 0, 255]);
        assert_eq!(texels[1], [0, 0, 255, 255]);
        assert_eq!(texels[2], [170, 0, 85, 255]);
        assert_eq!(texels[3], [85, 0, 170, 255]);
        // The second row's indices are all zero.
        assert_eq!(texels[4], [255, 0, 0, 255]);

        // The reverse order selects the half blend and transparent black.
        let mut block = [0u8; 8];
        block[..4].copy_from_slice(&[0x1F, 0x00, 0x00, 0xF8]);
        block[4] = 0b1110_0100;
        let texels = rgba_attexels(
            &Rfi::parse(&block_image(0x813B_C600, &block))
                .unwrap()
                .decode_rgba8()
                .unwrap(),
        );
        assert_eq!(texels[2], [127, 0, 127, 255]);
        assert_eq!(texels[3], [0, 0, 0, 255]);
        // The alpha variant makes that texel transparent.
        let texels = rgba_attexels(
            &Rfi::parse(&block_image(0x813B_E601, &block))
                .unwrap()
                .decode_rgba8()
                .unwrap(),
        );
        assert_eq!(texels[3], [0, 0, 0, 0]);
    }

    /// The six bytes of a 48-bit alpha index stream.
    fn alpha_indices(indices: &[u8; 16]) -> [u8; 6] {
        let mut bits = 0u64;
        for (pixel, &index) in indices.iter().enumerate() {
            bits |= u64::from(index) << (3 * pixel);
        }
        let bytes = bits.to_le_bytes();
        [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]]
    }

    #[test]
    fn bc4_alpha_palettes_divide_without_rounding() {
        let indices = [0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut block = [0u8; 8];
        block[0] = 255;
        block[1] = 0;
        block[2..].copy_from_slice(&alpha_indices(&indices));
        let decoded = Rfi::parse(&block_image(0x813B_4200, &block))
            .unwrap()
            .decode_rgba8()
            .unwrap();
        let texels = rgba_attexels(&decoded);
        // 6·255/7 = 218, 5·255/7 = 182, … 255/7 = 36, all truncated.
        assert_eq!(
            texels[..8].iter().map(|p| p[0]).collect::<Vec<_>>(),
            [255, 0, 218, 182, 145, 109, 72, 36]
        );
        assert_eq!(texels[0], [255, 255, 255, 255]);

        // Alpha-only: the value lands in alpha, under white.
        let decoded = Rfi::parse(&block_image(0x813B_2008, &block))
            .unwrap()
            .decode_rgba8()
            .unwrap();
        assert_eq!(rgba_attexels(&decoded)[2], [255, 255, 255, 218]);

        // The second mode: /5 blends, then 0 and 255.
        let mut block = [0u8; 8];
        block[0] = 0;
        block[1] = 255;
        let indices = [2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        block[2..].copy_from_slice(&alpha_indices(&indices));
        let decoded = Rfi::parse(&block_image(0x813B_4200, &block))
            .unwrap()
            .decode_rgba8()
            .unwrap();
        let texels = rgba_attexels(&decoded);
        assert_eq!(
            texels[..6].iter().map(|p| p[0]).collect::<Vec<_>>(),
            [51, 102, 153, 204, 0, 255]
        );
        assert_eq!(texels[6], [0, 0, 0, 255]);
    }

    #[test]
    fn bc3_combines_a_colour_block_with_an_alpha_block() {
        let mut block = [0u8; 16];
        // Red endpoint over blue, all indices zero (the second half).
        block[8..12].copy_from_slice(&[0x00, 0xF8, 0x1F, 0x00]);
        // Alpha endpoint 200 over 100, all indices 0 except the last pixel
        // (the first half).
        block[0] = 200;
        block[1] = 100;
        let indices = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        block[2..8].copy_from_slice(&alpha_indices(&indices));
        let texels = rgba_attexels(
            &Rfi::parse(&block_image(0x817B_E608, &block))
                .unwrap()
                .decode_rgba8()
                .unwrap(),
        );
        assert_eq!(texels[0], [255, 0, 0, 200]);
        assert_eq!(texels[15], [255, 0, 0, 100]);
    }

    #[test]
    fn the_two_channel_blocks_rebuild_z() {
        // X and Y both 128: the middle of the signed range gives Z 255.
        let mut block = [0u8; 16];
        for half in block.chunks_mut(8) {
            half[0] = 128;
            half[1] = 128;
        }
        let texels = rgba_attexels(
            &Rfi::parse(&block_image(0x927B_8400, &block))
                .unwrap()
                .decode_rgba8()
                .unwrap(),
        );
        assert_eq!(texels[0], [128, 128, 255, 255]);
    }

    #[test]
    fn latc_reads_only_the_colour_block_of_its_second_half() {
        // Junk in the first half; a red endpoint over blue in the second.
        let mut block = [0x55u8; 16];
        block[8..12].copy_from_slice(&[0x00, 0xF8, 0x1F, 0x00]);
        block[12..].fill(0);
        let texels = rgba_attexels(
            &Rfi::parse(&block_image(0x817B_6208, &block))
                .unwrap()
                .decode_rgba8()
                .unwrap(),
        );
        assert_eq!(texels[0], [255, 0, 0, 255]);
        assert_eq!(texels[15], [255, 0, 0, 255]);
    }

    #[test]
    fn channel_order_words_swap_red_and_blue() {
        let raw = image(1, 1, 0x0200_C600, 0x1000_0000, 3, &[30, 120, 220]);
        assert_eq!(
            Rfi::parse(&raw).unwrap().decode_rgba8().unwrap(),
            [220, 120, 30, 255]
        );
        let raw = image(1, 1, 0x0200_E608, 0x1000_0000, 4, &[30, 120, 220, 128]);
        assert_eq!(
            Rfi::parse(&raw).unwrap().decode_rgba8().unwrap(),
            [220, 120, 30, 128]
        );
        let raw = image(1, 1, 0x1201_C600, 0x1000_0000, 3, &[128, 128, 255]);
        assert_eq!(
            Rfi::parse(&raw).unwrap().decode_rgba8().unwrap(),
            [255, 128, 128, 255]
        );
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
        assert_eq!(
            mip_count_for(PixelFormat::Bc1, 0x813B_C600, 1024, 1024, 699_064),
            11
        );
        assert_eq!(
            mip_count_for(PixelFormat::Bc1, 0x813B_C600, 1024, 1024, 524_288),
            1
        );
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

    #[test]
    fn a_levels_size_comes_from_the_format_words_own_fields() {
        // Byte-per-channel layouts.
        assert_eq!(
            format_level_size(0x0100_C600, 1024, 1024),
            Some(3 * 1024 * 1024)
        );
        assert_eq!(
            format_level_size(0x0100_E608, 1024, 1024),
            Some(4 * 1024 * 1024)
        );
        assert_eq!(format_level_size(0x0100_4200, 2, 2), Some(4));
        assert_eq!(format_level_size(0x0100_6208, 2, 2), Some(8));
        // A one-bit channel mask packs eight pixels a byte.
        assert_eq!(format_level_size(0x0104_4040, 8, 8), Some(8));
        // Block compression: 8 bytes per 4×4 block (BC1/BC4) and 16
        // (BC3/BC5), blocks a level across and down.
        assert_eq!(format_level_size(0x813B_C600, 1024, 1024), Some(524_288));
        assert_eq!(format_level_size(0x813B_4200, 1024, 1024), Some(524_288));
        assert_eq!(format_level_size(0x817B_E608, 1024, 1024), Some(1_048_576));
        assert_eq!(format_level_size(0x927B_8400, 1024, 1024), Some(1_048_576));
        assert_eq!(format_level_size(0x813B_C600, 6, 5), Some(8 * 2 * 2));
    }

    #[test]
    fn an_image_is_sized_with_its_chain_only_when_the_flag_says_so() {
        // 1024² BC1: 524 288 bytes for the base, 699 064 with the chain
        // (every level down to 1 × 1, whole 4 × 4 blocks).
        assert_eq!(image_size(0x813B_C600, 0, 1024, 1024), Some(524_288));
        assert_eq!(
            image_size(0x813B_C600, FLAG_MIP_CHAIN, 1024, 1024),
            Some(699_064)
        );
        // 4 × 4 RGBA8: 64 + 16 + 4.
        assert_eq!(image_size(0x0100_E608, FLAG_MIP_CHAIN, 4, 4), Some(84));
        // A cube is sized per face: the flag does not multiply.
        assert_eq!(image_size(0x0100_C600, FLAG_CUBE, 2, 2), Some(12));
    }

    #[test]
    fn the_converters_pixel_count_takes_the_chain_and_six_faces() {
        assert_eq!(image_pixels(FLAG_MIP_CHAIN, 256, 256), 87_381);
        assert_eq!(
            image_pixels(FLAG_MIP_CHAIN | FLAG_CUBE, 256, 256),
            6 * 87_381
        );
        assert_eq!(image_pixels(0, 4, 4), 16);
        assert_eq!(image_pixels(FLAG_MIP_CHAIN, 4, 4), 16 + 4 + 1);
        assert_eq!(image_pixels(FLAG_MIP_CHAIN, 1, 1), 1);
        // Only the exact cube bits count: bit 28 set as well is not a cube.
        assert_eq!(image_pixels(0x5000_0000, 2, 2), 4);
    }

    #[test]
    fn a_cube_stores_six_faces_each_with_its_own_chain() {
        // Six 2×2 RGB8 faces, no chain: per-face data_size 12.
        let mut body = Vec::new();
        for face in 0..6u8 {
            body.extend_from_slice(&[face, 10, 20, face, 30, 40, face, 50, 60, face, 70, 80]);
        }
        let mut raw = image(2, 2, 0x0100_C600, 0x1000_0000, 12, &body);
        raw[20..24].copy_from_slice(&FLAG_CUBE.to_le_bytes());
        let rfi = Rfi::parse(&raw).unwrap();
        assert!(rfi.header.is_cube());
        assert_eq!(rfi.header.faces(), 6);
        assert_eq!(rfi.header.image_size(), Some(12));
        assert_eq!(rfi.mip_count, 1);
        let faces = rfi.decode_faces_rgba8().unwrap();
        assert_eq!(faces.len(), 6);
        assert_eq!(
            faces[0],
            [
                0, 10, 20, 255, 0, 30, 40, 255, 0, 50, 60, 255, 0, 70, 80, 255
            ]
        );
        assert_eq!(faces[5][..4], [5, 10, 20, 255]);
        // Copying keeps the header and every face; the base-only copy
        // clears the chain flag.
        let copy = ImageData::copy_of(&rfi).unwrap();
        assert_eq!(copy.pixels, body);
        let base = ImageData::copy_base(&rfi).unwrap();
        assert_eq!(base.header.flags & FLAG_MIP_CHAIN, 0);
        assert_eq!(base.header.data_size, 12);
        assert_eq!(base.pixels, body);
    }

    #[test]
    fn a_base_copy_steps_through_the_faces_by_the_source_data_size() {
        // Two-level 2×2 RGB8 cube faces: 12 + 3 bytes a face.
        let mut body = Vec::new();
        for face in 0..6u8 {
            body.extend_from_slice(&[face; 12]);
            body.extend_from_slice(&[100 + face; 3]);
        }
        let mut raw = image(2, 2, 0x0100_C600, 0x1000_0000, 15, &body);
        raw[20..24].copy_from_slice(&(FLAG_CUBE | FLAG_MIP_CHAIN).to_le_bytes());
        let rfi = Rfi::parse(&raw).unwrap();
        assert_eq!(rfi.mip_count, 2);
        let base = ImageData::copy_base(&rfi).unwrap();
        assert_eq!(base.header.data_size, 12);
        assert_eq!(base.pixels.len(), 72);
        assert_eq!(&base.pixels[12..24], &[1; 12]);
        assert_eq!(&base.pixels[60..72], &[5; 12]);
        let copy = ImageData::copy_of(&rfi).unwrap();
        assert_eq!(copy.pixels, body);
    }

    #[test]
    fn images_are_created_resized_and_released() {
        let mut blank = ImageData::blank(2, 2, 0x0100_C600, 0).unwrap();
        assert_eq!(blank.header.data_size, 12);
        assert_eq!(blank.pixels.len(), 12);
        assert_eq!(blank.header.depth, 1);
        blank.resize(3).unwrap();
        assert_eq!((blank.header.data_size, blank.pixels.len()), (3, 3));
        // A release frees the store and leaves the header.
        blank.release();
        assert_eq!((blank.header.data_size, blank.pixels.len()), (3, 0));
        // A cube's store holds six faces of the per-face size.
        let mut cube = ImageData::blank(2, 2, 0x0100_C600, FLAG_CUBE).unwrap();
        assert_eq!((cube.header.data_size, cube.pixels.len()), (12, 72));
        cube.resize(6).unwrap();
        assert_eq!((cube.header.data_size, cube.pixels.len()), (6, 36));
        // A chain flag sizes the chain.
        let chained = ImageData::blank(4, 4, 0x0100_E608, FLAG_MIP_CHAIN).unwrap();
        assert_eq!(chained.header.data_size, 84);
        // A render target without a detail class gets class 2, and no
        // chain unless asked.
        let target = ImageData::render_target(4, 4, 0x0100_E608, 0).unwrap();
        assert!(!target.header.has_mip_chain());
        assert_eq!(target.header.detail_class(), 2);
        assert_eq!(target.header.data_size, 64);
        let classed = ImageData::render_target(4, 4, 0x0100_E608, 0x0040_0000).unwrap();
        assert_eq!(classed.header.detail_class(), 1);
    }

    #[test]
    fn an_image_converts_through_its_formats_converter() {
        let raw = image(2, 1, 0x0100_C600, 0x1000_0000, 6, &[10, 20, 30, 40, 50, 60]);
        let rfi = Rfi::parse(&raw).unwrap();
        let mut data = ImageData::copy_of(&rfi).unwrap();
        assert!(data.convert(0x0100_E608).unwrap());
        assert_eq!(data.header.format_word, 0x0100_E608);
        assert_eq!(data.header.data_size, 8);
        assert_eq!(data.pixels, [10, 20, 30, 255, 40, 50, 60, 255]);
        // A requested word with bit 20 and the image's layout changes the
        // word only.
        assert!(data.convert(0x0111_E608).unwrap());
        assert_eq!(data.header.format_word, 0x0111_E608);
        assert_eq!(data.pixels, [10, 20, 30, 255, 40, 50, 60, 255]);
        // No converter for this pair: nothing changes.
        let mut other = ImageData::copy_of(&rfi).unwrap();
        assert!(!other.convert(0x0104_4040).unwrap());
        assert_eq!(other.header.format_word, 0x0100_C600);
        assert_eq!(other.pixels, [10, 20, 30, 40, 50, 60]);
        // A zero request takes the fallback: grey to RGB.
        let grey = image(2, 1, 0x0100_4200, 0x1000_0000, 2, &[7, 9]);
        let mut grey = ImageData::copy_of(&Rfi::parse(&grey).unwrap()).unwrap();
        assert!(grey.convert(0).unwrap());
        assert_eq!(grey.header.format_word, 0x0100_C600);
        assert_eq!(grey.pixels, [7, 7, 7, 9, 9, 9]);
        // A block-compressed word has no converter here.
        let bc1 = image(4, 4, 0x813B_C600, 0x1000_0000, 8, &[0; 8]);
        let mut bc1 = ImageData::copy_of(&Rfi::parse(&bc1).unwrap()).unwrap();
        assert!(!bc1.convert(0x0100_E608).unwrap());
    }

    #[test]
    fn a_chained_image_converts_every_level() {
        // 2×2 RGB8 with its 1×1 level: five pixels.
        let body: Vec<u8> = (0..15).collect();
        let mut raw = image(2, 2, 0x0100_C600, 0x1000_0000, 15, &body);
        raw[20..24].copy_from_slice(&FLAG_MIP_CHAIN.to_le_bytes());
        let mut data = ImageData::copy_of(&Rfi::parse(&raw).unwrap()).unwrap();
        assert!(data.convert(0x0100_E608).unwrap());
        assert_eq!(data.header.data_size, 20);
        assert!(data.header.has_mip_chain());
        assert_eq!(&data.pixels[16..20], &[12, 13, 14, 255]);
    }

    /// A shipped environment cube: six 256² RGB8 faces, each nine levels,
    /// with a per-face `data_size` of one chain. Skipped without the game.
    #[test]
    fn shipped_environment_cube() {
        let Some(install) = exa_rpk::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_rpk::GameData::open(install) else {
            return;
        };
        let Some(pack) = game.pack(exa_rpk::PackKind::Textures) else {
            return;
        };
        let Some(bytes) = pack.get("genref01r") else {
            return;
        };
        let rfi = Rfi::parse(bytes).unwrap();
        assert!(rfi.header.is_cube());
        assert_eq!(rfi.header.faces(), 6);
        assert_eq!(rfi.mip_count, 9);
        assert_eq!(rfi.header.data_size, 262_143);
        assert_eq!(rfi.header.image_size(), Some(262_143));
        let faces = rfi.decode_faces_rgba8().unwrap();
        assert_eq!(faces.len(), 6);
        assert_eq!(faces[0].len(), 256 * 256 * 4);
    }
}
