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
/// 256² RGB8 chains in one file.
pub const FLAG_CUBE: u32 = 0x4000_0000;
/// The mask over the cube bits of the header flags (bits 30–31).
pub const FLAG_CUBE_MASK: u32 = 0x7000_0000;

/// The number of bytes one level of an image whose stored format word is
/// `word` takes, as the original computes it from the word's fields:
///
/// - bit 31: block compression. The block's bytes are 1 + bits 19–23
///   (`0x8` for BC1/BC4, `0x10` for BC3/BC5); the block side is 1 + bits
///   16–18 (4), and a level is whole blocks across and down.
/// - bit 18: bit-packed (uncompressed). The bits per pixel are bits 6–12
///   plus bits 0–5 (24 for RGB8), and a level is `ceil(w·h·bits / 8)`.
/// - otherwise one byte-per-channel pixel: `w·h·ceil(bits / 8)`.
///
/// A word whose cube bits (30–31) read `01` multiplies the answer by six.
/// The shipped cubes carry that bit in the header flags instead, so this
/// branch is unreachable from the data but kept as the original has it.
pub fn format_level_size(word: u32, w: u32, h: u32) -> Option<usize> {
    let bits = ((word >> 6) & 0x7F) + (word & 0x3F);
    let mut size = if word & 0x8000_0000 != 0 {
        let block = (((word & 0x00F8_0000) >> 19) + 1) as usize;
        let side = (((word & 0x0007_0000) >> 16) + 1) as u64;
        let across = u64::from(w).div_ceil(side);
        let down = u64::from(h).div_ceil(side);
        (block as u64).checked_mul(across)?.checked_mul(down)? as usize
    } else if word & 0x0004_0000 != 0 {
        let bits = u64::from(bits);
        let total = bits.checked_mul(u64::from(w))?.checked_mul(u64::from(h))?;
        ((total + 7) >> 3) as usize
    } else {
        let bytes = (bits as u64).div_ceil(8);
        bytes.checked_mul(u64::from(w))?.checked_mul(u64::from(h))? as usize
    };
    if word & FLAG_CUBE_MASK == FLAG_CUBE {
        size = size.checked_mul(6)?;
    }
    Some(size)
}

