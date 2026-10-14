//! Scene nodes and static meshes inside `.rfc` chunk trees.
//!
//! A file starts with `u16 0xAFCF` and a `u16` format version, followed by
//! root chunks. The node list chunk (`0x3D00_0000`) holds a count and one
//! chunk per node; the chunk tag is the node kind. Every node starts with the
//! same header (name, flags, instance reference, parent index, transform,
//! bounds); mesh nodes (`0x3D03`) continue with vertex streams and a topology
//! chunk. Layout and evidence: `docs/formats/RFC.md`.
//!
//! Geometry is returned in metres, right-handed and Y-up. The stored units are
//! centimetres in the same handedness, so the conversion is a uniform scale.

use crate::environment::{ENVIRONMENT, Environment};
use crate::level::{self, LevelGrid, Prop};
use crate::material::Material;
use crate::{Error, Result};

const SANITY: u16 = 0xAFCF;
/// Format versions after this one store one extra word at the start of a mesh body.
const VERSION_WITHOUT_MESH_PREFIX: u16 = 0x3D21;
const CENTIMETRES_TO_METRES: f32 = 0.01;

pub const NODE_LIST: u32 = 0x3D00_0000;
pub const MATERIAL_LIST: u32 = 0xBA00;
pub const NODE_MESH: u32 = 0x3D03;
pub const NODE_LIGHT: u32 = 0x3D06;
const TOPOLOGY: u32 = 0x3DC0;
const TRIANGLES: u32 = 0x3DC1;

/// Index maps are matched to streams by the low 16 bits of their key.
const KEY_POSITIONS: u32 = 0x0001;
const KEY_UVS: u32 = 0x0010;
const KEY_SMOOTHING: u32 = 0x0002;

/// Indices are 16-bit when the unified vertex count is below this.
const WIDE_INDEX_THRESHOLD: usize = 0xFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamFormat {
    Vec3,
    Vec2,
    Rgba8,
    Skin,
}

impl StreamFormat {
    fn from_word(word: u32) -> Option<Self> {
        match (word >> 16) & 0xFF {
            0xF3 => Some(Self::Vec3),
            0xF2 => Some(Self::Vec2),
            0xB4 => Some(Self::Rgba8),
            0xA4 => Some(Self::Skin),
            _ => None,
        }
    }

    fn element_size(self) -> usize {
        match self {
            Self::Vec3 => 12,
            Self::Vec2 => 8,
            Self::Rgba8 => 4,
            Self::Skin => 32,
        }
    }
}

/// A decoded chunk-tree file: its version and its scene nodes.
#[derive(Debug, Clone)]
pub struct Scene {
    pub version: u16,
    pub nodes: Vec<Node>,
    /// Materials defined in the file, by name; undecodable payloads keep the error.
    pub materials: Vec<(String, std::result::Result<Material, String>)>,
    /// Tile grid, in dungeon level files.
    pub grid: Option<LevelGrid>,
    /// Objects placed from the object library, in level files.
    pub props: Vec<Prop>,
    /// Ambient, extent and sun settings, in level files.
    pub environment: Option<Environment>,
    /// Level sections that could not be decoded.
    pub level_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Node {
    /// Chunk tag of the node; [`NODE_MESH`] for meshes.
    pub kind: u32,
    pub name: String,
    pub flags: u32,
    pub flags2: u32,
    /// Index (1-based) of the parent in [`Scene::nodes`]; `None` at the root.
    pub parent: Option<usize>,
    /// Row-major 3×4: three rotation rows, then the translation (metres).
    pub transform: [[f32; 3]; 4],
    /// Bounding sphere in the node's space (metres).
    pub bounds_centre: [f32; 3],
    pub bounds_radius: f32,
    pub body: NodeBody,
}

#[derive(Debug, Clone)]
pub enum NodeBody {
    Mesh(Mesh),
    Light(Light),
    /// The node reuses another object's data; nothing else is stored.
    Instance(u32),
    /// A node kind that is not a mesh (lights, plain nodes, …).
    NotMesh,
    /// A mesh record that could not be decoded.
    Invalid(String),
}

/// An omni light (`0x3D06`). Values are kept raw where their meaning is
/// only partly established.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub flags: u32,
    /// Linear RGB.
    pub color: [f32; 3],
    /// Two factors whose product scales the light (`0.1 × 60`, `1.0 × 1.3`, …).
    pub multiplier: f32,
    pub intensity: f32,
    /// Radius of influence in metres.
    pub radius: f32,
    pub unknown: [f32; 2],
}

