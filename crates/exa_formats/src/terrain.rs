//! Terrain heightfields (`.rft`) and terrain palettes (`.ftb`).
//!
//! A terrain is a grid of 48 m tiles centred on the level origin. Each tile
//! stores 241 × 241 heights (20 cm apart) inside a 244 × 244 apron, an optional hole mask, and painted layers: a material or blend-rule
//! name from the palette with an 8-bit weight per vertex.
//!
//! A palette names the materials (texture set + shading parameters) and the
//! blend rules that mix two children by height, slope or noise.
//! Layout and evidence: `docs/formats/TERRAIN.md`.

use crate::{Error, Result};

pub const TERRAIN_MAGIC: u32 = 0x3EEF_AD01;
pub const PALETTE_MAGIC_V2: u32 = 0x3EEF_BD02;
pub const PALETTE_MAGIC_V3: u32 = 0x3EEF_BD03;
pub const TILE: u32 = 0xBDAD;
pub const LAYERS: u32 = 0xBD01;

/// Vertices along a tile edge.
pub const TILE_VERTICES: usize = 241;
/// Side of the stored height grid: one apron sample before, two after.
const APRON_SIDE: usize = 244;
const HEIGHTS_BYTES: usize = APRON_SIDE * APRON_SIDE * 4;
/// Hole mask: one bit per vertex, rows padded to 31 bytes.
const HOLE_ROW_BYTES: usize = 31;
const HOLES_BYTES: usize = HOLE_ROW_BYTES * TILE_VERTICES;
const TILE_HAS_HOLES: u32 = 0x200;
const LAYER_BYTES: usize = 16 + TILE_VERTICES * TILE_VERTICES;
const CENTIMETRES_PER_METRE: f32 = 100.0;
/// Stored heights are multiplied by the vertex spacing (cm) times this to
/// give centimetres: 2 for the 20 cm grid every terrain uses.
const VERTICAL_SCALE_PER_SPACING: f32 = 0.1;

#[derive(Debug, Clone)]
pub struct Terrain {
    /// Palette (`.ftb`) name.
    pub palette: String,
    /// Distance between vertices, metres.
    pub spacing: f32,
    /// Tile grid size; tiles may be missing.
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TerrainTile>,
}

#[derive(Debug, Clone)]
pub struct TerrainTile {
    pub x: u32,
    pub y: u32,
    pub flags: u32,
    /// Seed of the tile's grass and plant placement.
    pub vegetation_seed: u32,
    /// 244 × 244 row-major apron grid, metres; see [`TerrainTile::height`].
    heights: Vec<f32>,
    holes: Option<Vec<u8>>,
    pub layers: Vec<TerrainLayer>,
}

#[derive(Debug, Clone)]
pub struct TerrainLayer {
    /// Material or blend-rule name in the palette (matched without case).
    pub name: String,
    /// 241 × 241 row-major weights; layers of a vertex sum to about 255.
    pub weights: Vec<u8>,
}

impl Terrain {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let magic = r.u32()?;
        if magic != TERRAIN_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: TERRAIN_MAGIC,
            });
        }
        r.u32()?;
        let palette = r.name()?;
        let spacing_cm = r.f32()?;
        let spacing = spacing_cm / CENTIMETRES_PER_METRE;
        let to_metres = spacing_cm * VERTICAL_SCALE_PER_SPACING / CENTIMETRES_PER_METRE;
        let width = r.u32()?;
        let height = r.u32()?;
        let count = r.u32()? as usize;
        let mut tiles = Vec::with_capacity(count.min(1024));
        while r.remaining() >= 8 {
            let tag = r.u32()?;
            let size = r.u32()? as usize;
            let body = r.bytes(size)?;
            if tag == TILE {
                tiles.push(read_tile(body, to_metres)?);
            }
        }
        Ok(Self {
            palette,
            spacing,
            width,
            height,
            tiles,
        })
    }

    /// Centre of a tile relative to the level origin, metres, on the X/Z plane.
    pub fn tile_centre(&self, tile: &TerrainTile) -> [f32; 2] {
        let side = self.spacing * (TILE_VERTICES - 1) as f32;
        [
            (tile.x as f32 - (self.width as f32 - 1.0) * 0.5) * side,
            (tile.y as f32 - (self.height as f32 - 1.0) * 0.5) * side,
        ]
    }
}

