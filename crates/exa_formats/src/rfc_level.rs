//! Turns a loaded level section into the exporter's scene, which is what a
//! save stores as the visited level's section.
//!
//! The structured reader ([`crate::rfc`]) keeps the nodes, meshes, the grid
//! and the placements in metres, but drops a few records the exporter writes
//! back: the material payloads, the level's object and string tables, the
//! environment record, and the lattice spacing of a mesh's GI points (it
//! keeps the points only). [`LevelRaw`] scans those from the section's
//! chunks, so the conversion needs no second parse of the whole file.
//!
//! Conversion starts here: [`LevelRaw::scan`] is the first piece, and the
//! scene builder over a [`crate::rfc::Scene`] follows it.

use crate::chunk::{Chunks, list_children, rfc_chunks};
use crate::rfc::Mesh as ReadMesh;
use crate::rfc_write::{ExportMesh, GiPoints, IndexMap, MeshPart, VertexStream};
use crate::{Error, Result};

/// Root chunks read here.
const MATERIAL_LIST: u32 = 0x0000_BA00;
const MATERIAL: u32 = 0x0000_BA01;
const LOCAL_DATABASE: u32 = 0x0000_DBC0;
const LOCAL_OBJECTS: u32 = 0x0000_DBDB;
const LOCAL_STRINGS: u32 = 0x0000_DBCD;
const ENVIRONMENT: u32 = 0xEC00_DC00;

/// The GI lattice spacing (cm) every shipped mesh with points stores: the
/// points sample a 40 cm lattice (326 point chunks over four shipped levels
/// all carry it). The reader keeps the points but not the word, so the
/// conversion writes this measured value back.
pub const GI_LATTICE_SPACING: u32 = 40;

/// The records of a level section that the structured reader does not keep,
/// as the exporter needs them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelRaw {
    /// The section's material list: each material's 16-byte stored name and
    /// the payload after it, in file order.
    pub materials: Vec<([u8; 16], Vec<u8>)>,
    /// The local object database's record body (sub-chunk `0xDBDB`): its
    /// magic, table length, table and data, as the exporter writes them.
    pub object_database: Option<Vec<u8>>,
    /// The local string table's body (sub-chunk `0xDBCD`).
    pub object_strings: Option<Vec<u8>>,
    /// The environment record's body (`0xEC00DC00`, 0xF8 bytes in the
    /// shipped files).
    pub environment: Option<Vec<u8>>,
}

impl LevelRaw {
    /// Scans a level section (a chunk tree). Unknown root chunks are
    /// skipped, as the reader skips them; a malformed list is an error.
    pub fn scan(section: &[u8]) -> Result<Self> {
        let mut out = Self::default();
        for chunk in rfc_chunks(section)? {
            let chunk = chunk?;
            match chunk.tag {
                MATERIAL_LIST => out.read_materials(&chunk)?,
                LOCAL_DATABASE => out.read_database(&chunk)?,
                ENVIRONMENT => out.environment = Some(chunk.data.to_vec()),
                _ => {}
            }
        }
        Ok(out)
    }
    fn read_materials(&mut self, list: &crate::chunk::Chunk<'_>) -> Result<()> {
        let Some((count, children)) = list_children(list) else {
            return Err(Error::Malformed("material list is not a list".into()));
        };
        self.materials.reserve(count as usize);
        for child in children {
            if child.tag != MATERIAL {
                continue;
            }
            let mut name = [0u8; 16];
            name.copy_from_slice(child.data.get(..16).ok_or(Error::Truncated {
                offset: child.offset,
                needed: 16,
                len: child.data.len(),
            })?);
            self.materials.push((name, child.data[16..].to_vec()));
        }
        Ok(())
    }

    fn read_database(&mut self, db: &crate::chunk::Chunk<'_>) -> Result<()> {
        for sub in Chunks::new(db.data, 0) {
            let sub = sub?;
            match sub.tag {
                LOCAL_OBJECTS => self.object_database = Some(sub.data.to_vec()),
                LOCAL_STRINGS => self.object_strings = Some(sub.data.to_vec()),
                _ => {}
            }
        }
        Ok(())
    }
}

