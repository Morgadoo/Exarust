//! Level sections of a chunk tree: the tile grid (`0x3DED`) and placed props
//! (`0x3DE1_0000`). Layout and evidence: `docs/formats/RFC.md`.

use crate::{Error, Result};

pub const GRID: u32 = 0x3DED;
pub const PROPS: u32 = 0x3DE1_0000;

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
}

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
    let edges = c.u32s(cells)?;
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
        props.push(Prop {
            object,
            flags,
            transform,
            parent: (parent > 0).then_some(parent),
            extra,
        });
    }
    Ok(props)
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
        let c = g.cell(1, 0).unwrap();
        assert_eq!((g.edge(c, 0), g.edge(c, 3)), (7, 7));
        assert_eq!(g.edge(c, 1), 0);
        assert_eq!(g.plane_c, [3, 4]);
        assert!(g.cell(2, 0).is_none());
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
        };
        assert!(prop(0x8_0000).is_static());
        assert!(prop(0x18_0000).is_static());
        assert!(!prop(0x10_0000).is_static() && !prop(0x20_0000).is_static());
    }
}
