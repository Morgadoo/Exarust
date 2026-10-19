//! A thaumaturgy power tree.
//!
//! The file's first word is the power magic. The node count is the word at
//! byte 52. Each node is a power id, two more words, an x, a y, a link count,
//! and that many 16-byte links. A link count below 1 stores none. Every node
//! after the first subtracts the first node's x and y. A short file or a
//! negative count is an error. A following text chunk stores each string as a
//! length word and that many bytes. A length below 1 is empty. No chunk
//! leaves the texts empty. A chunk that ends early is an error.
//!
//! A drawn node sits at ten times its place times the zoom, plus the pan, and
//! rounds to the nearest even integer unless the view is dragged. A factor
//! then scales that point about the origin. A link with both marks at zero
//! is not drawn. Otherwise each end leaves its node along the sine and cosine
//! of its turn times a full circle. The reach is 0.514, or 0.295 when the
//! power's high mark is set, minus 0.003 times the zoom, times the zoom
//! rounded to an even 256. The samples are the cubic between those ends. A
//! zero source mark dims the line. An icon uses the sheet cell of the power's
//! low byte. A frame uses a four-by-two sheet and four fifths of the size. A
//! wide picture drops the fraction of the view; a narrow one uses half. A glow
//! is 32 spokes on a ring. A learned mark is copied from the known list, and
//! a node opens when every link's end is already open. A thick joint is the
//! two edge points around the middle of three.
//!
//! Evidence: the tree reader (magic check, the seek to the count, the node
//! fields, the link bytes, and the shift against the first node), and the
//! draw that places each node and builds each link. The fast sine polynomial
//! is not reproduced: a turn of zero is straight up, which matches.

use crate::{Error, Result, read_u32};

/// First word of a power tree.
pub const MAGIC: u32 = 0xAFCE_01CE;
/// First word of the text chunk after the nodes.
pub const TEXT_MAGIC: u32 = 0xABCD_0000;
/// The node count sits here.
const COUNT_AT: usize = 52;
/// The first node starts here.
const NODES_AT: usize = 56;
/// Bytes in one link.
const LINK: usize = 16;
/// Bytes in a node before its links.
const NODE_HEAD: usize = 24;
/// Power bits that use the shorter link reach.
const SMALL_NODE: u32 = 0x0f00_0000;
/// Reach of an ordinary node, before the zoom shrinks it.
const LARGE_REACH: f64 = 0.514;
/// Reach of a node whose high mark is set.
const SMALL_REACH: f64 = 0.295;
/// How much one zoom step shortens the reach.
const REACH_SHRINK: f64 = 0.003;

/// One node, with its place already relative to the first node.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerNode {
    pub power: u32,
    pub extra: u32,
    pub flags: u32,
    pub x: f32,
    pub y: f32,
    pub links: Vec<[u8; LINK]>,
}

/// The nodes of one domain's tree, and the texts that follow them.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerTree {
    pub nodes: Vec<PowerNode>,
    pub texts: Vec<Vec<u8>>,
}