/// A triangle mesh with unified vertices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Option<Vec<[f32; 2]>>,
    /// Raw per-vertex RGBA8; channel order is not yet verified.
    pub colors: Option<Vec<[u8; 4]>>,
    pub skin: Option<Vec<SkinWeights>>,
    /// Position-stream element of each vertex: vertices sharing a position
    /// share an id (seams split vertices by UV or normal).
    pub position_ids: Vec<u32>,
    /// Triangle list, counter-clockwise front faces.
    pub indices: Vec<u32>,
    pub groups: Vec<MeshGroup>,
    /// Per-triangle flags (sub-chunk `0x3DC3`): seam classes in the high
    /// byte, apparel cull bits `0x200000`/`0x400000`.
    pub triangle_flags: Option<Vec<u32>>,
    /// The word before the streams in newer files: skinning mode flags.
    pub skin_flags: u32,
}

/// Per-triangle flag sub-chunk of the triangle list.
pub const TRIANGLE_FLAGS: u32 = 0x3DC3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinWeights {
    pub bones: [u32; 4],
    pub weights: [f32; 4],
}

/// A run of triangles sharing one material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshGroup {
    pub material: String,
    /// Offset into [`Mesh::indices`].
    pub first_index: u32,
    pub index_count: u32,
}

impl Scene {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut c = Cursor::new(b);
        if c.u16()? != SANITY {
            return Err(Error::Malformed(
                "not a chunk tree (bad sanity word)".into(),
            ));
        }
        let version = c.u16()?;
        let mut nodes = Vec::new();
        let mut materials = Vec::new();
        let mut grid = None;
        let mut props = Vec::new();
        let mut environment = None;
        let mut level_errors = Vec::new();
        while c.remaining() >= 8 {
            let (tag, mut chunk) = c.chunk()?;
            if tag == level::GRID {
                match level::read_grid(chunk.b) {
                    Ok(g) => grid = Some(g),
                    Err(e) => level_errors.push(format!("grid: {e}")),
                }
            } else if tag == level::PROPS {
                match level::read_props(chunk.b) {
                    Ok(p) => props.extend(p),
                    Err(e) => level_errors.push(format!("props: {e}")),
                }
            } else if tag == ENVIRONMENT {
                match Environment::parse(chunk.b) {
                    Ok(e) => environment = Some(e),
                    Err(e) => level_errors.push(format!("environment: {e}")),
                }
            } else if tag == MATERIAL_LIST {
                if let Err(e) = read_materials(&mut chunk, &mut materials) {
                    materials.push((String::new(), Err(format!("material list: {e}"))));
                }
            } else if tag == NODE_LIST {
                let count = chunk.count(8)?;
                nodes.reserve(count);
                for _ in 0..count {
                    let (kind, node) = chunk.chunk()?;
                    nodes.push(read_node(kind, node, version)?);
                }
            }
        }
        Ok(Self {
            version,
            nodes,
            materials,
            grid,
            props,
            environment,
            level_errors,
        })
    }

    pub fn material(&self, name: &str) -> Option<&Material> {
        self.materials
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .and_then(|(_, m)| m.as_ref().ok())
    }

    pub fn meshes(&self) -> impl Iterator<Item = (&Node, &Mesh)> {
        self.nodes.iter().filter_map(|n| match &n.body {
            NodeBody::Mesh(m) => Some((n, m)),
            _ => None,
        })
    }
}

