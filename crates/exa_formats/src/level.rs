//! Level sections of a chunk tree: the tile grid (`0x3DED`) and placed props
//! (`0x3DE1_0000`). Layout and evidence: `docs/formats/RFC.md`.

use crate::{Error, Result};

pub const GRID: u32 = 0x3DED;
pub const PROPS: u32 = 0x3DE1_0000;

/// Whether a chunk tree is a level: one of its top-level chunks is the tile
/// grid or the placed props. Reads only the chunk headers; malformed input
/// is not a level.
pub fn is_level(b: &[u8]) -> bool {
    crate::FileKind::detect(b) == crate::FileKind::ChunkTree
        && crate::chunk::Chunks::new(b, 4)
            .map_while(Result::ok)
            .any(|c| c.tag == GRID || c.tag == PROPS)
}

/// Files whose third grid word holds this value store planes B and C in an
/// older bit layout, converted on load.
const LEGACY_MARKER: u32 = 600;
const CENTIMETRES_TO_METRES: f32 = 0.01;

/// The tile grid of a dungeon level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelGrid {
    pub wall_sets: Vec<String>,
    pub floor_sets: Vec<String>,
    pub width: u32,
    pub height: u32,
    /// Per cell, row-major: four 4-bit edge codes in the low 16 bits (top,
    /// left, bottom, right) and four 4-bit corner codes in the high 16 bits.
    pub edges: Vec<u32>,
    pub plane_b: Vec<u32>,
    pub plane_c: Vec<u32>,
}

impl LevelGrid {
    pub fn cell(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.width && y < self.height).then(|| (y * self.width + x) as usize)
    }

    /// Edge code `side` (0 top, 1 left, 2 bottom, 3 right) of a cell.
    pub fn edge(&self, cell: usize, side: u32) -> u32 {
        (self.edges[cell] >> (side * 4)) & 0xF
    }

    /// Corner code `corner` (0..4) of a cell.
    pub fn corner(&self, cell: usize, corner: u32) -> u32 {
        (self.edges[cell] >> (16 + corner * 4)) & 0xF
    }
}

/// An object from the object library placed in a level.
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub object: String,
    /// Placement flags; the original copies `0xB8_0000` of them onto the
    /// object's root node ([`Prop::STATIC`]).
    pub flags: u32,
    /// Row-major 3×4: three rotation rows, then the translation (metres).
    pub transform: [[f32; 3]; 4],
    /// 1-based index of the parent prop, if any.
    pub parent: Option<usize>,
    pub extra: u32,
    /// What the placement changes in the object's own nodes, in file order.
    pub attachments: Vec<PropAttachment>,
}

/// A change a placement makes to one node of its object, as the original's
/// `0x3DE0ECAC` and `0x3DE0ECDB` records carry it. The node is looked up
/// from the node the record applies to (the object's root for a top-level
/// record): that node itself for an empty name, else its first direct child
/// whose 16-byte name matches exactly. A record whose node is not found is
/// skipped with everything nested in it.
#[derive(Debug, Clone, PartialEq)]
pub enum PropAttachment {
    /// A new local transform (row-major 3×4, translation in metres), then
    /// the records for that node's own children.
    Transform {
        name: String,
        /// The word after the name; the original does not read it (0 in
        /// every shipped record).
        word: u32,
        transform: [[f32; 3]; 4],
        children: Vec<PropAttachment>,
    },
    /// New positions for the node's mesh (metres): the position stream's
    /// elements, which the original replaces when their count matches the
    /// stream's.
    MeshPositions {
        name: String,
        /// The word after the name; the original copies it onto the
        /// node's own copy of its owner's soft-body record [guess: the
        /// record was not traced]. 0 in every shipped record.
        word: u32,
        positions: Vec<[f32; 3]>,
    },
    /// A whole tree of node transforms (`0x3DE0EC00`): each entry names a
    /// child of the node the tree applies to, replaces its local transform
    /// and carries its own children's entries. An entry whose node is not
    /// found is skipped with its children.
    Tree { entries: Vec<PropAttachment> },
}

impl PropAttachment {
    /// The node name the record looks up (empty: the node it applies to).
    pub fn name(&self) -> &str {
        match self {
            Self::Transform { name, .. } | Self::MeshPositions { name, .. } => name,
            Self::Tree { .. } => "",
        }
    }
}

