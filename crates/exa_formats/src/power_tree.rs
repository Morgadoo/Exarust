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
//! zero source mark dims the line.
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
