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
pub const PALETTE_MAGIC_V1: u32 = 0x3EEF_BD01;
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
    /// Shading parameters in the original's runtime record (29 for every
    /// palette: version 2 files store 25, and the four the loader inserts
    /// at index 20 come out here). The first two look like tiling factors
    /// and 6..10 an RGBA tint.
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
        if magic == PALETTE_MAGIC_V1 {
            return parse_v1(b);
        }
        let (params, version_2) = match magic {
            PALETTE_MAGIC_V2 => (25, true),
            PALETTE_MAGIC_V3 => (29, false),
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
            let mut params = (0..params).map(|_| r.f32()).collect::<Result<Vec<_>>>()?;
            if version_2 {
                // A version 2 record is the runtime record short of four
                // parameters, which the original inserts at index 20:
                // 0.5, 0.3, 0.5, 1.0. The file's parameters 20–24 follow
                // them, so a version 2 material's displacement values sit at
                // indices 24–28 here.
                params.splice(20..20, [0.5, 0.3, 0.5, 1.0]);
            }
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

/// A version 1 palette, upgraded as the original's loader does it. The
/// header is a version 2 header cut short: magic, a word, the name, the two
/// header floats and the material count (no padding to 144 bytes). Then
/// come 152-byte materials, a rule count with 64-byte rules, and no plants:
/// the loader gives the palette three default plants.
///
/// Each old material is moved into the runtime record (REA on the version 1
/// upgrade): its name and flags; textures 1, 2 and 4 into the diffuse,
/// normal and height slots (the third, an old specular map, is dropped); the
/// mask slot from the diffuse name with its last letter turned to `M` when
/// the old height amount is above `f32::EPSILON`; parameter 0 from the first
/// old float and parameter 1 equal to it; 3, 1, 3 and 1.5 for parameters
/// 2–5; the old tint (floats 3–5) times 1.4; the negated old offset (1 when
/// it is below `f32::EPSILON`); 0.1; one less the old float 8; the old
/// floats 9–16; and parameters 20–28 zero. A diffuse map named `???Sh…` (an
/// old shader grass) takes the `Trn_GrasA01` set and tiling 2; a height map
/// named `???r…` or `???R…` zeroes parameters 16 and 17. No shipped palette
/// is version 1, so none of this is exercised by the game's data.
fn parse_v1(b: &[u8]) -> Result<Palette> {
    const MATERIAL_V1: usize = 152;
    let mut r = Reader::new(b);
    r.u32()?; // magic
    r.u32()?;
    let name = r.name()?;
    let environment = r.f32()?;
    let wetness = r.f32()?;
    let count = r.u32()? as usize;
    r.fits(count, MATERIAL_V1)?;
    let mut materials = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = r.bytes(MATERIAL_V1)?;
        let f = |offset: usize| {
            f32::from_le_bytes(raw[offset..offset + 4].try_into().expect("4 bytes"))
        };
        let field =
            |offset: usize| -> [u8; 16] { raw[offset..offset + 16].try_into().expect("16 bytes") };
        // The runtime record's six texture names, as raw 16-byte fields.
        let mut slots = [[0u8; 16]; 6];
        slots[0] = field(0x10);
        slots[1] = field(0x20);
        slots[4] = field(0x40);
        let mut p = [0.0f32; 29];
        p[0] = f(0x50);
        if f(0x54) > f32::EPSILON {
            slots[3] = slots[0];
            if let Some(last) = slots[3].iter().rposition(|&c| c != 0) {
                slots[3][last] = b'M';
            }
        }
        if slots[0][3] == b'S' && slots[0][4] == b'h' {
            let named = |s: &[u8]| {
                let mut n = [0u8; 16];
                n[..s.len()].copy_from_slice(s);
                n
            };
            slots[0] = named(b"Trn_GrasA01D");
            slots[1] = named(b"Trn_GrasA01N");
            slots[2] = named(b"Trn_GrasA01G");
            slots[3] = named(b"Trn_GrasA01M");
            p[0] = 2.0;
        }
        p[1] = p[0];
        p[2] = 3.0;
        p[3] = 1.0;
        p[4] = 3.0;
        p[5] = 1.5;
        p[6] = f(0x5C);
        p[7] = f(0x60);
        p[8] = f(0x64);
        p[9] = -f(0x68);
        if p[9] < f32::EPSILON {
            p[9] = 1.0;
        }
        for v in &mut p[6..9] {
            *v *= 1.4;
        }
        p[10] = 0.1;
        p[11] = 1.0 - f(0x70);
        for (i, v) in p[12..20].iter_mut().enumerate() {
            *v = f(0x74 + 4 * i);
        }
        if matches!(slots[4][3], b'r' | b'R') {
            p[16] = 0.0;
            p[17] = 0.0;
        }
        let text = |raw: &[u8]| -> String {
            let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            raw[..end].iter().map(|&c| c as char).collect()
        };
        materials.push(TerrainMaterial {
            name: text(&raw[..16]),
            flags: u32::from_le_bytes(raw[0x94..0x98].try_into().expect("4 bytes")),
            textures: slots.map(|s| text(&s)),
            params: p.to_vec(),
        });
    }
    let rules_count = r.u32()? as usize;
    r.fits(rules_count, RULE_BYTES)?;
    let mut rules = Vec::with_capacity(rules_count);
    for _ in 0..rules_count {
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
    let plants = ["Trn_GrassA01", "Trn_SPlntA01", "Trn_BPlntA01"]
        .into_iter()
        .map(|name| Plant {
            flags: 1,
            name: name.into(),
            params: default_plant_params(),
        })
        .collect();
    Ok(Palette {
        name,
        environment,
        wetness,
        materials,
        rules,
        plants,
    })
}

/// The 21 parameters each of a version 1 palette's three default plants
/// gets: colours (0.72, 0.56, 0.6, 1) twice, then 0.1, 0.1, 0.35, 1, 0.5,
/// 0.3, 0.6, 2, 0, 1, 0, 1 and a last 0.
fn default_plant_params() -> Vec<f32> {
    vec![
        0.72, 0.56, 0.6, 1.0, 0.72, 0.56, 0.6, 1.0, 0.1, 0.1, 0.35, 1.0, 0.5, 0.3, 0.6, 2.0, 0.0,
        1.0, 0.0, 1.0, 0.0,
    ]
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
    fn a_version_1_palette_is_upgraded() {
        let material = |name_: &str, textures: [&str; 4], f: [f32; 17], flags: u32| {
            let mut m = name(name_);
            for t in textures {
                m.extend(name(t));
            }
            m.extend(f32s(&f));
            m.extend(u32s(&[flags]));
            m
        };
        let mut b = u32s(&[PALETTE_MAGIC_V1, 0]);
        b.extend(name("Old"));
        b.extend(f32s(&[0.25, 0.5]));
        b.extend(u32s(&[3]));
        let mut f = [0.0f32; 17];
        f[0] = 1.5;
        f[1] = 0.5;
        f[3] = 0.1;
        f[4] = 0.2;
        f[5] = 0.5;
        f[6] = -2.0;
        f[8] = 0.25;
        for (i, v) in f[9..].iter_mut().enumerate() {
            *v = 0.5 + i as f32;
        }
        b.extend(material(
            "OldShade",
            ["Dtex", "Ntex", "Gtex", "Htex"],
            f,
            0x11,
        ));
        // An old shader grass (diffuse `???Sh…`) with no height amount and a
        // positive offset; a height map `???r…` zeroes 16 and 17.
        let mut g = [0.0f32; 17];
        g[6] = 3.0;
        g[13] = 7.0;
        b.extend(material("Grass", ["GrsShade", "", "", "Bmpr"], g, 0));
        // No height amount: the mask slot stays empty.
        b.extend(material("Plain", ["Dtex", "", "", ""], [0.0; 17], 0));
        b.extend(u32s(&[0]));
        let pal = Palette::parse(&b).unwrap();
        assert_eq!(pal.name, "Old");
        assert_eq!((pal.environment, pal.wetness), (0.25, 0.5));
        let m = &pal.materials[0];
        assert_eq!((m.name.as_str(), m.flags), ("OldShade", 0x11));
        assert_eq!(
            m.textures.each_ref().map(String::as_str),
            ["Dtex", "Ntex", "", "DteM", "Htex", ""]
        );
        assert_eq!(m.params.len(), 29);
        assert_eq!(&m.params[0..6], &[1.5, 1.5, 3.0, 1.0, 3.0, 1.5]);
        // The tint times 1.4 in f32, the negated offset, 0.1, 1 − 0.25.
        assert_eq!(&m.params[6..9], &[0.1 * 1.4, 0.2 * 1.4, 0.5 * 1.4]);
        assert_eq!(&m.params[9..12], &[2.0, 0.1, 0.75]);
        assert_eq!(&m.params[12..20], &[0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5]);
        assert!(m.params[20..].iter().all(|&v| v == 0.0));
        let g = &pal.materials[1];
        assert_eq!(
            g.textures.each_ref().map(String::as_str),
            [
                "Trn_GrasA01D",
                "Trn_GrasA01N",
                "Trn_GrasA01G",
                "Trn_GrasA01M",
                "Bmpr",
                ""
            ]
        );
        assert_eq!(&g.params[0..2], &[2.0, 2.0]);
        // −3 is below epsilon, so the offset becomes 1.
        assert_eq!(g.params[9], 1.0);
        assert_eq!(&g.params[16..18], &[0.0, 0.0]);
        assert_eq!(pal.materials[2].textures[3], "");
        assert_eq!(pal.plants.len(), 3);
        assert_eq!(
            pal.plants
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["Trn_GrassA01", "Trn_SPlntA01", "Trn_BPlntA01"]
        );
        assert_eq!(pal.plants[0].params.len(), 21);
        assert_eq!(&pal.plants[0].params[..4], &[0.72, 0.56, 0.6, 1.0]);
        // A truncated material list is an error, not a panic.
        assert!(Palette::parse(&b[..60]).is_err());
    }

    #[test]
    fn palette_v2_inserts_the_default_parameters() {
        let mut b = u32s(&[PALETTE_MAGIC_V2, 0]);
        b.extend(name("Old"));
        b.extend(f32s(&[0.0, 0.0]));
        b.resize(PALETTE_HEADER, 0);
        b.extend(u32s(&[1]));
        b.extend(name("Mat Old"));
        b.extend(u32s(&[0]));
        for _ in 0..6 {
            b.extend(name(""));
        }
        b.extend(f32s(&(0..25).map(|i| i as f32).collect::<Vec<_>>()));
        b.extend(u32s(&[0, 0]));
        let pal = Palette::parse(&b).unwrap();
        let p = &pal.materials[0].params;
        assert_eq!(p.len(), 29);
        assert_eq!(p[19], 19.0);
        assert_eq!(&p[20..24], &[0.5, 0.3, 0.5, 1.0]);
        assert_eq!(p[24], 20.0);
        assert_eq!(p[28], 24.0);
        // A version 3 palette stores the runtime record as is.
        let mut b = u32s(&[PALETTE_MAGIC_V3, 0]);
        b.extend(name("New"));
        b.extend(f32s(&[0.0, 0.0]));
        b.resize(PALETTE_HEADER, 0);
        b.extend(u32s(&[1]));
        b.extend(name("Mat New"));
        b.extend(u32s(&[0]));
        for _ in 0..6 {
            b.extend(name(""));
        }
        b.extend(f32s(&(0..29).map(|i| i as f32).collect::<Vec<_>>()));
        b.extend(u32s(&[0, 0]));
        let pal = Palette::parse(&b).unwrap();
        assert_eq!(pal.materials[0].params.len(), 29);
        assert_eq!(pal.materials[0].params[20], 20.0);
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
