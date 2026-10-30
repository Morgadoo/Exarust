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
use crate::level::{LevelGrid, PropAttachment};
use crate::rfc::{Mesh as ReadMesh, Node as ReadNode, NodeBody, Scene as ReadScene};
use crate::rfc_write::{
    ATTACH_BODY, ATTACH_CLOTH, ATTACH_EMITTER, Attachment, AttachmentData, ExportBody, ExportGrid,
    ExportLight, ExportLink, ExportMesh, ExportSound, ExportTrack, ExportVoxelLight, GiPoints,
    IDENTITY, IndexMap, LightAnimation, MeshPart, NodeData, SceneNode, VertexStream,
};
use crate::rigid_body::RigidBody;
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

/// The friction, group mask and last header word a rigid body's stored
/// header carries. The reader skips them, so a converted body writes the
/// values every shipped body holds (friction 0.75, the rest zero).
pub const BODY_FRICTION: f32 = 0.75;
pub const BODY_GROUP_MASK: u32 = 0;
pub const BODY_LAST: u32 = 0;

/// A level node in the exporter's form.
///
/// The reader stores each node's matrix as the file held it (local when the
/// parent is the root or exported, world otherwise), so the conversion keeps
/// it as both the local and the world placement: the writer stores whichever
/// the node's parent calls for, which is the frame the file held.
pub fn export_node(level: &ReadScene, index: usize) -> Result<SceneNode> {
    let node = level
        .nodes
        .get(index)
        .ok_or_else(|| Error::Malformed(format!("node {index} out of the level")))?;
    let parent = match node.parent {
        Some(0) | None => None,
        Some(p) if p <= level.nodes.len() => Some(p - 1),
        Some(p) => {
            return Err(Error::Malformed(format!(
                "node {index} parent {p} out of the level"
            )));
        }
    };
    let transform = matrix_cm(&node.transform);
    let mut out = SceneNode {
        name: stored_name(&node.name).unwrap_or([0; 16]),
        flags: node.flags,
        flags2: node.flags2,
        parent,
        local: transform,
        world: transform,
        half_extents: node.bounds_half_extents.map(|v| v * 100.0),
        centre: node.bounds_centre.map(|v| v * 100.0),
        radius: node.bounds_radius * 100.0,
        // Every shipped node stores a zero tail word, which the reader drops.
        tail: 0,
        ..Default::default()
    };
    out.shares = match node.instance {
        Some(0) | None => None,
        Some(i) if i <= level.nodes.len() => Some(i - 1),
        Some(i) => {
            return Err(Error::Malformed(format!(
                "node {index} instance {i} out of the level"
            )));
        }
    };
    out.body = export_body(node);
    out.attachment = match &node.body {
        NodeBody::Emitter { kind, size } => Some(Attachment {
            class: ATTACH_EMITTER,
            id: 0,
            data: AttachmentData::Emitter {
                kind: *kind,
                size: *size,
            },
        }),
        _ => node
            .rigid_body
            .as_ref()
            .and_then(|b| b.as_ref().ok())
            .map(|body| Attachment {
                class: body.class,
                id: 0,
                data: AttachmentData::Body(Box::new(export_rigid_body(body))),
            })
            .or_else(|| {
                // The marker's word is zero in every shipped file and the
                // reader drops it.
                node.cloth_marker.then(|| Attachment {
                    class: ATTACH_CLOTH,
                    id: 0,
                    data: AttachmentData::Cloth {
                        marker: 0,
                        particles: Vec::new(),
                    },
                })
            }),
    };
    out.tracks = node
        .tracks()
        .map(|t| ExportTrack {
            on_translation: true,
            // A free track's stored word is not kept (it is any word but 1,
            // 2 and 3); the named modes keep theirs.
            mode: match t.mode {
                crate::key_track::PlayMode::Once => 1,
                crate::key_track::PlayMode::Reverse => 2,
                crate::key_track::PlayMode::Loop => 3,
                crate::key_track::PlayMode::Free => 0,
            },
            duration: t.duration,
            keys: t
                .keys
                .iter()
                .map(|k| {
                    (
                        k.time,
                        [
                            k.position[0] * 100.0,
                            k.position[1] * 100.0,
                            k.position[2] * 100.0,
                        ],
                    )
                })
                .collect(),
        })
        .collect();
    Ok(out)
}