fn read_materials(
    c: &mut Cursor<'_>,
    out: &mut Vec<(String, std::result::Result<Material, String>)>,
) -> Result<()> {
    let count = c.count(24)?;
    for _ in 0..count {
        let (_, mut record) = c.chunk()?;
        let name = record.name()?;
        let payload = record.bytes(record.remaining())?;
        out.push((name, Material::parse(payload).map_err(|e| e.to_string())));
    }
    Ok(())
}

fn read_node(kind: u32, mut c: Cursor<'_>, version: u16) -> Result<Node> {
    let name = c.name()?;
    let flags = c.u32()?;
    let flags2 = c.u32()?;
    let instance = c.u32()?;
    let parent = c.u32()? as usize;
    let mut transform = [[0.0; 3]; 4];
    for row in &mut transform {
        for v in row.iter_mut() {
            *v = c.f32()?;
        }
    }
    for v in &mut transform[3] {
        *v *= CENTIMETRES_TO_METRES;
    }
    // Bounds: half extents, centre, radius; then one unidentified word.
    c.skip(12)?;
    let bounds_centre = [c.f32()?, c.f32()?, c.f32()?].map(|v| v * CENTIMETRES_TO_METRES);
    let bounds_radius = c.f32()? * CENTIMETRES_TO_METRES;
    c.skip(4)?;
    let body = if instance != 0 {
        NodeBody::Instance(instance)
    } else if kind == NODE_MESH {
        match read_mesh(&mut c, version) {
            Ok(m) => NodeBody::Mesh(m),
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else if kind == NODE_LIGHT {
        match read_light(&mut c) {
            Ok(l) => NodeBody::Light(l),
            Err(e) => NodeBody::Invalid(e.to_string()),
        }
    } else {
        NodeBody::NotMesh
    };
    Ok(Node {
        kind,
        name,
        flags,
        flags2,
        parent: (parent > 0).then_some(parent),
        transform,
        bounds_centre,
        bounds_radius,
        body,
    })
}

fn read_light(c: &mut Cursor<'_>) -> Result<Light> {
    let flags = c.u32()?;
    let color = [c.f32()?, c.f32()?, c.f32()?];
    c.skip(4)?;
    let multiplier = c.f32()?;
    let intensity = c.f32()?;
    let radius = c.f32()? * CENTIMETRES_TO_METRES;
    let unknown = [c.f32()?, c.f32()?];
    Ok(Light {
        flags,
        color,
        multiplier,
        intensity,
        radius,
        unknown,
    })
}

struct Stream<'a> {
    id: u32,
    format: StreamFormat,
    count: usize,
    data: &'a [u8],
}

fn read_mesh(c: &mut Cursor<'_>, version: u16) -> Result<Mesh> {
    let skin_flags = if version > VERSION_WITHOUT_MESH_PREFIX {
        c.u32()?
    } else {
        0
    };
    let stream_count = c.count_minus_one(12)?;
    let mut streams = Vec::with_capacity(stream_count);
    for _ in 0..stream_count {
        let id = c.u32()?;
        let word = c.u32()?;
        let count = c.count_minus_one(1)?;
        let format = StreamFormat::from_word(word)
            .ok_or_else(|| Error::Unsupported(format!("vertex stream format {word:#010x}")))?;
        let data = c.bytes(count * format.element_size())?;
        streams.push(Stream {
            id,
            format,
            count,
            data,
        });
    }

    let (tag, mut topo) = c.chunk()?;
    if tag != TOPOLOGY {
        return Err(Error::Malformed(format!(
            "expected topology chunk, got {tag:#x}"
        )));
    }
    topo.skip(4)?;
    let vertex_count = topo.count_minus_one(1)?;
    let wide = vertex_count >= WIDE_INDEX_THRESHOLD;
    let group_count = topo.count_minus_one(28)?;
    let mut materials = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        topo.skip(4 + 8)?;
        materials.push(topo.name()?);
    }
    let map_count = topo.count_minus_one(4)?;
    let mut maps = Vec::with_capacity(map_count);
    for _ in 0..map_count {
        let key = topo.u32()?;
        maps.push((key, topo.indices(vertex_count, wide)?));
    }

    let mut indices = Vec::new();
    let mut tri_ranges = Vec::new();
    let mut triangle_flags = None;
    if topo.remaining() > 0 {
        let (tag, mut tris) = topo.chunk()?;
        if tag != TRIANGLES {
            return Err(Error::Malformed(format!(
                "expected triangle chunk, got {tag:#x}"
            )));
        }
        tris.skip(4)?;
        let tri_count = tris.count_minus_one(6)?;
        indices = tris.indices(tri_count * 3, wide)?;
        for _ in 0..group_count {
            tri_ranges.push((tris.u32()? as usize, tris.u32()? as usize));
        }
        while tris.remaining() >= 8 {
            let Ok((tag, mut sub)) = tris.chunk() else {
                break;
            };
            if tag == TRIANGLE_FLAGS && sub.remaining() >= tri_count * 4 {
                triangle_flags = Some((0..tri_count).map(|_| sub.u32()).collect::<Result<_>>()?);
            }
        }
        if let Some(&bad) = indices.iter().find(|&&i| i as usize >= vertex_count) {
            return Err(Error::Malformed(format!(
                "triangle index {bad} out of {vertex_count} vertices"
            )));
        }
    }

    let map = |key: u32| maps.iter().find(|(k, _)| k & 0xFFFF == key).map(|(_, m)| m);
    let stream = |id_low: u32, format: StreamFormat| {
        streams
            .iter()
            .find(|s| s.id & 0xFFFF == id_low && s.format == format)
    };
    let gather = |s: &Stream<'_>, m: &[u32]| -> Result<Vec<usize>> {
        m.iter()
            .map(|&i| {
                let i = i as usize;
                if i < s.count {
                    Ok(i)
                } else {
                    Err(Error::Malformed(format!(
                        "vertex map index {i} out of {} elements",
                        s.count
                    )))
                }
            })
            .collect()
    };

    let pos_stream = stream(1, StreamFormat::Vec3)
        .ok_or_else(|| Error::Malformed("mesh has no position stream".into()))?;
    let pos_map =
        map(KEY_POSITIONS).ok_or_else(|| Error::Malformed("mesh has no position map".into()))?;
    let pos_idx = gather(pos_stream, pos_map)?;
    let positions: Vec<[f32; 3]> = pos_idx
        .iter()
        .map(|&i| vec3(pos_stream.data, i).map(|v| v * CENTIMETRES_TO_METRES))
        .collect();

    let uvs = match (stream(0x10, StreamFormat::Vec2), map(KEY_UVS)) {
        (Some(s), Some(m)) => Some(
            gather(s, m)?
                .into_iter()
                .map(|i| {
                    let o = i * 8;
                    [f32_at(s.data, o), f32_at(s.data, o + 4)]
                })
                .collect(),
        ),
        _ => None,
    };
    let colors = stream(1, StreamFormat::Rgba8)
        .map(|s| -> Result<Vec<[u8; 4]>> {
            Ok(gather(s, pos_map)?
                .into_iter()
                .map(|i| s.data[i * 4..i * 4 + 4].try_into().expect("4 bytes"))
                .collect())
        })
        .transpose()?;
    let skin = stream(1, StreamFormat::Skin)
        .map(|s| -> Result<Vec<SkinWeights>> {
            Ok(gather(s, pos_map)?
                .into_iter()
                .map(|i| {
                    let o = i * 32;
                    SkinWeights {
                        bones: std::array::from_fn(|k| u32_at(s.data, o + k * 4)),
                        weights: std::array::from_fn(|k| f32_at(s.data, o + 16 + k * 4)),
                    }
                })
                .collect())
        })
        .transpose()?;

    let smoothing: Vec<u32> = match map(KEY_SMOOTHING) {
        Some(m) => m.clone(),
        None => pos_idx.iter().map(|&i| i as u32).collect(),
    };
    let normals = smooth_normals(&positions, &indices, &smoothing);

    let tri_count = indices.len() / 3;
    let mut groups = Vec::with_capacity(group_count);
    for (material, &(first, last)) in materials.into_iter().zip(&tri_ranges) {
        // An empty group stores `last = first - 1`.
        if last + 1 == first {
            continue;
        }
        if first > last || last >= tri_count {
            return Err(Error::Malformed(format!(
                "group triangle range {first}..={last} out of {tri_count}"
            )));
        }
        groups.push(MeshGroup {
            material,
            first_index: (first * 3) as u32,
            index_count: ((last - first + 1) * 3) as u32,
        });
    }

    Ok(Mesh {
        positions,
        normals,
        uvs,
        colors,
        skin,
        position_ids: pos_idx.iter().map(|&i| i as u32).collect(),
        indices,
        groups,
        triangle_flags,
        skin_flags,
    })
}