/// Read a power tree. Malformed input is an error.
pub fn read_power_tree(bytes: &[u8]) -> Result<PowerTree> {
    let magic = read_u32(bytes, 0)?;
    if magic != MAGIC {
        return Err(Error::BadMagic {
            found: magic,
            expected: MAGIC,
        });
    }
    let count = read_u32(bytes, COUNT_AT)? as i32;
    if count < 0 {
        return Err(Error::Malformed("negative node count".into()));
    }
    let count = count as usize;
    let rest = bytes.len().saturating_sub(NODES_AT);
    if count.saturating_mul(NODE_HEAD) > rest {
        return Err(Error::Truncated {
            offset: NODES_AT,
            needed: count.saturating_mul(NODE_HEAD),
            len: rest,
        });
    }
    let mut offset = NODES_AT;
    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        let power = take_u32(bytes, &mut offset)?;
        let extra = take_u32(bytes, &mut offset)?;
        let flags = take_u32(bytes, &mut offset)?;
        let x = take_f32(bytes, &mut offset)?;
        let y = take_f32(bytes, &mut offset)?;
        let links_n = take_u32(bytes, &mut offset)? as i32;
        let n = if links_n < 1 { 0 } else { links_n as usize };
        let need = n.saturating_mul(LINK);
        if need > bytes.len().saturating_sub(offset) {
            return Err(Error::Truncated {
                offset,
                needed: need,
                len: bytes.len().saturating_sub(offset),
            });
        }
        let mut links = Vec::with_capacity(n);
        for _ in 0..n {
            let mut link = [0u8; LINK];
            link.copy_from_slice(&bytes[offset..offset + LINK]);
            offset += LINK;
            links.push(link);
        }
        nodes.push(PowerNode {
            power,
            extra,
            flags,
            x,
            y,
            links,
        });
    }
    if let Some(origin) = nodes.first() {
        let (ox, oy) = (origin.x, origin.y);
        for node in nodes.iter_mut().skip(1) {
            node.x -= ox;
            node.y -= oy;
        }
    }
    let texts = read_texts(bytes, offset)?;
    Ok(PowerTree { nodes, texts })
}

/// The text chunk, when its first word matches. Anything else leaves no texts.
fn read_texts(bytes: &[u8], mut offset: usize) -> Result<Vec<Vec<u8>>> {
    if offset == bytes.len() {
        return Ok(Vec::new());
    }
    let magic = take_u32(bytes, &mut offset)?;
    if magic != TEXT_MAGIC {
        return Ok(Vec::new());
    }
    // One skipped word before the check, then two skipped words around the
    // two size words the caller reads and does not use.
    skip(bytes, &mut offset, 4)?;
    skip(bytes, &mut offset, 4)?;
    let _kept = take_u32(bytes, &mut offset)?;
    skip(bytes, &mut offset, 4)?;
    let _other = take_u32(bytes, &mut offset)?;
    if bytes.len().saturating_sub(offset) < 16 {
        return Err(Error::Truncated {
            offset,
            needed: 16,
            len: bytes.len().saturating_sub(offset),
        });
    }
    let count = i32::from_le_bytes(
        bytes[offset + 12..offset + 16]
            .try_into()
            .expect("16 bytes"),
    );
    offset += 16;
    if count < 0 {
        return Err(Error::Malformed("negative text count".into()));
    }
    let count = count as usize;
    let table = count.saturating_add(1).saturating_mul(4);
    if table > bytes.len().saturating_sub(offset) {
        return Err(Error::Truncated {
            offset,
            needed: table,
            len: bytes.len().saturating_sub(offset),
        });
    }
    let mut offsets = Vec::with_capacity(count + 1);
    for _ in 0..count + 1 {
        offsets.push(take_u32(bytes, &mut offset)?);
    }
    let size = take_u32(bytes, &mut offset)? as usize;
    if size > bytes.len().saturating_sub(offset) {
        return Err(Error::Truncated {
            offset,
            needed: size,
            len: bytes.len().saturating_sub(offset),
        });
    }
    let blob = &bytes[offset..offset + size];
    let mut texts = Vec::with_capacity(count);
    for at in offsets.iter().take(count).map(|at| *at as usize) {
        if blob.len().saturating_sub(at) < 4 {
            return Err(Error::Truncated {
                offset: at,
                needed: 4,
                len: blob.len().saturating_sub(at),
            });
        }
        let len = i32::from_le_bytes(blob[at..at + 4].try_into().expect("4 bytes"));
        if len < 1 {
            texts.push(Vec::new());
            continue;
        }
        let len = len as usize;
        let start = at + 4;
        if blob.len().saturating_sub(start) < len {
            return Err(Error::Truncated {
                offset: start,
                needed: len,
                len: blob.len().saturating_sub(start),
            });
        }
        texts.push(blob[start..start + len].to_vec());
    }
    Ok(texts)
}