/// A node's body by its stored kind. An emitter is an attachment, not a
/// body, so it leaves the body empty.
fn export_body(node: &ReadNode) -> NodeData {
    match &node.body {
        NodeBody::Mesh(m) => NodeData::Mesh(Box::new(export_mesh(m))),
        NodeBody::Light(l) => NodeData::Light(ExportLight {
            flags: l.flags,
            color: l.color,
            multiplier: l.multiplier,
            intensity: l.intensity,
            radius: l.radius * 100.0,
            // The word the reader skips (the original's +0x228) is zero in
            // the shipped lights.
            unread: 0.0,
            softness: l.softness,
            spot: l.spot.unwrap_or([0.0; 2]),
            animation: l
                .animation
                .map(|a| LightAnimation {
                    amplitude: a.amplitude,
                    speed: a.speed,
                    emissive: a.emissive,
                })
                .unwrap_or_default(),
        }),
        NodeBody::VoxelLight(l) => NodeData::VoxelLight(ExportVoxelLight {
            color: l.color,
            intensity: l.intensity,
            radius: l.radius * 100.0,
        }),
        NodeBody::Camera { target, value } => NodeData::Target {
            node: target.map(|t| t - 1),
            value: *value,
        },
        NodeBody::Instance(_)
        | NodeBody::Sector
        | NodeBody::Emitter { .. }
        | NodeBody::NotMesh
        | NodeBody::Invalid(_) => NodeData::None,
    }
}

/// Three metre rows and a translation become the file's centimetre matrix.
fn matrix_cm(m: &[[f32; 3]; 4]) -> [[f32; 3]; 4] {
    [
        m[0],
        m[1],
        m[2],
        [m[3][0] * 100.0, m[3][1] * 100.0, m[3][2] * 100.0],
    ]
}

/// A rigid body in the exporter's form, in centimetres. Its links are the
/// stored points (the reader keeps no barycentric weights), and the header
/// words the reader skips take the shipped values.
pub fn export_rigid_body(body: &RigidBody) -> ExportBody {
    ExportBody {
        mobility: body.mobility,
        rest: body
            .rest
            .map(|p| [p[0] * 100.0, p[1] * 100.0, p[2] * 100.0]),
        point_masses: body.point_masses,
        friction: BODY_FRICTION,
        group_mask: BODY_GROUP_MASK,
        last: BODY_LAST,
        spheres: body
            .spheres
            .iter()
            .map(|s| {
                [
                    s.centre[0] * 100.0,
                    s.centre[1] * 100.0,
                    s.centre[2] * 100.0,
                    s.radius * 100.0,
                ]
            })
            .collect(),
        weighted_links: false,
        centre: [0.0; 3],
        links: body
            .links
            .iter()
            .map(|l| ExportLink {
                flags: l.flags,
                on_body: [
                    l.on_body[0] * 100.0,
                    l.on_body[1] * 100.0,
                    l.on_body[2] * 100.0,
                    0.0,
                ],
                on_parent: [
                    l.on_parent[0] * 100.0,
                    l.on_parent[1] * 100.0,
                    l.on_parent[2] * 100.0,
                    0.0,
                ],
                parent_rest: None,
                slack: l.slack * 100.0,
                strength: l.strength,
                smoothing: l.smoothing,
                axis: l.axis.unwrap_or([0.0; 3]).map(|v| v * 100.0),
            })
            .collect(),
        sounds: body
            .sounds
            .iter()
            .map(|s| ExportSound {
                id: s.id,
                point: [s.point[0] * 100.0, s.point[1] * 100.0, s.point[2] * 100.0],
                sound: stored_name(&s.sound).unwrap_or([0; 16]),
                volume: s.volume,
                pitch: s.pitch,
            })
            .collect(),
    }
}

/// A level's tile grid in the exporter's form: its sets' names, size and
/// three planes.
pub fn export_grid(grid: &LevelGrid) -> ExportGrid {
    ExportGrid {
        wall_sets: grid
            .wall_sets
            .iter()
            .map(|s| stored_name(s).unwrap_or([0; 16]))
            .collect(),
        floor_sets: grid
            .floor_sets
            .iter()
            .map(|s| stored_name(s).unwrap_or([0; 16]))
            .collect(),
        width: grid.width,
        height: grid.height,
        plane_a: grid.edges.clone(),
        plane_b: grid.plane_b.clone(),
        plane_c: grid.plane_c.clone(),
    }
}

