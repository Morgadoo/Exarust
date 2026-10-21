//! The original's planar vertex layouts and index batches.
//!
//! A geometry's format word picks the attribute streams: position always,
//! texture coordinates (two floats, or three when the `0x80` bit is also
//! set), normal, colour, a second texture coordinate, a tangent vec4 and a
//! morph delta vec3. Streams are planar, laid out in the buffer in the
//! original's allocation order: position, texture, colour, normal, tangent,
//! morph, second texture. The GL attribute pointers use the original's
//! locations: 0 position, 1 texture, 2 normal, 3 colour, 4 second texture,
//! 6 tangent and 7 morph (location 5 is unused).
//!
//! Bevy's mesh allocator owns the buffers and layouts in Exarust, so this
//! module keeps the original's flags, stride and offsets as tested
//! functions; `exa_assets::model` builds the interleaved-by-attribute Bevy
//! meshes. Rules: `docs/RENDERING.md` (Pass order).

/// The format-word bits.
pub mod flags {
    /// A texture coordinate stream of two floats (three with `TEXCOORD3`).
    pub const TEXCOORD: u32 = 0x10;
    /// The texture coordinate stream has three floats.
    pub const TEXCOORD3: u32 = 0x80;
    /// A second two-float texture coordinate stream.
    pub const TEXCOORD2: u32 = 0x20;
    /// A four-byte colour stream.
    pub const COLOUR: u32 = 0x04;
    /// A vec4 stream (the tangent).
    pub const TANGENT: u32 = 0x08;
    /// A three-float normal stream.
    pub const NORMAL: u32 = 0x02;
    /// A three-float morph-delta stream.
    pub const MORPH: u32 = 0x40;
}

/// The GL type of a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Float,
    UnsignedByte,
}

/// One attribute stream's place in the vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexAttribute {
    /// The shader's attribute location.
    pub location: u32,
    /// The stream's byte offset in the vertex.
    pub offset: u32,
    /// Floats or bytes per vertex.
    pub components: u32,
    pub kind: AttributeType,
    /// Colour bytes are normalised to 0..1.
    pub normalized: bool,
}

/// A format word's planar layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexLayout {
    /// Bytes per vertex.
    pub stride: u32,
    pub attributes: Vec<VertexAttribute>,
}

impl VertexLayout {
    /// The attribute at `location`.
    pub fn attribute(&self, location: u32) -> Option<&VertexAttribute> {
        self.attributes
            .iter()
            .find(|attribute| attribute.location == location)
    }
}

/// The texture stream's size: three floats when both the `0x10` and `0x80`
/// bits are set, two when only `0x10`, none otherwise.
pub fn texcoord_size(format: u32) -> u32 {
    if format & flags::TEXCOORD != 0 {
        if format & flags::TEXCOORD3 != 0 {
            12
        } else {
            8
        }
    } else {
        0
    }
}

/// The vertex layout a format word asks for. The stride sums position (12),
/// texture, colour (4), normal (12), tangent (16), morph (12) and second
/// texture (8) in the original's allocation order; the offsets follow the
/// same order.
pub fn layout(format: u32) -> VertexLayout {
    let mut attributes = Vec::new();
    let mut offset = 0u32;
    let push = |attributes: &mut Vec<VertexAttribute>,
                offset: u32,
                location,
                components,
                kind,
                normalized| {
        attributes.push(VertexAttribute {
            location,
            offset,
            components,
            kind,
            normalized,
        });
    };
    push(&mut attributes, offset, 0, 3, AttributeType::Float, false);
    offset += 12;
    let tex = texcoord_size(format);
    if tex != 0 {
        push(
            &mut attributes,
            offset,
            1,
            tex / 4,
            AttributeType::Float,
            false,
        );
        offset += tex;
    }
    if format & flags::COLOUR != 0 {
        push(
            &mut attributes,
            offset,
            3,
            4,
            AttributeType::UnsignedByte,
            true,
        );
        offset += 4;
    }
    if format & flags::NORMAL != 0 {
        push(&mut attributes, offset, 2, 3, AttributeType::Float, false);
        offset += 12;
    }
    if format & flags::TANGENT != 0 {
        push(&mut attributes, offset, 6, 4, AttributeType::Float, false);
        offset += 16;
    }
    if format & flags::MORPH != 0 {
        push(&mut attributes, offset, 7, 3, AttributeType::Float, false);
        offset += 12;
    }
    if format & flags::TEXCOORD2 != 0 {
        push(&mut attributes, offset, 4, 2, AttributeType::Float, false);
        offset += 8;
    }
    VertexLayout {
        stride: offset,
        attributes,
    }
}

/// The bytes a vertex buffer of `vertices` vertices needs.
pub fn vertex_bytes(format: u32, vertices: usize) -> Option<usize> {
    usize::try_from(layout(format).stride)
        .ok()?
        .checked_mul(vertices)
}