fn skip(bytes: &[u8], offset: &mut usize, n: usize) -> Result<()> {
    if bytes.len().saturating_sub(*offset) < n {
        return Err(Error::Truncated {
            offset: *offset,
            needed: n,
            len: bytes.len().saturating_sub(*offset),
        });
    }
    *offset += n;
    Ok(())
}

fn take_u32(bytes: &[u8], offset: &mut usize) -> Result<u32> {
    let value = read_u32(bytes, *offset)?;
    *offset += 4;
    Ok(value)
}

fn take_f32(bytes: &[u8], offset: &mut usize) -> Result<f32> {
    let value = take_u32(bytes, offset)?;
    Ok(f32::from_le_bytes(value.to_le_bytes()))
}

/// One end of a drawn link: its placed point, its power, and its mark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkEnd {
    pub at: [f32; 2],
    pub power: u32,
    pub mark: u32,
}

/// One drawn link: dim when its source mark is zero, and the curve between the nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeLink {
    pub dim: bool,
    pub points: Vec<[f32; 2]>,
}

/// Where a node sits before the view factor. Dragging leaves the point unrounded.
pub fn node_pixels(x: f32, y: f32, zoom: f32, pan_x: f32, pan_y: f32, snap: bool) -> [f32; 2] {
    let mut point = [x * 10.0 * zoom + pan_x, y * 10.0 * zoom + pan_y];
    if snap {
        point[0] = point[0].round_ties_even();
        point[1] = point[1].round_ties_even();
    }
    point
}

/// Scale a placed node about the origin by the view factor.
pub fn place_node(point: [f32; 2], origin_x: f32, origin_y: f32, factor: f32) -> [f32; 2] {
    [
        (point[0] - origin_x) * factor + origin_x,
        (point[1] - origin_y) * factor + origin_y,
    ]
}

/// The curve of one link. Both marks at zero draw nothing.
///
/// Each end is already placed. The link's first two words are the turns, as
/// fractions of a circle.
pub fn tree_link(source: LinkEnd, dest: LinkEnd, link: &[u8; 16], zoom: f32) -> Option<TreeLink> {
    if source.mark == 0 && dest.mark == 0 {
        return None;
    }
    let scale = (zoom * 256.0).round_ties_even();
    let out_turn = direction(word_f32(link, 0));
    let in_turn = direction(word_f32(link, 4));
    let start = offset(source.at, out_turn, reach(source.power, zoom, scale));
    let end = offset(dest.at, in_turn, reach(dest.power, zoom, scale));
    let (out_dir, in_dir, steps) = tangents(start, end, out_turn, in_turn, zoom);
    let mut points = Vec::with_capacity(steps + 1);
    points.push(start);
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        points.push(hermite(start, end, out_dir, in_dir, t));
    }
    Some(TreeLink {
        dim: source.mark == 0,
        points,
    })
}

fn word_f32(link: &[u8; 16], at: usize) -> f32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&link[at..at + 4]);
    f32::from_le_bytes(bytes)
}

fn reach(power: u32, zoom: f32, scale: f32) -> f32 {
    let base = if power & SMALL_NODE == 0 {
        LARGE_REACH
    } else {
        SMALL_REACH
    };
    ((base - f64::from(zoom) * REACH_SHRINK) * f64::from(scale)) as f32
}

fn direction(turn: f32) -> [f32; 2] {
    let (sine, cosine) = (turn * std::f32::consts::TAU).sin_cos();
    [sine, cosine]
}