/// The exporter's form of a mesh the reader decoded, in centimetres.
///
/// The reader keeps unified vertices with their stream elements: positions,
/// colours and the skin weights are indexed by [`ReadMesh::position_ids`],
/// the texture coordinates by [`ReadMesh::uv_ids`], and each vertex carries a
/// smoothing word. The conversion writes those elements back as streams with
/// their index maps (keys `0x0001_0001`, `0x0001_0010` and `0x0002`).
///
/// Two losses are deliberate. The stored normal stream is not rebuilt: the
/// reader derives normals from the smoothing map, which is the original's own
/// rule, so a reload computes them again. A part's vertex range is taken from
/// the triangles it covers, because the reader does not keep the stored
/// range; the shipped ranges hold exactly the vertices their triangles use.
pub fn export_mesh(mesh: &ReadMesh) -> ExportMesh {
    let count = mesh.position_count;
    let mut streams = Vec::with_capacity(4);

    let positions = by_element(count, &mesh.position_ids, &mesh.positions, [0.0; 3]);
    let mut data = Vec::with_capacity(positions.len() * 12);
    for p in &positions {
        for v in p {
            data.extend_from_slice(&(v * 100.0).to_le_bytes());
        }
    }
    streams.push(VertexStream {
        id: 1,
        format: 0x00F3_0001,
        count: count as u32,
        data,
    });

    if let Some(uvs) = &mesh.uvs {
        let uv_count = mesh.uv_count;
        if uv_count > 0 && !mesh.uv_ids.is_empty() {
            let elements = by_element(uv_count, &mesh.uv_ids, uvs, [0.0; 2]);
            let mut data = Vec::with_capacity(elements.len() * 8);
            for uv in &elements {
                for v in uv {
                    data.extend_from_slice(&v.to_le_bytes());
                }
            }
            streams.push(VertexStream {
                id: 0x10,
                format: 0x00F2_0010,
                count: uv_count as u32,
                data,
            });
        }
    }

    if let Some(colors) = &mesh.colors {
        let elements = by_element(count, &mesh.position_ids, colors, [0; 4]);
        streams.push(VertexStream {
            id: 1,
            format: 0x00B4_0001,
            count: count as u32,
            data: elements.into_iter().flatten().collect(),
        });
    }

    if let Some(skin) = &mesh.skin {
        let elements = by_element(
            count,
            &mesh.position_ids,
            skin,
            crate::rfc::SkinWeights {
                bones: [0; 4],
                weights: [0.0; 4],
            },
        );
        let mut data = Vec::with_capacity(elements.len() * 32);
        for w in &elements {
            for bone in w.bones {
                data.extend_from_slice(&bone.to_le_bytes());
            }
            for weight in w.weights {
                data.extend_from_slice(&weight.to_le_bytes());
            }
        }
        streams.push(VertexStream {
            id: 1,
            format: 0x00A4_0001,
            count: count as u32,
            data,
        });
    }

    let vertices = mesh.positions.len() as u32;
    let mut parts = Vec::with_capacity(mesh.groups.len());
    for group in &mesh.groups {
        let first = group.first_index as usize / 3;
        let tris = group.index_count as usize / 3;
        let corners = mesh
            .indices
            .get(first * 3..(first + tris) * 3)
            .unwrap_or_default();
        let (low, high) = if corners.is_empty() {
            (0, 0)
        } else {
            corners
                .iter()
                .fold((u32::MAX, 0u32), |(lo, hi), &i| (lo.min(i), hi.max(i)))
        };
        parts.push(MeshPart {
            vertices: [low, high],
            triangles: [first as u32, (first + tris.saturating_sub(1)) as u32],
            material: stored_name(&group.material),
        });
    }

    let mut maps = vec![IndexMap {
        key: 0x0001_0001,
        indices: mesh.position_ids.clone(),
    }];
    if mesh.uvs.is_some() && !mesh.uv_ids.is_empty() {
        maps.push(IndexMap {
            key: 0x0001_0010,
            indices: mesh.uv_ids.clone(),
        });
    }
    maps.push(IndexMap {
        key: 0x0002,
        indices: mesh.smoothing.clone(),
    });

    ExportMesh {
        render_word: mesh.render_flags,
        streams,
        vertex_count: vertices,
        parts,
        maps,
        triangles: mesh.indices.clone(),
        triangle_bytes: mesh.triangle_edges.clone(),
        triangle_flags: mesh.triangle_flags.clone(),
        kept_chunk: None,
        gi_points: (!mesh.gi_points.is_empty()).then(|| GiPoints {
            spacing: GI_LATTICE_SPACING,
            points: mesh
                .gi_points
                .iter()
                .map(|p| [p[0] * 100.0, p[1] * 100.0, p[2] * 100.0])
                .collect(),
        }),
    }
}