/// The scene nodes of a level's props: each prop becomes a prop node, and its
/// placement's part records become owned child nodes, a body record a body
/// part and a particle record a cloth part, nested as the records nest.
///
/// `first` is the scene index the first prop node takes; props are written in
/// list order after the scene nodes, as the exporter numbers and writes them,
/// so a prop's parent record (a 1-based prop index) becomes `first + p - 1`.
pub fn export_props(level: &ReadScene, first: usize) -> Result<Vec<SceneNode>> {
    let mut out: Vec<SceneNode> = Vec::new();
    for (i, prop) in level.props.iter().enumerate() {
        let parent = match prop.parent {
            Some(0) | None => None,
            Some(p) if p <= level.props.len() => Some(first + p - 1),
            Some(p) => {
                return Err(Error::Malformed(format!(
                    "prop {i} parent {p} out of the level"
                )));
            }
        };
        let index = first + i;
        let transform = matrix_cm(&prop.transform);
        out.push(SceneNode {
            name: stored_name(&prop.object).unwrap_or([0; 16]),
            flags: crate::rfc_write::flags::PROP
                | crate::rfc_write::flags::SAVED
                | (prop.flags & crate::rfc_write::flags::PLACEMENT_MASK),
            parent,
            local: transform,
            world: transform,
            tail: prop.extra,
            ..Default::default()
        });
        for attachment in &prop.attachments {
            match attachment {
                // The exporter writes no placement-tree record; its entries
                // become the part records they describe (a guess: the tree's
                // entries and a part record carry the same name, word and
                // transform).
                PropAttachment::Tree { entries } => {
                    for entry in entries {
                        push_part(&mut out, first, entry, index)?;
                    }
                }
                other => push_part(&mut out, first, other, index)?,
            }
        }
    }
    Ok(out)
}

