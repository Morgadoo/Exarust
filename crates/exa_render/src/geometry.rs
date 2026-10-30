//! The original's planar vertex layouts and index batches.
//!
//! A geometry's format word picks the attribute streams: position always,
//! then (when their bits are set) texture coordinates, colour, normal,
//! tangent, morph delta and a second texture stream. Streams are planar: each
//! one holds every vertex's values back to back, and the streams follow each
//! other in the buffer in that order. The GL attribute pointers use the
//! original's locations: 0 position, 1 texture, 2 normal, 3 colour, 4 second
//! texture, 6 tangent and 7 morph (location 5 is unused), all tightly packed
//! (stride 0) and none normalised.
//!
//! Two quirks of the original are kept, because both decisions read the same
//! format word differently. The buffer reserves a texture stream of three
//! floats only when the `0x80` and `0x20` bits are both set (two floats when
//! just `0x10` is), while the attribute pointer for location 1 exists when
//! `0x10` is set and takes three floats when `0x80` is also set. Real formats
//! set all the bits together or none.
//!
//! Bevy's mesh allocator owns the buffers and layouts in Exarust, so this
//! module keeps the original's flags, sizes and offsets as tested functions.
//! Nothing in the running renderer calls it yet. Rules: `docs/RENDERING.md`
//! (Still capture and vertex layouts).

/// The format-word bits.
pub mod flags {
    /// The texture coordinate stream (two floats, three with [`WIDE`]).
    pub const TEXCOORD: u32 = 0x10;
    /// Widens the texture stream to three floats.
    pub const WIDE: u32 = 0x80;
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

/// One attribute stream's place in the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexAttribute {
    /// The shader's attribute location.
    pub location: u32,
    /// The byte offset of the stream's first value in the buffer.
    pub offset: usize,
    /// Floats or bytes per vertex the attribute pointer reads.
    pub components: u32,
    pub kind: AttributeType,
}

/// A format word's planar layout for a number of vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexLayout {
    /// Bytes all streams take per vertex (what the buffer size multiplies).
    pub bytes_per_vertex: u32,
    /// The buffer's size in bytes.
    pub total_bytes: usize,
    /// The attributes the original enables, in location order of the draw
    /// setup: position, texture, normal, colour, second texture, tangent,
    /// morph.
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

/// The bytes per vertex the buffer reserves for the texture stream: twelve
/// when both `0x80` and `0x20` are set, eight when `0x10` is set otherwise,
/// none else.
pub fn texture_stream_size(format: u32) -> u32 {
    if format & (flags::WIDE | flags::TEXCOORD2) == (flags::WIDE | flags::TEXCOORD2) {
        12
    } else if format & flags::TEXCOORD != 0 {
        8
    } else {
        0
    }
}

/// The vertex layout a format word asks for, for `vertices` vertices.
/// Streams are reserved in the order position (12 bytes), texture, colour
/// (4), normal (12), tangent (16), morph (12), second texture (8).
pub fn layout(format: u32, vertices: usize) -> VertexLayout {
    let mut streams = [(0usize, 0usize); 7];
    let mut offset = 0usize;
    let mut bytes = 0u32;
    let mut reserve = |slot: usize, per_vertex: u32, streams: &mut [(usize, usize); 7]| {
        streams[slot] = (offset, per_vertex as usize);
        offset = offset.saturating_add(per_vertex as usize * vertices);
        bytes += per_vertex;
    };
    reserve(0, 12, &mut streams);
    reserve(1, texture_stream_size(format), &mut streams);
    reserve(
        2,
        if format & flags::COLOUR != 0 { 4 } else { 0 },
        &mut streams,
    );
    reserve(
        3,
        if format & flags::NORMAL != 0 { 12 } else { 0 },
        &mut streams,
    );
    reserve(
        4,
        if format & flags::TANGENT != 0 { 16 } else { 0 },
        &mut streams,
    );
    reserve(
        5,
        if format & flags::MORPH != 0 { 12 } else { 0 },
        &mut streams,
    );
    reserve(
        6,
        if format & flags::TEXCOORD2 != 0 { 8 } else { 0 },
        &mut streams,
    );

    let attribute = |slot: usize, location, components, kind| VertexAttribute {
        location,
        offset: streams[slot].0,
        components,
        kind,
    };
    let mut attributes = vec![attribute(0, 0, 3, AttributeType::Float)];
    if format & flags::TEXCOORD != 0 {
        let components = if format & flags::WIDE != 0 { 3 } else { 2 };
        attributes.push(attribute(1, 1, components, AttributeType::Float));
    }
    if format & flags::NORMAL != 0 {
        attributes.push(attribute(3, 2, 3, AttributeType::Float));
    }
    if format & flags::COLOUR != 0 {
        attributes.push(attribute(2, 3, 4, AttributeType::UnsignedByte));
    }
    if format & flags::TEXCOORD2 != 0 {
        attributes.push(attribute(6, 4, 2, AttributeType::Float));
    }
    if format & flags::TANGENT != 0 {
        attributes.push(attribute(4, 6, 4, AttributeType::Float));
    }
    if format & flags::MORPH != 0 {
        attributes.push(attribute(5, 7, 3, AttributeType::Float));
    }
    VertexLayout {
        bytes_per_vertex: bytes,
        total_bytes: offset,
        attributes,
    }
}