/// Area-weighted face normals accumulated over vertices sharing a smoothing key.
fn smooth_normals(positions: &[[f32; 3]], indices: &[u32], keys: &[u32]) -> Vec<[f32; 3]> {
    let key_count = keys.iter().copied().max().map_or(0, |m| m as usize + 1);
    let mut acc = vec![[0.0f32; 3]; key_count];
    for t in indices.as_chunks::<3>().0 {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| positions[i as usize]);
        let u = sub(b, a);
        let v = sub(c, a);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in t {
            let k = keys[i as usize] as usize;
            for d in 0..3 {
                acc[k][d] += n[d];
            }
        }
    }
    keys.iter()
        .map(|&k| {
            let n = acc[k as usize];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 {
                n.map(|x| x / len)
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

fn vec3(b: &[u8], i: usize) -> [f32; 3] {
    let o = i * 12;
    [f32_at(b, o), f32_at(b, o + 4), f32_at(b, o + 8)]
}

/// Bounds-checked little-endian reader over one chunk's payload.
struct Cursor<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
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

    fn skip(&mut self, n: usize) -> Result<()> {
        self.bytes(n).map(drop)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.bytes(2)?.try_into().expect("2 bytes"),
        ))
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

    /// A stored count, rejected when `count * min_item_size` cannot fit.
    fn count(&mut self, min_item_size: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        self.check_fits(n, min_item_size)
    }

    /// A stored "count minus one".
    fn count_minus_one(&mut self, min_item_size: usize) -> Result<usize> {
        let n = (self.u32()? as usize)
            .checked_add(1)
            .ok_or_else(|| Error::Malformed("count overflow".into()))?;
        self.check_fits(n, min_item_size)
    }

    fn check_fits(&self, n: usize, min_item_size: usize) -> Result<usize> {
        match n.checked_mul(min_item_size) {
            Some(bytes) if bytes <= self.remaining() => Ok(n),
            _ => Err(Error::Malformed(format!(
                "count {n} does not fit in {} remaining bytes",
                self.remaining()
            ))),
        }
    }

    fn indices(&mut self, n: usize, wide: bool) -> Result<Vec<u32>> {
        if wide {
            let raw = self.bytes(n * 4)?;
            Ok(raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|x| u32::from_le_bytes(*x))
                .collect())
        } else {
            let raw = self.bytes(n * 2)?;
            Ok(raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|x| u32::from(u16::from_le_bytes(*x)))
                .collect())
        }
    }

    /// The next chunk: its tag and a cursor over its payload.
    fn chunk(&mut self) -> Result<(u32, Cursor<'a>)> {
        let tag = self.u32()?;
        let size = self.u32()? as usize;
        let data = self.bytes(size)?;
        Ok((tag, Cursor::new(data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn f32s(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn name(s: &str) -> Vec<u8> {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n.to_vec()
    }

    /// One mesh node: a unit quad in the XZ plane (centimetres) as two
    /// triangles, two materials, a UV stream.
    fn quad_file(tri_index_override: Option<u16>) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend(name("Quad"));
        body.extend(u32s(&[0x89, 0, 0, 0]));
        body.extend(f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 100.0, 0.0, 0.0,
        ]));
        body.extend(f32s(&[0.0; 7]));
        body.extend(u32s(&[0]));
        body.extend(u32s(&[0])); // version prefix
        body.extend(u32s(&[1])); // two streams
        body.extend(u32s(&[0x1, 0xF3_0001, 3]));
        body.extend(f32s(&[
            0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 100.0, 0.0, -100.0, 0.0, 0.0, -100.0,
        ]));
        body.extend(u32s(&[0x10, 0xF2_0010, 3]));
        body.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]));

        let mut topo = u32s(&[0, 3, 1]); // skip, 4 vertices, 2 groups
        for (first, mat) in [(0u32, "mata"), (0, "matb")] {
            topo.extend(u32s(&[0, first, 3]));
            topo.extend(name(mat));
        }
        topo.extend(u32s(&[2])); // three maps
        for key in [0x0001_0001, 0x0001_0010, KEY_SMOOTHING] {
            topo.extend(u32s(&[key]));
            topo.extend([0u16, 1, 2, 3].iter().flat_map(|x| x.to_le_bytes()));
        }
        let mut tris = u32s(&[0, 1]); // skip, 2 triangles
        let last = tri_index_override.unwrap_or(3);
        tris.extend(
            [0u16, 1, 2, 0, 2, last]
                .iter()
                .flat_map(|x| x.to_le_bytes()),
        );
        tris.extend(u32s(&[0, 0, 1, 1]));
        tris.extend(chunk(0x3DC2, &[0, 0]));
        topo.extend(chunk(TRIANGLES, &tris));
        body.extend(chunk(TOPOLOGY, &topo));

        let mut list = u32s(&[1]);
        list.extend(chunk(NODE_MESH, &body));
        let mut file = vec![0xCF, 0xAF, 0x23, 0x3D];
        file.extend(chunk(NODE_LIST, &list));
        file
    }

    #[test]
    fn decodes_mesh_in_metres() {
        let scene = Scene::parse(&quad_file(None)).unwrap();
        assert_eq!(scene.version, 0x3D23);
        let (node, mesh) = scene.meshes().next().unwrap();
        assert_eq!(node.name, "Quad");
        assert_eq!(node.transform[3], [1.0, 0.0, 0.0]);
        assert_eq!(mesh.positions[2], [1.0, 0.0, -1.0]);
        assert_eq!(mesh.uvs.as_ref().unwrap()[1], [1.0, 0.0]);
        assert_eq!(mesh.indices, [0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.groups.len(), 2);
        assert_eq!(mesh.groups[1].material, "matb");
        assert_eq!(
            (mesh.groups[1].first_index, mesh.groups[1].index_count),
            (3, 3)
        );
        for n in &mesh.normals {
            assert!((n[1] - 1.0).abs() < 1e-6, "quad faces +Y: {n:?}");
        }
    }

    #[test]
    fn out_of_range_index_marks_mesh_invalid() {
        let scene = Scene::parse(&quad_file(Some(9))).unwrap();
        assert!(matches!(&scene.nodes[0].body, NodeBody::Invalid(e) if e.contains("out of 4")));
    }

    #[test]
    fn truncated_file_is_an_error() {
        let mut f = quad_file(None);
        f.truncate(f.len() - 10);
        assert!(Scene::parse(&f).is_err());
    }

    #[test]
    fn impossible_counts_are_rejected() {
        let mut c = Cursor::new(&[0xFF, 0xFF, 0xFF, 0x7F]);
        assert!(c.count_minus_one(4).is_err());
    }
}