/// How deep the reader follows nested transform records; deeper ones (no
/// shipped object nests more than a few) are dropped rather than recursed.
const ATTACHMENT_DEPTH: usize = 32;

/// Sub-chunk tags of a placed object's node overrides.
pub const PROP_TRANSFORM: u32 = 0x3DE0_ECAC;
pub const PROP_MESH_POSITIONS: u32 = 0x3DE0_ECDB;
pub const PROP_TREE: u32 = 0x3DE0_EC00;

impl Prop {
    /// Placement flag: the whole object is level geometry. The original
    /// marks every node under it, so none of its rigid bodies is
    /// registered and its meshes stay static colliders.
    pub const STATIC: u32 = 0x8_0000;

    /// Whether the prop is placed as static level geometry ([`Prop::STATIC`]).
    pub fn is_static(&self) -> bool {
        self.flags & Self::STATIC != 0
    }
}

/// The loader's border fill: every load writes the fixed edge words of the
/// grid's outer rows and columns into plane A, whatever the file stored:
/// the whole top row, the left and right columns, the whole bottom row, then
/// the four corners, in that order (a one-row or one-column grid ends with
/// the last write). Values are plane A's edge/corner words.
pub fn fill_borders(edges: &mut [u32], width: u32, height: u32) {
    let (w, h) = (width as usize, height as usize);
    // An empty grid has no border (the original would write outside it).
    if w == 0 || h == 0 || edges.len() < w * h {
        return;
    }
    edges[..w].fill(0x2001_0007);
    for row in 0..h {
        edges[row * w] = 0x0012_0070;
        edges[row * w + w - 1] = 0x1200_7000;
    }
    edges[(h - 1) * w..w * h].fill(0x0120_0700);
    edges[0] = 0x2013_0077;
    edges[w - 1] = 0x3201_7007;
    edges[(h - 1) * w] = 0x0132_0770;
    edges[w * h - 1] = 0x1320_7700;
}

pub(crate) fn read_grid(b: &[u8]) -> Result<LevelGrid> {
    let mut c = Reader { b, pos: 0 };
    let wall_sets = c.names()?;
    let floor_sets = c.names()?;
    let width = c.u32()?;
    let height = c.u32()?;
    let marker = c.u32()?;
    c.u32()?;
    let cells = (width as usize)
        .checked_mul(height as usize)
        .filter(|&n| {
            n.checked_mul(12)
                .is_some_and(|bytes| bytes <= c.remaining())
        })
        .ok_or_else(|| Error::Malformed(format!("grid {width}x{height} does not fit")))?;
    let mut edges = c.u32s(cells)?;
    let mut plane_b = c.u32s(cells)?;
    let mut plane_c = c.u32s(cells)?;
    if marker == LEGACY_MARKER {
        for (b, c) in plane_b.iter_mut().zip(plane_c.iter_mut()) {
            let old_b = *b;
            *b &= 0x3333_FFFF;
            let v = *c;
            *c = (v & 0x18_0000) << 2
                | (v & 0x7_0000)
                | (v & 0xE0_0000) << 2
                | ((old_b >> 20 & 0xC) | (old_b >> 18 & 0x3)) << 8
                | (v & 0xFF);
        }
    }
    fill_borders(&mut edges, width, height);
    Ok(LevelGrid {
        wall_sets,
        floor_sets,
        width,
        height,
        edges,
        plane_b,
        plane_c,
    })
}