/// An interleaved vertex stream: one vertex's attributes back to back in
/// the order the original's GPU buffer packs them, and the buffer's stride.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterleavedLayout {
    pub stride: u32,
    pub attributes: Vec<VertexAttribute>,
}

impl InterleavedLayout {
    /// The attribute at `location`.
    pub fn attribute(&self, location: u32) -> Option<&VertexAttribute> {
        self.attributes
            .iter()
            .find(|attribute| attribute.location == location)
    }
}

/// The interleaved layout a format word asks for (`0x06AE60`): position
/// (12 bytes) first, then the texture (12 when both `0x80` and `0x20` are
/// set, 8 when `0x10` is set otherwise), the colour (4), the normal (12),
/// the tangent (16), the morph (12), and the second texture (8), each the
/// previous stream's start plus its per-vertex size. The attribute locations
/// are [`layout`]'s.
pub fn interleaved_layout(format: u32) -> InterleavedLayout {
    let mut offset = 0u32;
    let mut attributes = Vec::new();
    let mut push = |location, components, kind, bytes: u32| {
        attributes.push(VertexAttribute {
            location,
            offset: offset as usize,
            components,
            kind,
        });
        offset += bytes;
    };
    push(0, 3, AttributeType::Float, 12);
    let texture = texture_stream_size(format);
    if texture != 0 {
        push(1, texture / 4, AttributeType::Float, texture);
    }
    if format & flags::COLOUR != 0 {
        push(3, 4, AttributeType::UnsignedByte, 4);
    }
    if format & flags::NORMAL != 0 {
        push(2, 3, AttributeType::Float, 12);
    }
    if format & flags::TANGENT != 0 {
        push(6, 4, AttributeType::Float, 16);
    }
    if format & flags::MORPH != 0 {
        push(7, 3, AttributeType::Float, 12);
    }
    if format & flags::TEXCOORD2 != 0 {
        push(4, 2, AttributeType::Float, 8);
    }
    InterleavedLayout {
        stride: offset,
        attributes,
    }
}

/// An index batch's element size from the original's encoded size: 1 is an
/// unsigned byte, 2 an unsigned short, anything else (0 included) an
/// unsigned int.
pub fn index_size(encoded: u32) -> usize {
    match encoded {
        1 => 1,
        2 => 2,
        _ => 4,
    }
}

/// The GL type constant the draw passes for an encoded index size.
pub fn index_type(encoded: u32) -> u32 {
    match encoded {
        1 => 0x1401,
        2 => 0x1403,
        _ => 0x1405,
    }
}

/// The bytes of a CPU index storage's header: a mode word that is zero for
/// CPU storage, the encoded index size and the batch count.
pub const CPU_HEADER_BYTES: usize = 12;
/// The bytes of one batch's header in CPU storage: its primitive mode and
/// index count. Its indices follow immediately.
pub const BATCH_HEADER_BYTES: usize = 8;

/// One index batch in a CPU storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexBatch {
    pub mode: u32,
    pub count: u32,
    /// The byte offset of the batch's header (mode and count) in the
    /// storage.
    pub header_offset: usize,
    /// The byte offset of its first index (eight bytes on).
    pub index_offset: usize,
}

/// A CPU index storage: batches stored one after another, each a mode and
/// count followed by its own indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBatches {
    pub index_size: usize,
    pub batches: Vec<IndexBatch>,
    /// The storage's allocation size: the header, then 8 bytes and the
    /// indices of every batch.
    pub total_bytes: usize,
}

