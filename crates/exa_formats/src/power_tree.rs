//! A thaumaturgy power tree.
//!
//! The file's first word is the power magic. The node count is the word at
//! byte 52. Each node is a power id, two more words, an x, a y, a link count,
//! and that many 16-byte links. A link count below 1 stores none. Every node
//! after the first subtracts the first node's x and y. A short file or a
//! negative count is an error. The text library after the nodes is not read.
//!
//! Evidence: the tree reader (magic check, the seek to the count, the node
//! fields, the link bytes, and the shift against the first node).

use crate::{Error, Result, read_u32};

/// First word of a power tree.
pub const MAGIC: u32 = 0xAFCE_01CE;
/// The node count sits here.
const COUNT_AT: usize = 52;
/// The first node starts here.
const NODES_AT: usize = 56;
/// Bytes in one link.
const LINK: usize = 16;
/// Bytes in a node before its links.
const NODE_HEAD: usize = 24;

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

/// The nodes of one domain's tree.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerTree {
    pub nodes: Vec<PowerNode>,
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
    Ok(PowerTree { nodes })
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