fn offset(center: [f32; 2], dir: [f32; 2], reach: f32) -> [f32; 2] {
    [center[0] + dir[0] * reach, center[1] + dir[1] * reach]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn scale(v: [f32; 2], by: f32) -> [f32; 2] {
    [v[0] * by, v[1] * by]
}

fn tangents(
    start: [f32; 2],
    end: [f32; 2],
    out_turn: [f32; 2],
    in_turn: [f32; 2],
    zoom: f32,
) -> ([f32; 2], [f32; 2], usize) {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let length_sq = delta[0] * delta[0] + delta[1] * delta[1];
    let length = length_sq.sqrt();
    let mut closeness = length / (zoom * 400.0);
    if closeness < 1.0 {
        closeness *= closeness;
    }
    let unit = if length_sq == 0.0 {
        [0.0, 0.0]
    } else {
        scale(delta, reciprocal_sqrt(length_sq))
    };
    let out_dot = dot(out_turn, unit);
    let mut out_scale = closeness;
    if closeness < 1.0 {
        out_scale = (1.0 - closeness) * (1.1 - out_dot) + closeness;
    }
    let in_dot = dot(in_turn, unit);
    let mut in_scale = closeness;
    if closeness < 1.0 {
        in_scale = (1.0 - closeness) * (in_dot + 1.1) + closeness;
    }
    let span = ((1.1 - out_dot) + in_dot + 1.1) * 6.0 + closeness.sqrt() * 4.0;
    let rounded = span.round_ties_even();
    let steps = if rounded.is_finite() {
        rounded as i32
    } else {
        30
    };
    let steps = steps.saturating_add(1).clamp(2, 30) as usize;
    (
        scale(out_turn, zoom * 400.0 * out_scale),
        scale(in_turn, zoom * -400.0 * in_scale),
        steps,
    )
}

fn hermite(p0: [f32; 2], p1: [f32; 2], t0: [f32; 2], t1: [f32; 2], t: f32) -> [f32; 2] {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = (t3 * 2.0 - t2 * 3.0) + 1.0;
    let h01 = t2 * 3.0 + t3 * -2.0;
    let h10 = (t3 - t2 * 2.0) + t;
    let h11 = t3 - t2;
    [
        p0[0] * h00 + p1[0] * h01 + t0[0] * h10 + t1[0] * h11,
        p0[1] * h00 + p1[1] * h01 + t0[1] * h10 + t1[1] * h11,
    ]
}

fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // The same approximate reciprocal square root the line uses for its unit.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// The square drawn for one icon. No columns means the sheet is not ready.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconSquare {
    pub origin: [f32; 2],
    pub size: f32,
    pub uv: [f32; 2],
    pub cell: [f32; 2],
    pub bias: f32,
    pub pad: f32,
}

/// The square drawn for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSquare {
    pub origin: [f32; 2],
    pub size: f32,
    pub uv: [f32; 2],
    pub cell: [f32; 2],
    pub shrink: [f32; 2],
}

/// Inner points of a glow, starting straight up and closing the circle.
#[derive(Debug, Clone, PartialEq)]
pub struct GlowRing {
    pub inner: f32,
    pub outer: f32,
    pub spokes: Vec<[f32; 2]>,
}

const ICON_BIAS: f32 = f32::from_bits(0x3f91_eb85);
const ICON_MARK: u32 = 0x0100;
const FRAME_NARROW: f32 = f32::from_bits(0x39cc_cccd);
const GLOW_SHARE: f32 = 0.84;
const GLOW_SPOKES: u32 = 32;
/// Bits 20 to 23 are ignored when a known id is matched to a node.
const KNOWN_MASK: u32 = 0xff0f_ffff;
/// A mark copied from a known record that already opened the node.
const OPENED: u32 = 0x1000;
/// The bit a node gains once it can be shown.
const SHOWN: u32 = 0x10000;
/// Cleared on a node that is not already open.
const HELD: u32 = 0x4000;
/// A power whose top nibble is 7 does not open from its links.
const UNOPENED: u32 = 0x7000_0000;
/// The thick joint's join scale, applied in double precision.
const JOIN_SCALE: f64 = -0.09;