pub(crate) fn read_props(b: &[u8]) -> Result<Vec<Prop>> {
    let mut c = Reader { b, pos: 0 };
    let count = c.u32()? as usize;
    if count.saturating_mul(8) > c.remaining() {
        return Err(Error::Malformed(format!("{count} props do not fit")));
    }
    let mut props = Vec::with_capacity(count);
    for _ in 0..count {
        c.u32()?; // record tag
        let size = c.u32()? as usize;
        let rec = c.bytes(size)?;
        let mut r = Reader { b: rec, pos: 0 };
        let flags = r.u32()?;
        let object = r.name()?;
        let mut transform = [[0.0f32; 3]; 4];
        for row in &mut transform {
            for v in row.iter_mut() {
                *v = f32::from_bits(r.u32()?);
            }
        }
        for v in &mut transform[3] {
            *v *= CENTIMETRES_TO_METRES;
        }
        let parent = r.u32()? as usize;
        let extra = r.u32()?;
        let mut attachments = Vec::new();
        while r.remaining() >= 8 {
            let Ok(tag) = r.u32() else { break };
            let Ok(size) = r.u32() else { break };
            let Ok(body) = r.bytes(size as usize) else {
                break;
            };
            if let Some(a) = read_attachment(tag, body) {
                attachments.push(a);
            }
        }
        props.push(Prop {
            object,
            flags,
            transform,
            parent: (parent > 0).then_some(parent),
            extra,
            attachments,
        });
    }
    Ok(props)
}

/// Reads a placement's node override chunk; `None` for another tag or a
/// record that does not decode.
pub(crate) fn read_attachment(tag: u32, body: &[u8]) -> Option<PropAttachment> {
    read_attachment_at(tag, body, 0)
}

fn read_attachment_at(tag: u32, body: &[u8], depth: usize) -> Option<PropAttachment> {
    match tag {
        PROP_TRANSFORM => read_prop_transform(body, depth).ok(),
        PROP_MESH_POSITIONS => read_prop_positions(body).ok(),
        PROP_TREE => read_prop_tree(body, depth).ok(),
        _ => None,
    }
}

/// One `0x3DE0EC00` tree: a count, a skipped word, then that many entries.
/// Each entry is a name, a skipped word, a 3×4 transform in centimetres, a
/// child count, the number of records its subtree holds, then its children;
/// the next sibling follows the subtree.
fn read_prop_tree(b: &[u8], depth: usize) -> Result<PropAttachment> {
    let mut r = Reader { b, pos: 0 };
    let count = r.u32()? as usize;
    r.u32()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        if let Some(entry) = read_tree_entry(&mut r, depth + 1)? {
            entries.push(entry);
        }
    }
    Ok(PropAttachment::Tree { entries })
}

fn read_tree_entry(r: &mut Reader<'_>, depth: usize) -> Result<Option<PropAttachment>> {
    let start = r.pos;
    let name = r.name()?;
    let word = r.u32()?;
    let mut transform = [[0.0f32; 3]; 4];
    for row in &mut transform {
        for v in row.iter_mut() {
            *v = f32::from_bits(r.u32()?);
        }
    }
    for v in &mut transform[3] {
        *v *= CENTIMETRES_TO_METRES;
    }
    let children = r.u32()? as usize;
    let descendants = r.u32()? as usize;
    let mut kids = Vec::new();
    // Past the depth limit the record is still skipped by its stride, so
    // its siblings stay in step, but it is not kept.
    let too_deep = depth > ATTACHMENT_DEPTH;
    if !too_deep {
        for _ in 0..children {
            if let Some(kid) = read_tree_entry(r, depth + 1)? {
                kids.push(kid);
            }
        }
    }
    // The next sibling follows the record and its whole subtree:
    // `(descendants + 1)` records of 0x4c bytes from here.
    let stride = descendants
        .checked_add(1)
        .and_then(|n| n.checked_mul(TREE_RECORD_BYTES))
        .ok_or_else(|| Error::Malformed("a placement tree does not fit".into()))?;
    let target = start + stride;
    if r.pos > target {
        return Err(Error::Malformed("a placement tree record overlaps".into()));
    }
    r.bytes(target - r.pos)?;
    if too_deep {
        return Ok(None);
    }
    Ok(Some(PropAttachment::Transform {
        name,
        word,
        transform,
        children: kids,
    }))
}

/// A tree record's fixed size: the 16-byte name, the word and transform,
/// child count and descendant count.
const TREE_RECORD_BYTES: usize = 0x4c;