impl TerrainTile {
    /// Height in metres of vertex (`i`, `j`), `i` along X and `j` along Z,
    /// both in `0..241`.
    pub fn height(&self, i: usize, j: usize) -> f32 {
        self.heights[(j + 1) * APRON_SIDE + i + 1]
    }

    /// Like [`TerrainTile::height`] but reaching one vertex into the
    /// neighbouring tiles through the apron (`-1..=241`); clamped beyond.
    pub fn height_apron(&self, i: i32, j: i32) -> f32 {
        let i = (i + 1).clamp(0, TILE_VERTICES as i32 + 1) as usize;
        let j = (j + 1).clamp(0, TILE_VERTICES as i32 + 1) as usize;
        self.heights[j * APRON_SIDE + i]
    }

    /// Whether vertex (`i`, `j`) is cut out of the surface.
    pub fn is_hole(&self, i: usize, j: usize) -> bool {
        self.holes
            .as_ref()
            .is_some_and(|h| h[j * HOLE_ROW_BYTES + i / 8] >> (i % 8) & 1 != 0)
    }
}

fn read_tile(b: &[u8], to_metres: f32) -> Result<TerrainTile> {
    let mut r = Reader::new(b);
    let flags = r.u32()?;
    let x = r.u32()?;
    let y = r.u32()?;
    let vegetation_seed = r.u32()?;
    let heights = r
        .bytes(HEIGHTS_BYTES)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| f32::from_le_bytes(*v) * to_metres)
        .collect();
    let holes = if flags & TILE_HAS_HOLES != 0 {
        Some(r.bytes(HOLES_BYTES)?.to_vec())
    } else {
        None
    };
    let mut layers = Vec::new();
    while r.remaining() >= 8 {
        let tag = r.u32()?;
        let size = r.u32()? as usize;
        let body = r.bytes(size)?;
        if tag != LAYERS {
            continue;
        }
        let mut l = Reader::new(body);
        let n = l.u32()? as usize;
        if n.checked_mul(LAYER_BYTES).is_none_or(|s| s > l.remaining()) {
            return Err(Error::Malformed(format!("{n} terrain layers do not fit")));
        }
        for _ in 0..n {
            let name = l.name()?.to_ascii_lowercase();
            let weights = l.bytes(TILE_VERTICES * TILE_VERTICES)?.to_vec();
            layers.push(TerrainLayer { name, weights });
        }
    }
    Ok(TerrainTile {
        x,
        y,
        flags,
        vegetation_seed,
        heights,
        holes,
        layers,
    })
}

/// Texture slots of a terrain material, in file order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTexture {
    Diffuse = 0,
    Normal = 1,
    Gloss = 2,
    Mask = 3,
    Height = 4,
    Bump = 5,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainMaterial {
    pub name: String,
    pub flags: u32,
    /// Texture names by [`TerrainTexture`] slot; empty when unused.
    pub textures: [String; 6],
    /// Shading parameters (25 in version 2 palettes, 29 in version 3). The
    /// first two look like tiling factors and 6..10 an RGBA tint.
    pub params: Vec<f32>,
}

impl TerrainMaterial {
    pub fn texture(&self, slot: TerrainTexture) -> Option<&str> {
        let t = &self.textures[slot as usize];
        (!t.is_empty()).then_some(t.as_str())
    }

    pub fn tint(&self) -> [f32; 4] {
        std::array::from_fn(|i| self.params.get(6 + i).copied().unwrap_or(1.0))
    }
}