/// One stream element per unified vertex, from the first vertex that names
/// the element (the stream is shared, so every vertex naming it has the same
/// value; an element no vertex names stays the fallback).
fn by_element<T: Copy>(count: usize, ids: &[u32], values: &[T], fallback: T) -> Vec<T> {
    let mut out = vec![fallback; count];
    let mut seen = vec![false; count];
    for (vertex, &element) in ids.iter().enumerate() {
        let element = element as usize;
        if element >= count || seen[element] {
            continue;
        }
        if let Some(&value) = values.get(vertex) {
            out[element] = value;
            seen[element] = true;
        }
    }
    out
}

/// A mesh part's stored 16-byte material name; a name of zeros is no
/// material, which the writer stores as `_Default`.
fn stored_name(name: &str) -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    let bytes = name.as_bytes();
    let n = bytes.len().min(16);
    out[..n].copy_from_slice(&bytes[..n]);
    (n > 0 && out[0] != 0).then_some(out)
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

    fn name(s: &str) -> [u8; 16] {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n
    }

    fn section(roots: &[Vec<u8>]) -> Vec<u8> {
        let mut v = crate::chunk::RFC_MAGIC.to_le_bytes().to_vec();
        for r in roots {
            v.extend_from_slice(r);
        }
        v
    }

    #[test]
    fn the_scanner_keeps_materials_databases_and_the_environment() {
        let mut list = 2u32.to_le_bytes().to_vec();
        let mut wood = name("Wood").to_vec();
        wood.extend_from_slice(&[1, 2, 3]);
        list.extend(chunk(MATERIAL, &wood));
        // A non-material child is skipped, as the reader's list walk does.
        list.extend(chunk(0x1234, b"skip"));

        let mut db = chunk(LOCAL_OBJECTS, b"magicdb");
        db.extend(chunk(LOCAL_STRINGS, b"strings"));
        db.extend(chunk(0xDBCE, b"objects"));

        let env = vec![7u8; 0xF8];
        let bytes = section(&[
            chunk(MATERIAL_LIST, &list),
            chunk(LOCAL_DATABASE, &db),
            chunk(ENVIRONMENT, &env),
            chunk(0x9999, b"unknown"),
        ]);
        let raw = LevelRaw::scan(&bytes).unwrap();
        assert_eq!(raw.materials, [(name("Wood"), vec![1, 2, 3])]);
        assert_eq!(raw.object_database.as_deref(), Some(&b"magicdb"[..]));
        assert_eq!(raw.object_strings.as_deref(), Some(&b"strings"[..]));
        assert_eq!(raw.environment.as_deref(), Some(&env[..]));
    }

    #[test]
    fn an_empty_and_a_malformed_section() {
        let empty = LevelRaw::scan(&section(&[])).unwrap();
        assert_eq!(empty, LevelRaw::default());
        assert!(LevelRaw::scan(b"not a tree").is_err());
        // A list that is not shaped like a list is an error, not a panic.
        let bytes = section(&[chunk(MATERIAL_LIST, b"\x02\x00\x00\x00")]);
        assert!(LevelRaw::scan(&bytes).is_err());
        // A material record shorter than its name is truncated.
        let mut list = 1u32.to_le_bytes().to_vec();
        list.extend(chunk(MATERIAL, b"short"));
        let bytes = section(&[chunk(MATERIAL_LIST, &list)]);
        assert!(matches!(
            LevelRaw::scan(&bytes),
            Err(Error::Truncated { needed: 16, .. })
        ));
    }

    /// A quad with a shared position stream and two material groups, as the
    /// reader returns it (metres, per-vertex arrays).
    fn read_mesh() -> ReadMesh {
        ReadMesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
            colors: Some(vec![[1, 2, 3, 4]; 4]),
            skin: None,
            position_ids: vec![0, 1, 2, 3],
            indices: vec![0, 1, 2, 0, 2, 3],
            groups: vec![
                crate::rfc::MeshGroup {
                    material: "Wood".into(),
                    first_index: 0,
                    index_count: 3,
                },
                crate::rfc::MeshGroup {
                    material: String::new(),
                    first_index: 3,
                    index_count: 3,
                },
            ],
            triangle_flags: Some(vec![0x20, 0x40]),
            triangle_edges: Some(vec![1, 2]),
            gi_points: vec![[0.5, 0.0, 0.5]],
            render_flags: 0x0002_00FF,
            position_count: 4,
            smoothing: vec![1, 0, 0, 0],
            uv_ids: vec![0, 1, 2, 3],
            uv_count: 4,
        }
    }

    #[test]
    fn a_read_mesh_converts_to_streams_maps_and_parts() {
        let m = export_mesh(&read_mesh());
        assert_eq!(m.render_word, 0x0002_00FF);
        assert_eq!(m.vertex_count, 4);
        // Positions keep the shared stream's elements in centimetres.
        assert_eq!(
            (m.streams[0].id, m.streams[0].format, m.streams[0].count),
            (1, 0x00F3_0001, 4)
        );
        assert_eq!(
            m.streams[0].data[..12]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|w| f32::from_le_bytes(*w))
                .collect::<Vec<_>>(),
            [0.0, 0.0, 0.0]
        );
        assert_eq!(
            &m.streams[0].data[12..24],
            &[
                100.0f32.to_le_bytes(),
                0.0f32.to_le_bytes(),
                0.0f32.to_le_bytes()
            ]
            .concat()
        );
        // The UV stream and the colour stream follow; the 16-byte name is
        // kept for a named part and `_Default` is written for an empty one.
        assert_eq!((m.streams[1].id, m.streams[1].format), (0x10, 0x00F2_0010));
        assert_eq!((m.streams[2].id, m.streams[2].format), (1, 0x00B4_0001));
        assert_eq!(m.parts.len(), 2);
        assert_eq!(m.parts[0].vertices, [0, 2]);
        assert_eq!(m.parts[1].vertices, [0, 3]);
        assert_eq!(m.parts[0].triangles, [0, 0]);
        assert_eq!(m.parts[1].triangles, [1, 1]);
        assert_eq!(m.parts[0].material, stored_name("Wood"));
        assert!(m.parts[1].material.is_none());
        // Maps: positions, uvs, smoothing, one index per vertex.
        let keys: Vec<_> = m.maps.iter().map(|m| m.key).collect();
        assert_eq!(keys, [0x0001_0001, 0x0001_0010, 0x0002]);
        assert_eq!(m.maps[0].indices, [0, 1, 2, 3]);
        assert_eq!(m.maps[2].indices, [1, 0, 0, 0]);
        // The GI points are written in centimetres at the measured spacing.
        let gi = m.gi_points.unwrap();
        assert_eq!(gi.spacing, 40);
        assert_eq!(gi.points, [[50.0, 0.0, 50.0]]);
        assert_eq!(m.triangle_bytes.as_deref(), Some(&[1u8, 2][..]));
        assert_eq!(m.triangle_flags.as_deref(), Some(&[0x20u32, 0x40][..]));
    }

    #[test]
    fn a_converted_mesh_reads_back_unchanged() {
        use crate::rfc::{NodeBody, Scene};
        use crate::rfc_write::{ExportScene, SceneNode, class};

        let mesh = export_mesh(&read_mesh());
        let scene = ExportScene {
            nodes: vec![SceneNode {
                name: stored_name("Quad").unwrap(),
                flags: class::MESH | crate::rfc_write::flags::SAVED,
                body: crate::rfc_write::NodeData::Mesh(Box::new(mesh)),
                ..Default::default()
            }],
            ..Default::default()
        };
        let bytes = crate::rfc_write::Exporter::new()
            .export(&scene, &crate::rfc_write::ExportOptions::default(), None)
            .unwrap();
        let parsed = Scene::parse(&bytes).unwrap();
        let NodeBody::Mesh(m) = &parsed.nodes[0].body else {
            panic!("not a mesh")
        };
        let again = export_mesh(m);
        assert_eq!(again.vertex_count, 4);
        assert_eq!(again.parts.len(), 2);
        assert_eq!(again.maps.len(), 3);
        assert_eq!(
            again.streams[0].data[..24],
            export_mesh(&read_mesh()).streams[0].data[..24]
        );
        assert_eq!(again.triangles, [0, 1, 2, 0, 2, 3]);
    }

    /// The installed story level converts every mesh and reads back: the
    /// node count matches and each converted mesh keeps its vertices.
    #[test]
    fn a_shipped_level_converts_its_meshes() {
        use crate::rfc::NodeBody;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let bytes = resource.get("exanima01.rfc").unwrap();
        let scene = crate::rfc::Scene::parse(bytes).unwrap();
        let mut meshes = 0;
        for node in &scene.nodes {
            if let NodeBody::Mesh(m) = &node.body {
                let converted = export_mesh(m);
                assert_eq!(converted.vertex_count as usize, m.positions.len());
                assert_eq!(converted.triangles.len(), m.indices.len());
                assert_eq!(converted.parts.len(), m.groups.len());
                if !m.gi_points.is_empty() {
                    assert_eq!(converted.gi_points.unwrap().spacing, GI_LATTICE_SPACING);
                }
                meshes += 1;
            }
        }
        assert!(meshes > 40, "the level has {meshes} meshes");
    }
}