/// Appends one placement record as an owned child node. A transform record is
/// a body part (the part record carries only its word and matrix, so its data
/// is empty); a particle record is a cloth part. `parent` is the scene index
/// of the node the record belongs to.
fn push_part(
    out: &mut Vec<SceneNode>,
    first: usize,
    record: &PropAttachment,
    parent: usize,
) -> Result<()> {
    let index = first + out.len();
    let (name, word, class, data, transform, children) = match record {
        PropAttachment::Transform {
            name,
            word,
            transform,
            children,
        } => (
            stored_name(name).unwrap_or([0; 16]),
            *word,
            ATTACH_BODY,
            AttachmentData::None,
            matrix_cm(transform),
            Some(children),
        ),
        PropAttachment::MeshPositions {
            name,
            word,
            positions,
        } => (
            stored_name(name).unwrap_or([0; 16]),
            *word,
            ATTACH_CLOTH,
            AttachmentData::Cloth {
                marker: 0,
                particles: positions
                    .iter()
                    .map(|p| [p[0] * 100.0, p[1] * 100.0, p[2] * 100.0, 0.0])
                    .collect(),
            },
            IDENTITY,
            None,
        ),
        PropAttachment::Tree { .. } => {
            return Err(Error::Malformed(
                "a placement tree cannot stand as a part record".into(),
            ));
        }
    };
    out.push(SceneNode {
        name,
        flags: crate::rfc_write::flags::OWNED | crate::rfc_write::flags::SAVED,
        parent: Some(parent),
        local: transform,
        world: transform,
        attachment: Some(Attachment {
            class,
            id: word,
            data,
        }),
        ..Default::default()
    });
    if let Some(children) = children {
        for child in children {
            push_part(out, first, child, index)?;
        }
    }
    Ok(())
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

    use crate::rfc_write::{
        Attachment, AttachmentData, ExportBody, ExportGrid, ExportLight, ExportTrack, IDENTITY,
        NodeData, SceneNode, class, flags,
    };

    fn name16(s: &str) -> [u8; 16] {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n
    }

    fn write(level: &crate::rfc_write::ExportScene) -> Vec<u8> {
        crate::rfc_write::Exporter::new()
            .export(level, &crate::rfc_write::ExportOptions::default(), None)
            .unwrap()
    }

    fn at(t: [f32; 3]) -> [[f32; 3]; 4] {
        let mut m = IDENTITY;
        m[3] = t;
        m
    }

    fn body() -> ExportBody {
        ExportBody {
            mobility: 1.5,
            rest: [
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [0.0, 20.0, 0.0],
                [0.0, 0.0, 40.0],
            ],
            point_masses: [4.0; 4],
            friction: 0.75,
            group_mask: 0,
            last: 0,
            spheres: vec![[1.0, 2.0, 3.0, 4.0]],
            weighted_links: false,
            centre: [0.0; 3],
            links: vec![crate::rfc_write::ExportLink {
                flags: 0,
                on_body: [1.0, 2.0, 3.0, 0.0],
                on_parent: [4.0, 5.0, 6.0, 0.0],
                parent_rest: None,
                slack: 7.0,
                strength: 8.0,
                smoothing: 9.0,
                axis: [0.0; 3],
            }],
            sounds: vec![crate::rfc_write::ExportSound {
                id: 3,
                point: [1.0, 2.0, 3.0],
                sound: name16("stone"),
                volume: 0.5,
                pitch: 1.25,
            }],
        }
    }

    #[test]
    fn nodes_convert_with_their_frames_bounds_and_attachments() {
        let mut mesh = SceneNode {
            name: name16("Crate"),
            flags: class::MESH | flags::SAVED | flags::ATTACHED,
            parent: None,
            local: at([100.0, 200.0, 300.0]),
            world: at([100.0, 200.0, 300.0]),
            half_extents: [10.0, 20.0, 30.0],
            centre: [1.0, 2.0, 3.0],
            radius: 40.0,
            body: NodeData::Mesh(Box::new(export_mesh(&read_mesh()))),
            attachment: Some(Attachment {
                class: 0x1000_0000,
                id: 0,
                data: AttachmentData::Body(Box::new(body())),
            }),
            tracks: vec![ExportTrack {
                on_translation: true,
                mode: 1,
                duration: 3.0,
                keys: vec![(0.0, [0.0, 0.0, 0.0]), (3.0, [0.0, -5.0, 0.0])],
            }],
            ..Default::default()
        };
        mesh.tail = 0;
        let child = SceneNode {
            name: name16("Lid"),
            flags: class::PLAIN | flags::SAVED,
            parent: Some(0),
            local: at([0.0, 50.0, 0.0]),
            world: at([0.0, 50.0, 0.0]),
            ..Default::default()
        };
        let mut light = SceneNode {
            name: name16("Torch"),
            flags: class::LIGHT | flags::SAVED,
            flags2: 0x200,
            body: NodeData::Light(ExportLight {
                flags: 0x1_0010,
                color: [1.0, 0.5, 0.25],
                multiplier: 0.1,
                intensity: 60.0,
                radius: 630.0,
                unread: 0.0,
                softness: 0.5,
                spot: [2.0, 3.0],
                animation: crate::rfc_write::LightAnimation {
                    amplitude: 4.0,
                    speed: 5.0,
                    emissive: 6.0,
                },
            }),
            ..Default::default()
        };
        light.tail = 0;
        let mut camera = SceneNode {
            name: name16("Cam"),
            flags: class::TARGET | flags::SAVED,
            body: NodeData::Target {
                node: Some(0),
                value: 7,
            },
            ..Default::default()
        };
        camera.tail = 0;
        let mut loose_camera = SceneNode {
            name: name16("CamNone"),
            flags: class::TARGET | flags::SAVED,
            body: NodeData::Target {
                node: None,
                value: 8,
            },
            ..Default::default()
        };
        loose_camera.tail = 0;
        let mut emitter = SceneNode {
            name: name16("Fire"),
            flags: class::PLAIN | flags::SAVED | flags::ATTACHED,
            attachment: Some(Attachment {
                class: ATTACH_EMITTER,
                id: 0,
                data: AttachmentData::Emitter {
                    kind: 0x3DD0_E002,
                    size: 2.5,
                },
            }),
            ..Default::default()
        };
        emitter.tail = 0;
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![mesh, child, light, camera, loose_camera, emitter],
            whole_scene: false,
            ..Default::default()
        };
        let parsed = crate::rfc::Scene::parse(&write(&scene)).unwrap();
        assert!(parsed.level_errors.is_empty(), "{:?}", parsed.level_errors);

        let n = export_node(&parsed, 0).unwrap();
        assert_eq!(n.name, name16("Crate"));
        assert_eq!(n.flags & flags::CLASS, class::MESH);
        assert_eq!(n.local[3], [100.0, 200.0, 300.0]);
        // Centimetres survive the metre round trip to float precision.
        for (a, b) in n.half_extents.iter().zip([10.0, 20.0, 30.0]) {
            assert!((a - b).abs() < 1e-3);
        }
        for (a, b) in n.centre.iter().zip([1.0, 2.0, 3.0]) {
            assert!((a - b).abs() < 1e-3);
        }
        assert!((n.radius - 40.0).abs() < 1e-3);
        let Some(Attachment {
            class: 0x1000_0000,
            data: AttachmentData::Body(b),
            ..
        }) = &n.attachment
        else {
            panic!("no body")
        };
        assert_eq!(b.friction, BODY_FRICTION);
        assert!((b.rest[3][2] - 40.0).abs() < 1e-3);
        assert_eq!(b.spheres, [[1.0, 2.0, 3.0, 4.0]]);
        for (a, b) in b.links[0].on_parent.iter().zip([4.0, 5.0, 6.0, 0.0]) {
            assert!((a - b).abs() < 1e-3);
        }
        assert_eq!(b.sounds[0].sound, name16("stone"));
        assert_eq!(n.tracks.len(), 1);
        assert_eq!((n.tracks[0].mode, n.tracks[0].duration), (1, 3.0));
        assert!((n.tracks[0].keys[1].1[1] + 5.0).abs() < 1e-3);
        assert!(matches!(n.body, NodeData::Mesh(_)));

        let lid = export_node(&parsed, 1).unwrap();
        assert_eq!(lid.parent, Some(0));
        assert_eq!(lid.local[3], [0.0, 50.0, 0.0]);

        let t = export_node(&parsed, 2).unwrap();
        let NodeData::Light(l) = &t.body else {
            panic!("not a light")
        };
        assert_eq!(l.radius, 630.0);
        assert_eq!(l.spot, [2.0, 3.0]);
        assert_eq!(l.animation.amplitude, 4.0);
        assert_eq!(l.animation.emissive, 6.0);

        let cam = export_node(&parsed, 3).unwrap();
        assert!(matches!(
            cam.body,
            NodeData::Target {
                node: Some(0),
                value: 7
            }
        ));
        let none = export_node(&parsed, 4).unwrap();
        assert!(matches!(
            none.body,
            NodeData::Target {
                node: None,
                value: 8
            }
        ));

        let fire = export_node(&parsed, 5).unwrap();
        assert!(matches!(
            fire.attachment,
            Some(Attachment {
                data: AttachmentData::Emitter {
                    kind: 0x3DD0_E002,
                    size: 2.5
                },
                ..
            })
        ));
        // A node past the list is an error, never a panic.
        assert!(export_node(&parsed, 6).is_err());
    }

    #[test]
    fn a_cloth_marker_and_a_grid_convert() {
        let mut cloth = SceneNode {
            name: name16("Banner"),
            flags: class::MESH | flags::SAVED | flags::ATTACHED,
            body: NodeData::Mesh(Box::new(export_mesh(&read_mesh()))),
            ..Default::default()
        };
        cloth.tail = 0;
        let grid = ExportGrid {
            wall_sets: vec![name16("gews_a"), name16("gews_b")],
            floor_sets: vec![name16("gefs_a")],
            width: 4,
            height: 4,
            plane_a: (0..16).collect(),
            plane_b: (16..32).collect(),
            plane_c: (32..48).collect(),
        };
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![cloth],
            grid: Some(grid),
            ..Default::default()
        };
        // The writer needs a cloth attachment to write the marker; the
        // conversion reads the marker flag from a parsed level, so give the
        // scene one here.
        let mut scene = scene;
        scene.nodes[0].attachment = Some(Attachment {
            class: ATTACH_CLOTH,
            id: 0,
            data: AttachmentData::Cloth {
                marker: 0,
                particles: vec![[1.0, 2.0, 3.0, 0.0]],
            },
        });
        let parsed = crate::rfc::Scene::parse(&write(&scene)).unwrap();
        let n = export_node(&parsed, 0).unwrap();
        assert!(matches!(
            n.attachment,
            Some(Attachment {
                data: AttachmentData::Cloth { marker: 0, .. },
                ..
            })
        ));
        let g = export_grid(parsed.grid.as_ref().unwrap());
        assert_eq!(g.wall_sets, [name16("gews_a"), name16("gews_b")]);
        assert_eq!(g.floor_sets, [name16("gefs_a")]);
        assert_eq!((g.width, g.height), (4, 4));
        // The reader fills the grid's border cells from the interior (the
        // original loader's rule), so only the interior words return as
        // stored; the other two planes are raw.
        for y in 1..3usize {
            for x in 1..3usize {
                assert_eq!(g.plane_a[y * 4 + x], (y * 4 + x) as u32);
            }
        }
        assert_eq!(g.plane_b, (16..32).collect::<Vec<_>>());
        assert_eq!(g.plane_c, (32..48).collect::<Vec<_>>());
    }

    #[test]
    fn props_convert_with_their_part_records() {
        // A prop whose placement carries a body part with a nested body part
        // and a cloth part, and a child prop.
        let parent = SceneNode {
            name: name16("Table"),
            flags: crate::rfc_write::flags::PROP | crate::rfc_write::flags::SAVED,
            local: at([500.0, 0.0, 250.0]),
            world: at([500.0, 0.0, 250.0]),
            tail: 0x22,
            ..Default::default()
        };
        let mut part = SceneNode {
            name: name16("Top"),
            flags: crate::rfc_write::flags::OWNED | crate::rfc_write::flags::SAVED,
            parent: Some(1),
            local: at([0.0, 70.0, 0.0]),
            world: at([0.0, 70.0, 0.0]),
            attachment: Some(Attachment {
                class: ATTACH_BODY,
                id: 9,
                data: AttachmentData::None,
            }),
            ..Default::default()
        };
        part.tail = 0;
        let nested = SceneNode {
            name: name16("Leg"),
            flags: crate::rfc_write::flags::OWNED | crate::rfc_write::flags::SAVED,
            parent: Some(2),
            local: at([10.0, 0.0, 0.0]),
            world: at([10.0, 0.0, 0.0]),
            attachment: Some(Attachment {
                class: ATTACH_BODY,
                id: 11,
                data: AttachmentData::None,
            }),
            ..Default::default()
        };
        let cloth = SceneNode {
            name: name16("Cloth"),
            flags: crate::rfc_write::flags::OWNED | crate::rfc_write::flags::SAVED,
            parent: Some(1),
            attachment: Some(Attachment {
                class: ATTACH_CLOTH,
                id: 12,
                data: AttachmentData::Cloth {
                    marker: 0,
                    particles: vec![[1.0, 2.0, 3.0, 0.0], [4.0, 5.0, 6.0, 0.0]],
                },
            }),
            ..Default::default()
        };
        let child_prop = SceneNode {
            name: name16("Chair"),
            flags: crate::rfc_write::flags::PROP | crate::rfc_write::flags::SAVED,
            // The table is scene index 1 (the room is 0), so the child prop
            // hangs under it.
            parent: Some(1),
            local: at([100.0, 0.0, 0.0]),
            world: at([100.0, 0.0, 0.0]),
            ..Default::default()
        };
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![plain_level(), parent, part, nested, cloth, child_prop],
            ..Default::default()
        };
        let parsed = crate::rfc::Scene::parse(&write(&scene)).unwrap();
        assert!(parsed.level_errors.is_empty(), "{:?}", parsed.level_errors);
        assert_eq!(parsed.props.len(), 2);

        // One scene node precedes the props, so the first prop is node 1.
        let nodes = export_props(&parsed, 1).unwrap();
        assert_eq!(nodes.len(), 5);
        assert_eq!(nodes[0].name, name16("Table"));
        assert_eq!(nodes[0].parent, None);
        assert_eq!(nodes[0].local[3], [500.0, 0.0, 250.0]);
        assert_eq!(nodes[0].tail, 0x22);
        assert_eq!(
            nodes[0].flags & crate::rfc_write::flags::PROP,
            crate::rfc_write::flags::PROP
        );
        // The body part, its nested body part, then the cloth part.
        let Some(Attachment {
            class: ATTACH_BODY,
            id: 9,
            ..
        }) = &nodes[1].attachment
        else {
            panic!("no body part")
        };
        assert_eq!(nodes[1].parent, Some(1));
        assert_eq!(nodes[1].name, name16("Top"));
        assert_eq!(nodes[1].local[3], [0.0, 70.0, 0.0]);
        let Some(Attachment {
            class: ATTACH_BODY,
            id: 11,
            ..
        }) = &nodes[2].attachment
        else {
            panic!("no nested body part")
        };
        assert_eq!(nodes[2].parent, Some(2));
        let Some(Attachment {
            class: ATTACH_CLOTH,
            data: AttachmentData::Cloth { particles, .. },
            ..
        }) = &nodes[3].attachment
        else {
            panic!("no cloth part")
        };
        assert_eq!(nodes[3].parent, Some(1));
        assert_eq!(particles.len(), 2);
        for (a, b) in particles[1].iter().zip([4.0, 5.0, 6.0, 0.0]) {
            assert!((a - b).abs() < 1e-3);
        }
        // The second prop parents to the first.
        assert_eq!(nodes[4].parent, Some(1));
        assert_eq!(nodes[4].name, name16("Chair"));

        // Writing those nodes again keeps the prop and part records.
        // The converted props were laid out after the level's one node, so
        // the second write keeps that room first.
        let mut replanted = vec![plain_level()];
        replanted.extend(nodes.clone());
        let again = crate::rfc::Scene::parse(&write(&crate::rfc_write::ExportScene {
            nodes: replanted,
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(again.props.len(), 2);
        assert_eq!(again.props[0].object, "Table");
        assert_eq!(again.props[0].extra, 0x22);
        assert_eq!(again.props[0].attachments.len(), 2);
        assert_eq!(again.props[0].attachments[1].name(), "Cloth");

        // A prop parent past the list is an error.
        let mut bad = crate::rfc::Scene::parse(&write(&scene)).unwrap();
        bad.props[1].parent = Some(9);
        assert!(export_props(&bad, 1).is_err());
    }

    /// A plain node so the props are numbered after a scene node.
    fn plain_level() -> SceneNode {
        SceneNode {
            name: name16("Room"),
            flags: class::PLAIN | flags::SAVED,
            ..Default::default()
        }
    }

    /// Every node of the installed level converts and writes back: the same
    /// count, names, parents and placements (to a millimetre).
    #[test]
    fn a_shipped_level_converts_its_nodes() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let scene = crate::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let nodes: Vec<_> = (0..scene.nodes.len())
            .map(|i| export_node(&scene, i).unwrap())
            .collect();
        let out = crate::rfc_write::ExportScene {
            nodes,
            ..Default::default()
        };
        let again = crate::rfc::Scene::parse(&write(&out)).unwrap();
        assert!(again.level_errors.is_empty(), "{:?}", again.level_errors);
        assert_eq!(again.nodes.len(), scene.nodes.len());
        for (a, b) in scene.nodes.iter().zip(&again.nodes) {
            assert_eq!(stored_name(&a.name), stored_name(&b.name));
            assert_eq!(a.parent, b.parent);
            for k in 0..3 {
                assert!((a.transform[3][k] - b.transform[3][k]).abs() < 1e-3);
            }
            assert!((a.bounds_radius - b.bounds_radius).abs() < 1e-3);
        }
    }

    /// Every prop of the installed level converts and writes back: the same
    /// count and objects, and each prop's part records survive the second
    /// write.
    #[test]
    fn a_shipped_level_converts_its_props() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let scene = crate::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let props = export_props(&scene, scene.nodes.len()).unwrap();
        let mut nodes: Vec<_> = (0..scene.nodes.len())
            .map(|i| export_node(&scene, i).unwrap())
            .collect();
        nodes.extend(props);
        let again = crate::rfc::Scene::parse(&write(&crate::rfc_write::ExportScene {
            nodes,
            ..Default::default()
        }))
        .unwrap();
        assert!(again.level_errors.is_empty(), "{:?}", again.level_errors);
        assert_eq!(again.props.len(), scene.props.len());
        for (a, b) in scene.props.iter().zip(&again.props) {
            assert_eq!(a.object, b.object);
            assert_eq!(a.flags & 0xB8_0000, b.flags & 0xB8_0000);
            assert_eq!(a.extra, b.extra);
            for k in 0..3 {
                assert!((a.transform[3][k] - b.transform[3][k]).abs() < 1e-3);
            }
            assert_eq!(a.attachments.len(), b.attachments.len());
        }
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