/// A wide node drops the fraction of the view. A narrow node uses half of it.
pub fn picture_span(power: u32, view: f32) -> f32 {
    if power & SMALL_NODE == 0 {
        view.trunc()
    } else {
        view * 0.5
    }
}

/// The icon square. `columns` and `rows` are the sheet the texture names.
pub fn icon_square(
    power: u32,
    x: f32,
    y: f32,
    size: f32,
    columns: u32,
    rows: u32,
) -> Option<IconSquare> {
    if columns == 0 {
        return None;
    }
    let index = i64::from((power & 0xff) as i32 - 1);
    let columns_i = i64::from(columns);
    let column = index - (index / columns_i) * columns_i;
    let row = index / columns_i;
    let cell = [1.0 / columns as f32, 1.0 / rows as f32];
    let uv = [column as f32 * cell[0], 1.0 - (row as f32 + 1.0) * cell[1]];
    let bias = if power & 0xff00 == ICON_MARK {
        1.0
    } else {
        ICON_BIAS
    };
    Some(IconSquare {
        origin: [x - size * 0.5, y - size * 0.5],
        size,
        uv,
        cell,
        bias,
        pad: size * 0.05 + 8.0,
    })
}

/// The frame square. `slot` is the one-based picture on the four-by-two sheet.
pub fn frame_square(slot: i32, x: f32, y: f32, size: f32) -> FrameSquare {
    let index = i64::from(slot) - 1;
    let adjust = (index >> 63) & 3;
    let column = index - ((index + adjust) & !3);
    let row = (index + adjust) >> 2;
    let gap = 1.0 - size * FRAME_NARROW;
    let drawn = (f64::from(size) * 0.8) as f32;
    FrameSquare {
        origin: [x - drawn * 0.5, y - drawn * 0.5],
        size: drawn,
        uv: [column as f32 * 0.25, 1.0 - (row as f32 + 1.0) * 0.5],
        cell: [0.25, 0.5],
        shrink: [
            (f64::from(gap) * 0.005) as f32,
            (f64::from(gap) * 0.01) as f32,
        ],
    }
}

/// One joint of a thick line: the two edge points and the join factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThickJoint {
    pub left: [f32; 2],
    pub right: [f32; 2],
    pub join: f32,
}

/// The marks after known powers are copied and links are opened.
///
/// Each known pair is an id and a mark. The id matches a node when bits 20 to
/// 23 are cleared. The link's third word is a one-based end. An end that is
/// not a node is not open.
pub fn learn_marks(nodes: &[PowerNode], known: &[(u32, u32)]) -> Vec<u32> {
    if nodes.is_empty() {
        return Vec::new();
    }
    let mut marks = vec![0u32; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        for (id, mark) in known {
            if id & KNOWN_MASK == node.power {
                marks[index] = *mark;
            }
        }
    }
    marks[0] |= SHOWN;
    for index in 0..nodes.len() {
        if marks[index] & OPENED == 0 {
            marks[index] &= !HELD;
            if nodes[index].power & 0xf000_0000 != UNOPENED && links_open(&nodes[index], &marks) {
                marks[index] |= SHOWN;
            }
        } else {
            marks[index] |= SHOWN;
        }
    }
    marks
}

fn links_open(node: &PowerNode, marks: &[u32]) -> bool {
    node.links.iter().all(|link| {
        let end = u32::from_le_bytes(link[8..12].try_into().expect("four bytes")) as usize;
        end > 0 && end <= marks.len() && marks[end - 1] & OPENED != 0
    })
}