/// The pixel count of an image's full mip chain (base level first), as the
/// original's data-size helper counts it: the base `w·h` plus every halved
/// level while `w + h > 2`, dimensions never dropping below one.
pub fn mip_chain_pixels(mut w: u32, mut h: u32) -> u64 {
    let mut pixels = u64::from(w) * u64::from(h);
    while u64::from(w) + u64::from(h) > 2 {
        if w > 1 {
            w >>= 1;
        }
        if h > 1 {
            h >>= 1;
        }
        pixels += u64::from(w) * u64::from(h);
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
    /// fields ([`format_level_size`]); a cube's six faces are not included.
    /// A hand-built header without a format word falls back on the decoded
    /// [`PixelFormat`].
    pub fn level_size(&self, w: u32, h: u32) -> Option<usize> {
        format_level_size(self.format_word, w, h)
            .filter(|&size| size > 0)
            .or_else(|| PixelFormat::from_word(self.format_word).level_size(w, h))
    }

    /// Bytes of the base level of one face.
    pub fn face_base_size(&self) -> Option<usize> {
        self.level_size(self.width, self.height)
    }

    /// Bytes of every stored level of one face.
    pub fn face_chain_size(&self) -> Option<usize> {
        let (mut w, mut h) = (self.width, self.height);
        let mut total = self.level_size(w, h)?;
        if self.has_mip_chain() {
            while w + h > 2 {
                if w > 1 {
                    w >>= 1;
                }
                if h > 1 {
                    h >>= 1;
                }
                total = total.checked_add(self.level_size(w, h)?)?;
            }
        }
        Some(total)
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

    /// The full pixel stream as stored (or unpacked), all faces and levels.
    pub fn pixels(&self) -> Result<Cow<'a, [u8]>> {
        if !self.is_packed() {
            return Ok(Cow::Borrowed(self.data));
        }
        let bpp = self
            .format
            .bytes_per_pixel()
            .ok_or_else(|| Error::Unsupported(format!("packed {:?}", self.format)))?;
        let len = self.header.data_size as usize * self.header.faces() as usize;
        unpack(self.data, bpp, len).map(Cow::Owned)
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
    /// six faces back to back, each its own chain.
    pub fn face_chain(&self, face: u32) -> Result<Cow<'a, [u8]>> {
        let face_len = self.face_chain_len()?;
        let start = face as usize * face_len;
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
            PixelFormat::Bc1 | PixelFormat::Bc3 | PixelFormat::Bc4 | PixelFormat::Bc5 => {
                decode_blocks(self.format, src, w, h, out)
            }
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
            PixelFormat::Rgb8 | PixelFormat::L8 | PixelFormat::La8 | PixelFormat::A8 => {
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
/// keeps a cube's per-face `data_size`; `pixels` holds all faces.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageData {
    pub header: RfiHeader,
    pub format: PixelFormat,
    pub pixels: Vec<u8>,
}

impl ImageData {
    /// A copy of an image's pixel stream, unpacking it first when it is
    /// run-length encoded (the original logs "Source image is run length
    /// encoded." and decodes). The whole stream is copied: every face and
    /// every stored mip level.
    pub fn copy_of(image: &Rfi<'_>) -> Result<Self> {
        Ok(Self {
            header: image.header,
            format: image.format,
            pixels: image.pixels()?.into_owned(),
        })
    }

    /// A copy of an image's base level only: the original's copy with its
    /// chain flag cleared (`FUN_100046770` with the flag off sizes the base
    /// level).
    pub fn copy_base(image: &Rfi<'_>) -> Result<Self> {
        let face_len = image.face_chain_len()?;
        let base = image
            .header
            .level_size(image.header.width, image.header.height)
            .or_else(|| {
                image
                    .format
                    .level_size(image.header.width, image.header.height)
            })
            .ok_or_else(|| Error::Unsupported(format!("{:?}", image.format)))?;
        let source = image.pixels()?;
        let mut pixels = Vec::with_capacity(base * image.header.faces() as usize);
        for face in 0..image.header.faces() as usize {
            let start = face * face_len;
            let slice = source.get(start..start + base).ok_or(Error::Truncated {
                offset: HEADER_LEN + start,
                needed: base,
                len: HEADER_LEN + source.len(),
            })?;
            pixels.extend_from_slice(slice);
        }
        let mut header = image.header;
        header.flags &= !FLAG_MIP_CHAIN;
        header.data_size = base as u32;
        Ok(Self {
            header,
            format: image.format,
            pixels,
        })
    }

    /// A blank image of `width` × `height` in `format_word`, as the
    /// original's create call does: the format's own size, no mip chain.
    pub fn blank(width: u16, height: u16, format_word: u32, flags: u32) -> Result<Self> {
        let format = PixelFormat::from_word(format_word);
        let size = format_level_size(format_word, u32::from(width), u32::from(height))
            .ok_or_else(|| Error::Unsupported(format!("rfi format word {format_word:#010x}")))?;
        Ok(Self {
            header: RfiHeader {
                width: u32::from(width),
                height: u32::from(height),
                depth: 1,
                format_word,
                flags,
                storage: 0x1000_0000,
                data_size: size as u32,
            },
            format,
            pixels: vec![0; size],
        })
    }

    /// The blank image of the original's icon target: a mip chain flag and
    /// detail class 2, with room for the full chain.
    pub fn render_target(width: u16, height: u16, format_word: u32, flags: u32) -> Result<Self> {
        let mut data = Self::blank(width, height, format_word, flags)?;
        let flags = if data.header.flags & 0x00C0_0000 == 0 {
            data.header.flags | 0x0080_0000
        } else {
            data.header.flags
        };
        data.header.flags = flags | FLAG_MIP_CHAIN;
        let chain = {
            let (mut w, mut h) = (data.header.width, data.header.height);
            let mut total = format_level_size(format_word, w, h).unwrap_or(0);
            while w + h > 2 {
                if w > 1 {
                    w >>= 1;
                }
                if h > 1 {
                    h >>= 1;
                }
                total += format_level_size(format_word, w, h).unwrap_or(0);
            }
            total
        };
        data.header.data_size = (chain / data.header.faces() as usize) as u32;
        data.pixels = vec![0; chain];
        Ok(data)
    }

    /// Resizes the pixel store (the original's `FUN_100046c70`): the header
    /// keeps the requested size, times six for a cube. Existing bytes stay,
    /// new ones are zero; shrinking drops the tail.
    pub fn resize(&mut self, size: usize) -> Result<()> {
        self.header.data_size =
            u32::try_from(size).map_err(|_| Error::Unsupported(format!("image size {size}")))?;
        let len = size
            .checked_mul(self.header.faces() as usize)
            .ok_or_else(|| Error::Unsupported(format!("image size {size}")))?;
        self.pixels.resize(len, 0);
        Ok(())
    }

    /// Frees the pixel store (the original's `FUN_100046c00`).
    pub fn release(&mut self) {
        self.pixels = Vec::new();
        self.header.data_size = 0;
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

/// The stored format word without its cube bits: one face's levels carry no
/// face count.
fn face_word(word: u32) -> u32 {
    word & !FLAG_CUBE_MASK
}

fn mip_count_for(format: PixelFormat, word: u32, w: u32, h: u32, data_size: u32) -> u32 {
    let word = face_word(word);
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

/// The registry key of the Rayform Image reader (Exarust's; the original's
/// numeric key is not stored in any file).
pub const TAG: u32 = RFI_MAGIC;

/// The loader-registry entry of the Rayform Image reader.
pub fn format() -> crate::image_registry::ImageFormat {
    crate::image_registry::ImageFormat {
        tag: TAG,
        extension: "rfi",
        description: "Rayform Image",
        loader: crate::image_registry::load_rfi,
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
        // A cube word multiplies by six (the original's branch; the shipped
        // cubes carry the bit in the header flags instead).
        assert_eq!(format_level_size(0x4000_C600, 2, 2), Some(6 * 12));
    }

    #[test]
    fn the_mip_chain_pixel_count_is_the_base_plus_every_halving() {
        assert_eq!(mip_chain_pixels(256, 256), 87_381);
        assert_eq!(mip_chain_pixels(4, 4), 16 + 4 + 1);
        assert_eq!(mip_chain_pixels(1, 1), 1);
    }

    #[test]
    fn a_cube_stores_six_faces_each_with_its_own_chain() {
        // Six 2×2 RGB8 faces, no chain: per-face data_size 12.
        let mut body = Vec::new();
        for face in 0..6u8 {
            body.extend_from_slice(&[face, 10, 20, face, 30, 40, face, 50, 60, face, 70, 80]);
        }
        let flags = FLAG_CUBE;
        let raw = image(2, 2, 0x0100_C600, 0x1000_0000, 12, &body);
        let mut raw = raw;
        raw[20..24].copy_from_slice(&flags.to_le_bytes());
        let rfi = Rfi::parse(&raw).unwrap();
        assert!(rfi.header.is_cube());
        assert_eq!(rfi.header.faces(), 6);
        assert_eq!(rfi.header.face_chain_size(), Some(12));
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
        assert_eq!(base.pixels.len(), 12 * 6);
    }

    #[test]
    fn images_are_created_resized_and_released() {
        let mut blank = ImageData::blank(2, 2, 0x0100_C600, 0).unwrap();
        assert_eq!(blank.header.data_size, 12);
        assert_eq!(blank.pixels.len(), 12);
        blank.resize(3).unwrap();
        assert_eq!((blank.header.data_size, blank.pixels.len()), (3, 3));
        blank.release();
        assert_eq!((blank.header.data_size, blank.pixels.len()), (0, 0));
        // A cube's resize stores six faces.
        let mut cube = ImageData::blank(2, 2, 0x0100_C600, FLAG_CUBE).unwrap();
        cube.resize(12).unwrap();
        assert_eq!(cube.header.data_size, 12);
        assert_eq!(cube.pixels.len(), 72);
        // The icon target carries a chain flag and class 2.
        let target = ImageData::render_target(4, 4, 0x0100_E608, 0).unwrap();
        assert!(target.header.has_mip_chain());
        assert_eq!(target.header.detail_class(), 2);
        assert_eq!(target.header.data_size, 4 * 4 * 4 + 2 * 2 * 4 + 4);
        assert!(ImageData::blank(2, 2, 0x0100_C600, 0).is_ok());
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
        assert_eq!(rfi.header.face_chain_size(), Some(262_143));
        let faces = rfi.decode_faces_rgba8().unwrap();
        assert_eq!(faces.len(), 6);
        assert_eq!(faces[0].len(), 256 * 256 * 4);
    }
}