/// An index batch's element size from the original's 1/2/any encoding: 1
/// is an unsigned byte, 2 an unsigned short, anything else an unsigned
/// int.
pub fn index_size(encoded: u32) -> usize {
    match encoded {
        1 => 1,
        2 => 2,
        _ => 4,
    }
}

/// One index batch: its primitive mode, its index count and where its
/// indices start in the batch buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexBatch {
    pub mode: u32,
    pub count: u32,
    /// The byte offset of the first index.
    pub offset: usize,
}

/// The index storage's header and batch records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBatches {
    pub index_size: usize,
    /// The batches in the order they draw.
    pub batches: Vec<IndexBatch>,
    /// The bytes of all batches' indices (`count · index_size` each).
    pub data_bytes: usize,
}

/// Builds the batch list the original appends into a storage of
/// `batch_capacity` records. Every batch record is its mode and count
/// followed by its indices; a CPU storage starts with a 12-byte header, a
/// GPU one with a 20-byte header and 16-byte records.
pub fn index_batches(
    batches: &[(u32, u32)],
    encoded_size: u32,
    gpu: bool,
    batch_capacity: usize,
) -> IndexBatches {
    let size = index_size(encoded_size);
    let header = if gpu { 0x14 } else { 0xc };
    let record = if gpu { 0x10 } else { 0x8 };
    let mut offset = header + batch_capacity * record;
    let mut out = Vec::with_capacity(batches.len());
    for &(mode, count) in batches {
        out.push(IndexBatch {
            mode,
            count,
            offset,
        });
        offset += count as usize * size;
    }
    IndexBatches {
        index_size: size,
        batches: out,
        data_bytes: offset - (header + batch_capacity * record),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_position_only_vertex_is_twelve_bytes() {
        let l = layout(0);
        assert_eq!(l.stride, 12);
        assert_eq!(l.attributes.len(), 1);
        assert_eq!(l.attribute(0).unwrap().components, 3);
    }

    #[test]
    fn streams_take_the_originals_order_and_locations() {
        // Position, two texcoords, colour, normal, tangent, morph and a
        // second texcoord: 12 + 8 + 4 + 12 + 16 + 12 + 8 = 72.
        let format = flags::TEXCOORD
            | flags::COLOUR
            | flags::NORMAL
            | flags::TANGENT
            | flags::MORPH
            | flags::TEXCOORD2;
        let l = layout(format);
        assert_eq!(l.stride, 72);
        let places: Vec<_> = l
            .attributes
            .iter()
            .map(|a| (a.location, a.offset, a.components, a.kind))
            .collect();
        assert_eq!(
            places,
            [
                (0, 0, 3, AttributeType::Float),
                (1, 12, 2, AttributeType::Float),
                (3, 20, 4, AttributeType::UnsignedByte),
                (2, 24, 3, AttributeType::Float),
                (6, 36, 4, AttributeType::Float),
                (7, 52, 3, AttributeType::Float),
                (4, 64, 2, AttributeType::Float),
            ]
        );
        assert!(l.attribute(3).unwrap().normalized);
        // The `0x80` bit widens the texture stream to three floats.
        let wide = layout(flags::TEXCOORD | flags::TEXCOORD3);
        assert_eq!(wide.stride, 24);
        assert_eq!(wide.attribute(1).unwrap().components, 3);
        // The bit alone (without 0x10) adds no stream.
        assert_eq!(layout(flags::TEXCOORD3).stride, 12);
    }

    #[test]
    fn the_vertex_size_multiplies_the_stride() {
        assert_eq!(vertex_bytes(0, 10), Some(120));
        assert_eq!(vertex_bytes(flags::NORMAL | flags::COLOUR, 2), Some(56));
        assert_eq!(vertex_bytes(0, usize::MAX), None);
    }

    #[test]
    fn index_batches_place_their_data_after_the_records() {
        // CPU: a 12-byte header and two 8-byte records, three u16 indices
        // then one u32 index.
        let cpu = index_batches(&[(4, 3), (4, 1)], 2, false, 2);
        assert_eq!(cpu.index_size, 2);
        assert_eq!(cpu.batches[0].offset, 12 + 2 * 8);
        assert_eq!(cpu.batches[1].offset, 12 + 2 * 8 + 3 * 2);
        assert_eq!(cpu.data_bytes, 3 * 2 + 2);
        // GPU records are 16 bytes after a 20-byte header.
        let gpu = index_batches(&[(4, 3)], 1, true, 4);
        assert_eq!(gpu.batches[0].offset, 0x14 + 4 * 0x10);
        assert_eq!(gpu.data_bytes, 3);
        assert_eq!(index_size(99), 4);
    }
}