/// Lays out `batches` (primitive mode, index count) the way the original
/// appends them into a CPU storage with `encoded_size`.
pub fn index_batches(batches: &[(u32, u32)], encoded_size: u32) -> IndexBatches {
    let size = index_size(encoded_size);
    let mut offset = CPU_HEADER_BYTES;
    let mut out = Vec::with_capacity(batches.len());
    for &(mode, count) in batches {
        out.push(IndexBatch {
            mode,
            count,
            header_offset: offset,
            index_offset: offset + BATCH_HEADER_BYTES,
        });
        offset += BATCH_HEADER_BYTES + count as usize * size;
    }
    IndexBatches {
        index_size: size,
        batches: out,
        total_bytes: offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_position_only_vertex_is_twelve_bytes() {
        let l = layout(0, 10);
        assert_eq!(l.bytes_per_vertex, 12);
        assert_eq!(l.total_bytes, 120);
        assert_eq!(l.attributes.len(), 1);
        assert_eq!(l.attribute(0).unwrap().components, 3);
    }

    #[test]
    fn streams_are_planar_in_the_originals_order_and_locations() {
        // Ten vertices: position 12, texture 8, colour 4, normal 12,
        // tangent 16, morph 12, second texture 8 bytes each.
        let format = flags::TEXCOORD
            | flags::COLOUR
            | flags::NORMAL
            | flags::TANGENT
            | flags::MORPH
            | flags::TEXCOORD2;
        let l = layout(format, 10);
        assert_eq!(l.bytes_per_vertex, 72);
        assert_eq!(l.total_bytes, 720);
        let places: Vec<_> = l
            .attributes
            .iter()
            .map(|a| (a.location, a.offset, a.components, a.kind))
            .collect();
        assert_eq!(
            places,
            [
                (0, 0, 3, AttributeType::Float),
                (1, 120, 2, AttributeType::Float),
                (2, 120 + 80 + 40, 3, AttributeType::Float),
                (3, 120 + 80, 4, AttributeType::UnsignedByte),
                (4, 120 + 80 + 40 + 120 + 160 + 120, 2, AttributeType::Float),
                (6, 120 + 80 + 40 + 120, 4, AttributeType::Float),
                (7, 120 + 80 + 40 + 120 + 160, 3, AttributeType::Float),
            ]
        );
    }

    #[test]
    fn the_wide_texture_quirk_follows_the_two_decisions() {
        // All three bits: the reserved stream and the pointer agree.
        let all = layout(flags::TEXCOORD | flags::WIDE | flags::TEXCOORD2, 1);
        assert_eq!(all.bytes_per_vertex, 12 + 12 + 8);
        assert_eq!(all.attribute(1).unwrap().components, 3);
        assert_eq!(all.attribute(4).unwrap().offset, 24);
        // The wide and second-texture bits without the texture bit reserve
        // the wide stream but set no pointer for it.
        let hidden = layout(flags::WIDE | flags::TEXCOORD2, 1);
        assert_eq!(hidden.bytes_per_vertex, 12 + 12 + 8);
        assert!(hidden.attribute(1).is_none());
        // The texture and wide bits alone reserve two floats but read three.
        let short = layout(flags::TEXCOORD | flags::WIDE, 1);
        assert_eq!(short.bytes_per_vertex, 12 + 8);
        assert_eq!(short.attribute(1).unwrap().components, 3);
        // The wide bit alone reserves nothing.
        assert_eq!(layout(flags::WIDE, 1).bytes_per_vertex, 12);
    }

    #[test]
    fn index_sizes_and_types_follow_the_encoding() {
        assert_eq!((index_size(0), index_type(0)), (4, 0x1405));
        assert_eq!((index_size(1), index_type(1)), (1, 0x1401));
        assert_eq!((index_size(2), index_type(2)), (2, 0x1403));
        assert_eq!((index_size(99), index_type(99)), (4, 0x1405));
    }

    #[test]
    fn the_interleaved_layout_packs_the_streams_in_the_original_order() {
        // Everything: 12 position + 12 wide texture + 4 colour + 12 normal
        // + 16 tangent + 12 morph + 8 second texture = 76.
        let format = flags::TEXCOORD
            | flags::WIDE
            | flags::TEXCOORD2
            | flags::COLOUR
            | flags::NORMAL
            | flags::TANGENT
            | flags::MORPH;
        let l = interleaved_layout(format);
        assert_eq!(l.stride, 76);
        let at = |location| {
            let a = l.attribute(location).copied().unwrap();
            (a.offset, a.components)
        };
        assert_eq!(at(0), (0, 3));
        assert_eq!(at(1), (12, 3));
        assert_eq!(at(3), (24, 4));
        assert_eq!(at(2), (28, 3));
        assert_eq!(at(6), (40, 4));
        assert_eq!(at(7), (56, 3));
        assert_eq!(at(4), (68, 2));
        // The plain format is position only; a texture without the wide bit
        // is two floats.
        let l = interleaved_layout(0);
        assert_eq!(l.stride, 12);
        assert_eq!(l.attributes.len(), 1);
        let l = interleaved_layout(flags::TEXCOORD | flags::COLOUR);
        assert_eq!(l.stride, 24);
        assert_eq!(at_of(&l, 1), (12, 2));
        assert_eq!(at_of(&l, 3), (20, 4));
        // No stream, no attribute.
        assert!(interleaved_layout(flags::NORMAL).attribute(1).is_none());
    }

    fn at_of(l: &InterleavedLayout, location: u32) -> (usize, u32) {
        let a = l.attribute(location).copied().unwrap();
        (a.offset, a.components)
    }

    #[test]
    fn cpu_batches_interleave_headers_and_indices() {
        // A 12-byte header; a batch of three u16 indices (8 + 6 bytes), then
        // one of one index (8 + 2).
        let cpu = index_batches(&[(4, 3), (5, 1)], 2);
        assert_eq!(cpu.index_size, 2);
        assert_eq!(cpu.batches[0].header_offset, 12);
        assert_eq!(cpu.batches[0].index_offset, 20);
        assert_eq!(cpu.batches[1].header_offset, 12 + 8 + 6);
        assert_eq!(cpu.batches[1].index_offset, 12 + 8 + 6 + 8);
        assert_eq!(cpu.total_bytes, 12 + 14 + 10);
        assert_eq!(index_batches(&[], 1).total_bytes, 12);
    }
}