/// Blends `a` into `b` as the quantity selected by `kind` goes from `lo` to
/// `hi`. Children name materials or other rules.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendRule {
    pub name: String,
    pub flags: u32,
    pub kind: u32,
    pub a: String,
    pub b: String,
    pub lo: f32,
    pub hi: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plant {
    pub flags: u32,
    pub name: String,
    pub params: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub name: String,
    /// Header float at offset 24. The original copies it into the scene's
    /// environment record; the terrain's passes do not read it.
    pub environment: f32,
    /// Header float at offset 28. The terrain draw puts it in its specular
    /// level (`+ 0.3`, unused while the gloss pass is on) and turns on its
    /// wet-floor flag above 0.4 (only `UWSwrA1` ships a non-zero value,
    /// 0.5).
    pub wetness: f32,
    pub materials: Vec<TerrainMaterial>,
    pub rules: Vec<BlendRule>,
    pub plants: Vec<Plant>,
}

/// What a palette name resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteEntry {
    Material(usize),
    Rule(usize),
}

const PALETTE_HEADER: usize = 144;
const MATERIAL_FIXED: usize = 16 + 4 + 6 * 16;
const RULE_BYTES: usize = 64;
const PLANT_BYTES: usize = 104;

impl Palette {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let magic = r.u32()?;
        let params = match magic {
            PALETTE_MAGIC_V2 => 25,
            PALETTE_MAGIC_V3 => 29,
            found => {
                return Err(Error::BadMagic {
                    found,
                    expected: PALETTE_MAGIC_V3,
                });
            }
        };
        r.u32()?;
        let name = r.name()?;
        let environment = r.f32()?;
        let wetness = r.f32()?;
        r.bytes(PALETTE_HEADER - r.pos)?;
        let n = r.u32()? as usize;
        r.fits(n, MATERIAL_FIXED + params * 4)?;
        let mut materials = Vec::with_capacity(n);
        for _ in 0..n {
            let name = r.name()?;
            let flags = r.u32()?;
            let mut textures: [String; 6] = Default::default();
            for t in &mut textures {
                *t = r.name()?;
            }
            let params = (0..params).map(|_| r.f32()).collect::<Result<_>>()?;
            materials.push(TerrainMaterial {
                name,
                flags,
                textures,
                params,
            });
        }
        let n = r.u32()? as usize;
        r.fits(n, RULE_BYTES)?;
        let mut rules = Vec::with_capacity(n);
        for _ in 0..n {
            rules.push(BlendRule {
                name: r.name()?,
                flags: r.u32()?,
                kind: r.u32()?,
                a: r.name()?,
                b: r.name()?,
                lo: r.f32()?,
                hi: r.f32()?,
            });
        }
        let mut plants = Vec::new();
        if r.remaining() >= 4 {
            let n = r.u32()? as usize;
            r.fits(n, PLANT_BYTES)?;
            for _ in 0..n {
                let flags = r.u32()?;
                let name = r.name()?;
                let params = (0..(PLANT_BYTES - 20) / 4)
                    .map(|_| r.f32())
                    .collect::<Result<_>>()?;
                plants.push(Plant {
                    flags,
                    name,
                    params,
                });
            }
        }
        Ok(Self {
            name,
            environment,
            wetness,
            materials,
            rules,
            plants,
        })
    }

    /// Resolves a layer or child name; rules shadow materials of the same name.
    pub fn entry(&self, name: &str) -> Option<PaletteEntry> {
        let name = name.trim();
        if let Some(i) = self
            .rules
            .iter()
            .position(|r| r.name.eq_ignore_ascii_case(name))
        {
            return Some(PaletteEntry::Rule(i));
        }
        self.materials
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
            .map(PaletteEntry::Material)
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

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

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn name(&mut self) -> Result<String> {
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
    }

    fn fits(&self, n: usize, item: usize) -> Result<()> {
        match n.checked_mul(item) {
            Some(bytes) if bytes <= self.remaining() => Ok(()),
            _ => Err(Error::Malformed(format!("{n} records do not fit"))),
        }
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

    fn f32s(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn tile(x: u32, y: u32, holes: bool) -> Vec<u8> {
        let mut t = u32s(&[if holes { TILE_HAS_HOLES } else { 0 }, x, y, 0]);
        let mut h = vec![0.0f32; APRON_SIDE * APRON_SIDE];
        h[APRON_SIDE + 1] = 150.0; // vertex (0, 0)
        h[2 * APRON_SIDE + 4] = -25.0; // vertex (3, 1)
        t.extend(f32s(&h));
        if holes {
            let mut m = vec![0u8; HOLES_BYTES];
            m[HOLE_ROW_BYTES * 2] = 0b10; // vertex (1, 2)
            t.extend(m);
        }
        let mut layers = u32s(&[1]);
        layers.extend(name("Grassy"));
        let mut w = vec![255u8; TILE_VERTICES * TILE_VERTICES];
        w[1] = 7;
        layers.extend(w);
        t.extend(u32s(&[LAYERS, layers.len() as u32]));
        t.extend(layers);
        let mut c = u32s(&[TILE, t.len() as u32]);
        c.extend(t);
        c
    }

    #[test]
    fn tiles_heights_holes_and_layers() {
        let mut b = u32s(&[TERRAIN_MAGIC, 0]);
        b.extend(name("seaside"));
        b.extend(f32s(&[20.0]));
        b.extend(u32s(&[2, 1, 2]));
        b.extend(tile(0, 0, false));
        b.extend(tile(1, 0, true));
        let t = Terrain::parse(&b).unwrap();
        assert_eq!(t.palette, "seaside");
        assert_eq!(t.spacing, 0.2);
        assert_eq!(t.tiles.len(), 2);
        let a = &t.tiles[0];
        assert_eq!(a.height(0, 0), 3.0);
        assert_eq!(a.height(3, 1), -0.5);
        assert_eq!(a.height_apron(-1, -1), 0.0);
        assert_eq!(a.height_apron(0, 0), 3.0);
        assert!(!a.is_hole(1, 2));
        assert!(t.tiles[1].is_hole(1, 2));
        assert!(!t.tiles[1].is_hole(0, 2));
        assert_eq!(a.layers[0].name, "grassy");
        assert_eq!(a.layers[0].weights[1], 7);
        assert_eq!(t.tile_centre(a), [-24.0, 0.0]);
        assert_eq!(t.tile_centre(&t.tiles[1]), [24.0, 0.0]);
    }

    #[test]
    fn truncated_terrain_is_an_error() {
        let mut b = u32s(&[TERRAIN_MAGIC, 0]);
        b.extend(name("x"));
        b.extend(f32s(&[20.0]));
        b.extend(u32s(&[1, 1, 1, TILE, 100, 0]));
        assert!(Terrain::parse(&b).is_err());
        assert!(Terrain::parse(&[1, 2, 3]).is_err());
    }

    #[test]
    fn palette_materials_rules_and_lookup() {
        let mut b = u32s(&[PALETTE_MAGIC_V3, 0]);
        b.extend(name("Seaside"));
        b.extend(f32s(&[0.25, 0.5]));
        b.resize(PALETTE_HEADER, 0);
        b.extend(u32s(&[1]));
        b.extend(name("Mat Sand"));
        b.extend(u32s(&[0x1001]));
        b.extend(name("Trn_SandA01D"));
        for _ in 0..5 {
            b.extend(name(""));
        }
        let mut p = vec![0.0f32; 29];
        p[6..10].copy_from_slice(&[0.4, 0.5, 0.6, 1.0]);
        b.extend(f32s(&p));
        b.extend(u32s(&[1]));
        b.extend(name("Coast"));
        b.extend(u32s(&[0, 0]));
        b.extend(name("Mat Sand"));
        b.extend(name("Beach"));
        b.extend(f32s(&[-0.5, 0.75]));
        b.extend(u32s(&[0]));
        let pal = Palette::parse(&b).unwrap();
        assert_eq!(pal.name, "Seaside");
        assert_eq!((pal.environment, pal.wetness), (0.25, 0.5));
        let m = &pal.materials[0];
        assert_eq!(m.texture(TerrainTexture::Diffuse), Some("Trn_SandA01D"));
        assert_eq!(m.texture(TerrainTexture::Normal), None);
        assert_eq!(m.tint(), [0.4, 0.5, 0.6, 1.0]);
        assert_eq!(pal.rules[0].lo, -0.5);
        assert_eq!(pal.entry("coast"), Some(PaletteEntry::Rule(0)));
        assert_eq!(pal.entry("MAT SAND"), Some(PaletteEntry::Material(0)));
        assert_eq!(pal.entry("nothing"), None);
    }
}