/// One `0x3DE0ECAC` record: the node's name, a word, a 3×4 transform whose
/// translation is in centimetres (68 bytes), then sub-chunks: the records
/// for that node's children (`0x3DE0ECAC` and `0x3DE0ECDB`; others are
/// skipped, as is a truncated one and everything after it).
fn read_prop_transform(b: &[u8], depth: usize) -> Result<PropAttachment> {
    let mut r = Reader { b, pos: 0 };
    let name = r.name()?;
    let word = r.u32()?;
    let mut transform = [[0.0f32; 3]; 4];
    for row in &mut transform {
        for v in row.iter_mut() {
            *v = f32::from_bits(r.u32()?);
        }
    }
    for v in &mut transform[3] {
        *v *= CENTIMETRES_TO_METRES;
    }
    let mut children = Vec::new();
    while depth < ATTACHMENT_DEPTH && r.remaining() >= 8 {
        let Ok(tag) = r.u32() else { break };
        let Ok(size) = r.u32() else { break };
        let Ok(body) = r.bytes(size as usize) else {
            break;
        };
        if let Some(a) = read_attachment_at(tag, body, depth + 1) {
            children.push(a);
        }
    }
    Ok(PropAttachment::Transform {
        name,
        word,
        transform,
        children,
    })
}