/// The two edge points around `at`, using the chord from `prev` to `next`.
///
/// A zero-length chord stays on the point.
pub fn thick_joint(prev: [f32; 2], at: [f32; 2], next: [f32; 2], width: f32) -> ThickJoint {
    let chord = [next[0] - prev[0], next[1] - prev[1]];
    let length = (chord[0] * chord[0] + chord[1] * chord[1]).sqrt();
    let offset = if length == 0.0 || width == 0.0 {
        [0.0, 0.0]
    } else {
        let scale = width / length;
        [-chord[1] * scale, chord[0] * scale]
    };
    let incoming = [at[0] - prev[0], at[1] - prev[1]];
    let outgoing = [next[0] - at[0], next[1] - at[1]];
    let diff = dot(offset, incoming) - dot(offset, outgoing);
    let join = if width == 0.0 {
        0.0
    } else {
        (f64::from(diff) * JOIN_SCALE / f64::from(width * width)) as f32
    };
    ThickJoint {
        left: [at[0] - offset[0], at[1] - offset[1]],
        right: [at[0] + offset[0], at[1] + offset[1]],
        join,
    }
}

/// Thirty-two spokes. The ring starts straight up and returns there.
pub fn glow_ring(x: f32, y: f32, across: f32, out: f32) -> GlowRing {
    let inner = across * GLOW_SHARE;
    let outer = inner + out;
    let mut spokes = Vec::with_capacity(GLOW_SPOKES as usize + 1);
    for step in 0..=GLOW_SPOKES {
        let (sine, cosine) = (step as f32 * std::f32::consts::TAU / GLOW_SPOKES as f32).sin_cos();
        spokes.push([x + sine * inner, y + cosine * inner]);
    }
    GlowRing {
        inner,
        outer,
        spokes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn put_f32(out: &mut Vec<u8>, value: f32) {
        put_u32(out, u32::from_le_bytes(value.to_le_bytes()));
    }

    fn sample() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.resize(COUNT_AT, 0);
        put_u32(&mut out, 2);
        put_u32(&mut out, 7);
        put_u32(&mut out, 1);
        put_u32(&mut out, 2);
        put_f32(&mut out, 10.0);
        put_f32(&mut out, 20.0);
        put_u32(&mut out, 0);
        put_u32(&mut out, 9);
        put_u32(&mut out, 3);
        put_u32(&mut out, 4);
        put_f32(&mut out, 15.0);
        put_f32(&mut out, 25.0);
        put_u32(&mut out, 1);
        out.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        out
    }

    #[test]
    fn a_tree_shifts_later_nodes_toward_the_first() {
        let tree = read_power_tree(&sample()).unwrap();
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.nodes[0].power, 7);
        assert_eq!(tree.nodes[0].x, 10.0);
        assert_eq!(tree.nodes[0].y, 20.0);
        assert!(tree.nodes[0].links.is_empty());
        assert!(tree.texts.is_empty());
        assert_eq!(tree.nodes[1].power, 9);
        assert_eq!(tree.nodes[1].x, 5.0);
        assert_eq!(tree.nodes[1].y, 5.0);
        assert_eq!(
            tree.nodes[1].links,
            vec![[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]]
        );

        let mut negative = sample();
        // The second node's link count is the last u32 before the 16 link bytes.
        let at = negative.len() - 16 - 4;
        negative[at..at + 4].copy_from_slice(&(-1i32).to_le_bytes());
        negative.truncate(at + 4);
        let tree = read_power_tree(&negative).unwrap();
        assert!(tree.nodes[1].links.is_empty());

        assert!(read_power_tree(&[0, 0, 0, 0]).is_err());
        assert!(read_power_tree(&MAGIC.to_le_bytes()).is_err());
    }

    #[test]
    fn a_text_chunk_keeps_its_strings() {
        let mut bytes = sample();
        put_u32(&mut bytes, TEXT_MAGIC);
        put_u32(&mut bytes, 0);
        put_u32(&mut bytes, 0);
        put_u32(&mut bytes, 0);
        put_u32(&mut bytes, 0);
        put_u32(&mut bytes, 0);
        bytes.extend_from_slice(&[0; 12]);
        put_u32(&mut bytes, 2);
        put_u32(&mut bytes, 0);
        put_u32(&mut bytes, 9);
        put_u32(&mut bytes, 13);
        put_u32(&mut bytes, 13);
        put_u32(&mut bytes, 5);
        bytes.extend_from_slice(b"Blast");
        put_u32(&mut bytes, 0);

        let tree = read_power_tree(&bytes).unwrap();
        assert_eq!(tree.nodes[1].x, 5.0);
        assert_eq!(tree.texts, [b"Blast".to_vec(), Vec::new()]);

        let mut short = sample();
        put_u32(&mut short, TEXT_MAGIC);
        assert!(read_power_tree(&short).is_err());
    }

    #[test]
    fn a_tree_line_leaves_each_node_by_its_reach() {
        assert_eq!(node_pixels(2.0, -1.0, 1.0, 0.0, 0.0, true), [20.0, -10.0]);
        assert_eq!(node_pixels(0.25, 0.0, 1.0, 0.0, 0.0, true), [2.0, 0.0]);
        assert_eq!(node_pixels(0.25, 0.0, 1.0, 0.0, 0.0, false), [2.5, 0.0]);
        assert_eq!(place_node([20.0, -10.0], 0.0, 0.0, 0.5), [10.0, -5.0]);

        let link = [0u8; 16];
        let origin = LinkEnd {
            at: [0.0, 0.0],
            power: 0,
            mark: 0,
        };
        let small = LinkEnd {
            at: [400.0, 0.0],
            power: 0x0100_0000,
            mark: 0,
        };
        assert!(tree_link(origin, small, &link, 1.0).is_none());

        let line = tree_link(origin, LinkEnd { mark: 1, ..small }, &link, 1.0)
            .expect("a marked end is drawn");
        assert!(line.dim);
        assert_eq!(line.points[0], [0.0, f32::from_bits(0x4302_d0e5)]);
        assert_eq!(
            *line.points.last().expect("the line has an end"),
            [400.0, f32::from_bits(0x4295_8106)]
        );
        assert_eq!(line.points.len(), 19);

        let bright = tree_link(
            LinkEnd { mark: 1, ..origin },
            LinkEnd { mark: 1, ..small },
            &link,
            1.0,
        )
        .expect("either mark draws the line");
        assert!(!bright.dim);
        assert_eq!(bright.points[0], line.points[0]);
        assert_eq!(bright.points.last(), line.points.last());
    }

    #[test]
    fn a_node_picture_uses_its_cell() {
        assert!(icon_square(6, 100.0, 50.0, 10.0, 0, 2).is_none());
        let icon = icon_square(6, 100.0, 50.0, 10.0, 4, 2).expect("a sheet draws");
        assert_eq!(icon.origin, [95.0, 45.0]);
        assert_eq!(icon.size, 10.0);
        assert_eq!(icon.uv, [0.25, 0.0]);
        assert_eq!(icon.cell, [0.25, 0.5]);
        assert_eq!(icon.bias, f32::from_bits(0x3f91_eb85));
        assert_eq!(icon.pad, f32::from_bits(0x4108_0000));
        assert_eq!(
            icon_square(0x106, 100.0, 50.0, 10.0, 4, 2)
                .expect("a marked icon")
                .bias,
            1.0
        );

        assert_eq!(picture_span(0, 10.5), 10.0);
        assert_eq!(picture_span(0x0100_0000, 10.5), f32::from_bits(0x40a8_0000));

        let frame = frame_square(5, 0.0, 0.0, 10.0);
        assert_eq!(frame.size, 8.0);
        assert_eq!(frame.origin, [-4.0, -4.0]);
        assert_eq!(frame.uv, [0.0, 0.0]);
        assert_eq!(frame.cell, [0.25, 0.5]);
        assert_eq!(frame.shrink[0], f32::from_bits(0x3ba3_3333));
        assert_eq!(frame.shrink[1], f32::from_bits(0x3c23_3333));
        assert_eq!(frame_square(1, 0.0, 0.0, 10.0).uv, [0.0, 0.5]);

        let glow = glow_ring(0.0, 0.0, 10.0, 4.0);
        assert_eq!(glow.inner, f32::from_bits(0x4106_6666));
        assert_eq!(glow.outer, f32::from_bits(0x4146_6666));
        assert_eq!(glow.spokes.len(), 33);
        assert_eq!(glow.spokes[0], [0.0, glow.inner]);
    }

    fn node(power: u32, links: Vec<[u8; 16]>) -> PowerNode {
        PowerNode {
            power,
            extra: 0,
            flags: 0,
            x: 0.0,
            y: 0.0,
            links,
        }
    }

    fn link_to(end: u32) -> [u8; 16] {
        let mut link = [0u8; 16];
        link[8..12].copy_from_slice(&end.to_le_bytes());
        link
    }

    #[test]
    fn a_known_power_opens_the_nodes_it_reaches() {
        assert!(learn_marks(&[], &[]).is_empty());
        assert_eq!(learn_marks(&[node(7, vec![])], &[]), vec![0x10000]);
        assert_eq!(
            learn_marks(&[node(7, vec![])], &[(0x00f0_0007, 5), (7, 2)]),
            vec![0x10002]
        );
        assert_eq!(
            learn_marks(&[node(0x7000_0000, vec![])], &[]),
            vec![0x10000]
        );

        let closed = learn_marks(&[node(7, vec![]), node(9, vec![link_to(1)])], &[]);
        assert_eq!(closed, vec![0x10000, 0]);

        let opened = learn_marks(
            &[node(7, vec![]), node(9, vec![link_to(1)])],
            &[(7, 0x1000)],
        );
        assert_eq!(opened, vec![0x11000, 0x10000]);

        let skipped = learn_marks(
            &[node(7, vec![]), node(0x7000_0000, vec![link_to(1)])],
            &[(7, 0x1000)],
        );
        assert_eq!(skipped, vec![0x11000, 0]);

        let held = learn_marks(
            &[node(7, vec![]), node(9, vec![link_to(0)])],
            &[(9, 0x5000)],
        );
        assert_eq!(held, vec![0x10000, 0x15000]);
    }

    #[test]
    fn a_thick_joint_sits_across_the_chord() {
        let straight = thick_joint([0.0, 0.0], [10.0, 0.0], [20.0, 0.0], 2.0);
        assert_eq!(straight.left, [10.0, -2.0]);
        assert_eq!(straight.right, [10.0, 2.0]);
        assert_eq!(straight.join, 0.0);

        let turned = thick_joint([0.0, 0.0], [10.0, 0.0], [10.0, 10.0], 2.0);
        assert_eq!(
            turned.left,
            [f32::from_bits(0x4136_a09e), f32::from_bits(0xbfb5_04f4)]
        );
        assert_eq!(
            turned.right,
            [f32::from_bits(0x4109_5f62), f32::from_bits(0x3fb5_04f4)]
        );
        assert_eq!(turned.join, f32::from_bits(0x3f22_eadc));

        let still = thick_joint([1.0, 2.0], [1.0, 2.0], [1.0, 2.0], 2.0);
        assert_eq!(still.left, [1.0, 2.0]);
        assert_eq!(still.right, [1.0, 2.0]);
        assert_eq!(still.join, 0.0);
    }

    #[test]
    fn loose_power_trees_parse_when_they_are_installed() {
        let Ok(root) = std::env::var("EXANIMA_DIR") else {
            return;
        };
        let mut stack = vec![(std::path::PathBuf::from(root), 0u8)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() && depth < 3 {
                    stack.push((path, depth + 1));
                    continue;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("pwr") {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                let tree = read_power_tree(&bytes);
                assert!(tree.is_ok(), "a loose power tree did not parse");
                assert!(!tree.unwrap().nodes.is_empty());
            }
        }
    }
}