/// One `0x3DE0ECDB` record: the node's name, a word, a count and that many
/// positions in centimetres. The original's reader leaves its cursor at the
/// record's start when it looks for nested records, so it reads the name as
/// a chunk header and applies none (and none ships).
fn read_prop_positions(b: &[u8]) -> Result<PropAttachment> {
    let mut r = Reader { b, pos: 0 };
    let name = r.name()?;
    let word = r.u32()?;
    let count = r.u32()? as usize;
    if count.saturating_mul(12) > r.remaining() {
        return Err(Error::Malformed(format!(
            "{count} placement positions do not fit"
        )));
    }
    let mut positions = Vec::with_capacity(count);
    for _ in 0..count {
        let mut p = [0.0f32; 3];
        for v in &mut p {
            *v = f32::from_bits(r.u32()?) * CENTIMETRES_TO_METRES;
        }
        positions.push(p);
    }
    Ok(PropAttachment::MeshPositions {
        name,
        word,
        positions,
    })
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.b.len() - self.pos
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.pos..self.pos + n).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.b.len(),
        })?;
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn u32s(&mut self, n: usize) -> Result<Vec<u32>> {
        Ok(self
            .bytes(n * 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|x| u32::from_le_bytes(*x))
            .collect())
    }

    fn name(&mut self) -> Result<String> {
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
    }

    fn names(&mut self) -> Result<Vec<String>> {
        let n = self.u32()? as usize;
        if n.saturating_mul(16) > self.remaining() {
            return Err(Error::Malformed(format!("{n} set names do not fit")));
        }
        (0..n).map(|_| self.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> Vec<u8> {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n.to_vec()
    }

    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    /// A chunk tree with a grid or props chunk at the top is a level; a
    /// model, a truncated tree or another kind of file is not.
    #[test]
    fn levels_have_a_grid_or_props_at_the_top() {
        let tree = |tag: u32| u32s(&[0x3D23_AFCF, 0xBA00, 0, tag, 4, 0]);
        assert!(is_level(&tree(GRID)));
        assert!(is_level(&tree(PROPS)));
        assert!(!is_level(&tree(0x3D00_0000)));
        let mut cut = tree(GRID);
        cut.truncate(cut.len() - 2);
        assert!(!is_level(&cut));
        assert!(!is_level(&u32s(&[0xAFBF_0C01, GRID, 0])));
        assert!(!is_level(&[]));
    }

    #[test]
    fn grid_planes_and_codes() {
        let mut b = u32s(&[1]);
        b.extend(name("GEWS_A"));
        b.extend(u32s(&[2]));
        b.extend(name("GEFS_A"));
        b.extend(name("GEFS_B"));
        b.extend(u32s(&[2, 1, 0, 600]));
        b.extend(u32s(&[0x2013_0077, 0x3201_7007]));
        b.extend(u32s(&[1, 2]));
        b.extend(u32s(&[3, 4]));
        let g = read_grid(&b).unwrap();
        assert_eq!(g.floor_sets, ["GEFS_A", "GEFS_B"]);
        assert_eq!((g.width, g.height), (2, 1));
        // A one-row grid's plane A ends as the loader's last corner writes,
        // whatever the file stored.
        assert_eq!(g.edges, [0x0132_0770, 0x1320_7700]);
        let c = g.cell(1, 0).unwrap();
        assert_eq!((g.edge(c, 0), g.edge(c, 1)), (0, 0));
        assert_eq!((g.edge(c, 2), g.edge(c, 3)), (7, 7));
        assert_eq!(g.plane_c, [3, 4]);
        assert!(g.cell(2, 0).is_none());
    }

    #[test]
    fn the_loader_fills_the_grid_border() {
        let (w, h) = (4usize, 3usize);
        let mut edges = vec![0u32; w * h];
        fill_borders(&mut edges, w as u32, h as u32);
        assert_eq!(
            edges,
            [
                0x2013_0077,
                0x2001_0007,
                0x2001_0007,
                0x3201_7007,
                0x0012_0070,
                0,
                0,
                0x1200_7000,
                0x0132_0770,
                0x0120_0700,
                0x0120_0700,
                0x1320_7700,
            ]
        );
        // One row: the bottom-row and corner writes land on the same cells;
        // the last corner wins.
        let mut row = vec![9u32; 3];
        fill_borders(&mut row, 3, 1);
        assert_eq!(row, [0x0132_0770, 0x0120_0700, 0x1320_7700]);
        // One cell takes the last corner word.
        let mut one = vec![9u32; 1];
        fill_borders(&mut one, 1, 1);
        assert_eq!(one, [0x1320_7700]);
        // An empty grid is left alone.
        let mut empty: Vec<u32> = Vec::new();
        fill_borders(&mut empty, 0, 3);
        assert!(empty.is_empty());
    }

    #[test]
    fn a_tree_override_reads_named_transforms_and_skips_its_word() {
        let mut body = u32s(&[2, 0]);
        let entry = |name_: &str, x: f32, children: u32, descendants: u32| {
            let mut b = name(name_);
            b.extend(u32s(&[7]));
            for row in 0..4 {
                for col in 0..3 {
                    let v = if row == 3 && col == 0 { x } else { 0.0 };
                    b.extend(u32s(&[v.to_bits()]));
                }
            }
            b.extend(u32s(&[children, descendants]));
            b
        };
        body.extend(entry("arm", 100.0, 0, 0));
        body.extend(entry("head", 200.0, 0, 0));
        let Some(PropAttachment::Tree { entries }) = read_attachment(PROP_TREE, &body) else {
            panic!("not a tree");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name(), "arm");
        let PropAttachment::Transform { transform, .. } = &entries[0] else {
            panic!("not a transform");
        };
        assert_eq!(transform[3][0], 1.0);
        let PropAttachment::Transform { transform, .. } = &entries[1] else {
            panic!("not a transform");
        };
        assert_eq!(transform[3][0], 2.0);
    }

    #[test]
    fn a_tree_child_follows_its_subtree_stride() {
        let mut body = u32s(&[2, 0]);
        // One parent with one child: the parent's descendants count is one,
        // so the next sibling starts two records on.
        body.extend(name("arm"));
        body.extend(u32s(&[0]));
        body.extend(u32s(&[0; 12]));
        body.extend(u32s(&[1, 1]));
        body.extend(name("hand"));
        body.extend(u32s(&[0]));
        body.extend(u32s(&[0; 12]));
        body.extend(u32s(&[0, 0]));
        body.extend(name("torso"));
        body.extend(u32s(&[0]));
        body.extend(u32s(&[0; 12]));
        body.extend(u32s(&[0, 0]));
        let Some(PropAttachment::Tree { entries }) = read_attachment(PROP_TREE, &body) else {
            panic!("not a tree");
        };
        let PropAttachment::Transform { children, .. } = &entries[0] else {
            panic!("not a transform");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name(), "hand");
        assert_eq!(entries[1].name(), "torso");
    }

    #[test]
    fn grid_that_does_not_fit_is_rejected() {
        let mut b = u32s(&[0, 0, 1000, 1000, 0, 0]);
        b.extend(u32s(&[0; 3]));
        assert!(read_grid(&b).is_err());
    }

    #[test]
    fn props_in_metres() {
        let mut rec = u32s(&[0x10]);
        rec.extend(name("chair a"));
        let m: [f32; 12] = [1., 0., 0., 0., 1., 0., 0., 0., 1., 250., 0., -100.];
        rec.extend(m.iter().flat_map(|f| f.to_le_bytes()));
        rec.extend(u32s(&[0, 0]));
        let mut b = u32s(&[1, 0x3DE1_0100, rec.len() as u32]);
        b.extend(rec);
        let props = read_props(&b).unwrap();
        assert_eq!(props[0].object, "chair a");
        assert_eq!(props[0].transform[3], [2.5, 0.0, -1.0]);
        assert_eq!(props[0].parent, None);
        assert!(!props[0].is_static());
    }

    /// A prop's node overrides: a transform with a nested child record, a
    /// position list, and a truncated record that is dropped.
    #[test]
    fn prop_attachments_change_a_node() {
        let mut rec = u32s(&[0]);
        rec.extend(name("banner a01"));
        let m: [f32; 12] = [1., 0., 0., 0., 1., 0., 0., 0., 1., 250., 0., -100.];
        rec.extend(m.iter().flat_map(|f| f.to_le_bytes()));
        rec.extend(u32s(&[0, 0]));
        // A node transform: name, a word, then 3 x 4 rows in centimetres,
        // then the records of that node's children.
        let mut tr = name("UW01_Bann");
        tr.extend(u32s(&[7]));
        let rows: [f32; 12] = [1., 0., 0., 0., 1., 0., 0., 0., 1., 100., 200., 300.];
        tr.extend(rows.iter().flat_map(|f| f.to_le_bytes()));
        let mut child = name("Ring");
        child.extend(u32s(&[0]));
        child.extend(rows.iter().flat_map(|f| f.to_le_bytes()));
        tr.extend(u32s(&[PROP_TRANSFORM, child.len() as u32]));
        tr.extend(child);
        rec.extend(u32s(&[PROP_TRANSFORM, tr.len() as u32]));
        rec.extend(tr);
        // New positions: name, a word, count, then the positions.
        let mut pos = name("UW01_Bann");
        pos.extend(u32s(&[7, 2]));
        pos.extend(f32s(&[0.0, 0.0, 0.0, 100.0, 50.0, -20.0]));
        rec.extend(u32s(&[PROP_MESH_POSITIONS, pos.len() as u32]));
        rec.extend(pos);
        // A truncated override is dropped, the prop still reads.
        rec.extend(u32s(&[PROP_MESH_POSITIONS, 8]));
        rec.extend(u32s(&[1, 2]));
        let mut b = u32s(&[1, 0x3DE1_0100, rec.len() as u32]);
        b.extend(rec);
        let props = read_props(&b).unwrap();
        let p = &props[0];
        const ROWS: [[f32; 3]; 4] = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 2.0, 3.0],
        ];
        assert_eq!(p.transform[3], [2.5, 0.0, -1.0]);
        assert_eq!(p.attachments.len(), 2, "{:?}", p.attachments);
        assert_eq!(
            p.attachments[0],
            PropAttachment::Transform {
                name: "UW01_Bann".into(),
                word: 7,
                transform: ROWS,
                children: vec![PropAttachment::Transform {
                    name: "Ring".into(),
                    word: 0,
                    transform: ROWS,
                    children: Vec::new(),
                }],
            }
        );
        let PropAttachment::MeshPositions {
            name,
            word,
            positions,
        } = &p.attachments[1]
        else {
            panic!("not a position override");
        };
        assert_eq!((name.as_str(), *word), ("UW01_Bann", 7));
        assert_eq!(positions[0], [0.0, 0.0, 0.0]);
        assert!((positions[1][0] - 1.0).abs() < 1e-6);
        assert!((positions[1][1] - 0.5).abs() < 1e-6);
        assert!((positions[1][2] + 0.2).abs() < 1e-6);
    }

    fn f32s(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    /// Placement flag `0x80000` makes the whole object level geometry
    /// (3 883 of the 79 935 shipped props); `0x100000` and `0x200000` do
    /// not.
    #[test]
    fn static_props() {
        let prop = |flags| Prop {
            object: "barrel wdo1 a01".into(),
            flags,
            transform: [[0.0; 3]; 4],
            parent: None,
            extra: 0,
            attachments: Vec::new(),
        };
        assert!(prop(0x8_0000).is_static());
        assert!(prop(0x18_0000).is_static());
        assert!(!prop(0x10_0000).is_static() && !prop(0x20_0000).is_static());
    }
}
