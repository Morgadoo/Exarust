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
    ExportLight, ExportLink, ExportMesh, ExportScene, ExportSound, ExportTrack, ExportVoxelLight,
    GiPoints, IDENTITY, IndexMap, MeshPart, NodeData, SceneNode, VertexStream,
};
use crate::rigid_body::RigidBody;
use crate::{Error, Result};

/// Root chunks read here: the writer's own tags, so a section the exporter
/// wrote scans back.
use crate::rfc_write::tag::{
    CHARACTER_DB, CHARACTERS, ENVIRONMENT, LOCAL_DATABASE, LOCAL_OBJECTS, LOCAL_STRINGS, MATERIAL,
    MATERIAL_LIST,
};

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
    /// The character database's body inside the characters section
    /// (`0xCECDDB00`): magic, table length, table and data. The structured
    /// reader parses its records and drops the raw body, which a written
    /// section needs because its placements name records by id.
    pub character_database: Option<Vec<u8>>,
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
                CHARACTERS => out.read_characters(&chunk)?,
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

    fn read_characters(&mut self, section: &crate::chunk::Chunk<'_>) -> Result<()> {
        // The section starts with its placement count, then its chunks.
        for sub in Chunks::new(section.data, 4) {
            let sub = sub?;
            if sub.tag == CHARACTER_DB {
                self.character_database = Some(sub.data.to_vec());
            }
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
            // The writer stores `_Default` for a part with no material, so
            // that name reads back as no material, not as a phantom one the
            // library would be asked for.
            material: stored_name(&group.material)
                .filter(|name| name != &crate::rfc_write::DEFAULT_MATERIAL),
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
        // The file masks the saved bit out of the stored flags, so a node
        // read back carries the class but not 0x40000; the exporter's walk
        // skips every non-plain node without it. A node that came from a
        // stored section is one the export keeps.
        flags: node.flags | crate::rfc_write::flags::SAVED,
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
            // The word the reader skips is zero in the shipped lights.
            unread: 0.0,
            softness: l.softness,
            spot: l.spot.unwrap_or([0.0; 2]),
            animation: l.animation,
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
/// `first` is the scene index the first prop node takes. The part nodes follow
/// their prop, so the props are not contiguous: a parent record (a 1-based
/// prop index) is resolved through [`prop_nodes`], the scene index each prop
/// node actually takes.
pub fn export_props(level: &ReadScene, first: usize) -> Result<Vec<SceneNode>> {
    let indices = prop_nodes(level, first);
    let mut out: Vec<SceneNode> = Vec::new();
    for (i, prop) in level.props.iter().enumerate() {
        let parent = match prop.parent {
            Some(0) | None => None,
            Some(p) if p <= indices.len() => Some(indices[p - 1]),
            Some(p) => {
                return Err(Error::Malformed(format!(
                    "prop {i} parent {p} out of the level"
                )));
            }
        };
        let index = indices[i];
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

/// The scene index each prop node takes when the props start at `first`: a
/// prop's own node, then the part nodes its records add.
pub fn prop_nodes(level: &ReadScene, first: usize) -> Vec<usize> {
    let mut at = first;
    level
        .props
        .iter()
        .map(|prop| {
            let index = at;
            at += 1 + attachments_nodes(&prop.attachments);
            index
        })
        .collect()
}

/// How many nodes a placement's records add below their owner: one per
/// transform or particle record, its children included; a placement tree
/// adds one per entry it flattens.
fn attachments_nodes(list: &[PropAttachment]) -> usize {
    list.iter()
        .map(|record| match record {
            PropAttachment::Transform { children, .. } => 1 + attachments_nodes(children),
            PropAttachment::MeshPositions { .. } => 1,
            PropAttachment::Tree { entries } => attachments_nodes(entries),
        })
        .sum()
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

/// The scene nodes of a level's placed objects, written after the scene nodes
/// and the props as the exporter writes them: `nodes + props` is the index of
/// the first object node.
///
/// A placed object's parent word is the exporter's node number, which follows
/// the export's own numbering: 1..n the scene nodes, n..n+p the props and from
/// n+p the objects. A number that names both the last scene node and the first
/// prop is read as the node, the collision the original's numbering has. The
/// word version 0x3D22 files store after the second word is not written at
/// version 0x3D23, so it is dropped.
pub fn export_objects(level: &ReadScene, first: usize) -> Result<Vec<SceneNode>> {
    let nodes = level.nodes.len();
    let props = prop_nodes(level, nodes);
    let indices = object_nodes(level, first);
    let mut out: Vec<SceneNode> = Vec::new();
    for (i, object) in level.placed_objects.iter().enumerate() {
        let parent = parent_node(object.slot, nodes, &props, &indices);
        let index = indices[i];
        let transform = matrix_cm(&object.transform);
        out.push(SceneNode {
            // A placed object's record stores no name.
            flags: crate::rfc_write::flags::OBJECT
                | crate::rfc_write::flags::SAVED
                | (object.object_flags & crate::rfc_write::flags::PLACEMENT_MASK),
            parent,
            local: transform,
            world: transform,
            tail: object.second_word,
            object: Some(crate::rfc_write::ObjectIds {
                id: object.id,
                placement: object.flags,
            }),
            ..Default::default()
        });
        for attachment in &object.attachments {
            match attachment {
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

/// The scene index each placed-object node takes when the objects start at
/// `first`, part nodes included.
pub fn object_nodes(level: &ReadScene, first: usize) -> Vec<usize> {
    let mut at = first;
    level
        .placed_objects
        .iter()
        .map(|object| {
            let index = at;
            at += 1 + attachments_nodes(&object.attachments);
            index
        })
        .collect()
}

/// A stored parent number as a scene index in the export's numbering, `None`
/// for the world and for a number the level no longer names. The number names
/// the scene nodes (1..n), then the props (n..n+p) and then the objects (from
/// n+p), each through its node's actual scene index, because a node's part
/// children sit between it and the next one. A number that names both the
/// last scene node and the first prop is read as the node, the collision the
/// original's numbering has.
fn parent_node(number: i32, nodes: usize, props: &[usize], objects: &[usize]) -> Option<usize> {
    if number <= 0 {
        return None;
    }
    let number = number as usize;
    if number <= nodes {
        Some(number - 1)
    } else if number < nodes + props.len() {
        // A prop's number is the node count plus its index.
        props.get(number - nodes).copied()
    } else {
        objects.get(number - nodes - props.len()).copied()
    }
}

/// The whole exporter scene of a loaded level section: its scene nodes, the
/// props and placed objects after them, its placed characters, the tile grid,
/// the section's material library, the local object database and its strings,
/// the character database and the environment record.
///
/// The scene is the game's whole scene for an export of the current level, so
/// the environment is written; the shared material libraries are left empty,
/// which lists every material the meshes use and writes the ones the level
/// itself holds (the rest fall to the shared libraries by name, as the
/// original's lookup does when the level's library lacks them).
pub fn level_scene(level: &ReadScene, raw: &LevelRaw) -> Result<ExportScene> {
    let mut nodes = Vec::with_capacity(level.nodes.len());
    for i in 0..level.nodes.len() {
        nodes.push(export_node(level, i)?);
    }
    let prop_start = nodes.len();
    nodes.extend(export_props(level, prop_start)?);
    let object_start = nodes.len();
    nodes.extend(export_objects(level, object_start)?);
    nodes.extend(export_characters(level)?);
    Ok(ExportScene {
        nodes,
        grid: level.grid.as_ref().map(export_grid),
        shared_materials: Vec::new(),
        material_library: raw.materials.clone(),
        object_db: raw
            .object_database
            .as_deref()
            .and_then(|body| rdb_image(body, crate::rfc_write::OBJECT_DB_MAGIC)),
        object_strings: raw.object_strings.clone().unwrap_or_default(),
        character_db: raw
            .character_database
            .as_deref()
            .and_then(|body| rdb_image(body, crate::rfc_write::CHARACTER_DB_MAGIC)),
        environment: raw.environment.clone(),
        whole_scene: true,
    })
}

/// A database body (magic, table length, table, data) as the writer's image.
/// A body of another magic or one too short for its table is no database.
fn rdb_image(body: &[u8], magic: u32) -> Option<crate::rfc_write::RdbImage> {
    let stored = u32::from_le_bytes(body.get(..4)?.try_into().ok()?);
    if stored != magic {
        return None;
    }
    let len = u32::from_le_bytes(body.get(4..8)?.try_into().ok()?) as usize;
    if len < 16 || !len.is_multiple_of(16) {
        return None;
    }
    let table = body.get(8..8 + len)?.to_vec();
    Some(crate::rfc_write::RdbImage {
        count: (len / 16 - 1) as u32,
        table,
        data: body.get(8 + len..)?.to_vec(),
    })
}

/// The scene nodes of a level's placed characters, in placement order.
///
/// A placement stores its feet position, facing and condition, and the
/// exporter writes the body's position words as (x, floor, z): the first and
/// third position values, with the second kept as the floor height. A saved
/// bone tree becomes the exporter's pose tree in centimetres; a legacy flat
/// pose has no tree to write back, so it is an error rather than a guess at a
/// hierarchy. The AI home is stored with the home kind so the writer emits it:
/// the reader does not keep the record's kind word (a stand-in, labelled).
pub fn export_characters(level: &ReadScene) -> Result<Vec<SceneNode>> {
    let mut out = Vec::new();
    for placement in level
        .characters
        .iter()
        .flat_map(|chars| chars.placements.iter())
    {
        let name = level
            .characters
            .as_ref()
            .and_then(|chars| chars.record(placement.id))
            .and_then(|record| stored_name(&record.name))
            .unwrap_or([0; 16]);
        let pose = pose_tree(&placement.pose)?;
        let parts = match &placement.pose {
            crate::characters::SavedPose::Flat(frames) => frames.iter().map(matrix_cm).collect(),
            _ => Vec::new(),
        };
        let motion = placement.motion.unwrap_or(crate::characters::MotionState {
            state: 0,
            value: 0.0,
        });
        let ai = Some(crate::rfc_write::ExportAi {
            motile: true,
            skip_home: false,
            mode: if placement.motion.is_some() { 1 } else { 0 },
            home: placement
                .ai_home
                .map(|home| home.position.map(|v| v * 100.0))
                .unwrap_or([0.0; 3]),
            home_heading: placement.ai_home.map_or(0.0, |home| home.heading),
        });
        out.push(SceneNode {
            name,
            // A character is not a class-1 node: it is written with its body.
            flags: crate::rfc_write::flags::CHARACTER | crate::rfc_write::flags::SAVED,
            parent: None,
            character: Some(crate::rfc_write::ExportCharacter {
                record: placement.id,
                // The record's kind word is not kept; a placement that
                // carries a home gets the home kind so the writer stores it.
                record_kind: if placement.ai_home.is_some() {
                    crate::rfc_write::RECORD_KIND_HOME
                } else {
                    0
                },
                body_position: [
                    placement.position[0] * 100.0,
                    0.0,
                    placement.position[2] * 100.0,
                ],
                floor_height: placement.position[1] * 100.0,
                heading: placement.yaw,
                condition: placement.condition,
                motion_state: motion.state,
                motion_value: motion.value,
                pose,
                parts,
                ai,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    Ok(out)
}

/// The exporter's pose tree from the placement's saved pose.
fn pose_tree(pose: &crate::characters::SavedPose) -> Result<Option<crate::rfc_write::PoseNode>> {
    match pose {
        crate::characters::SavedPose::None => Ok(None),
        crate::characters::SavedPose::Tree(records) => {
            let mut at = 0;
            let node = pose_node(records, &mut at)?;
            if at != records.len() {
                return Err(Error::Malformed(format!(
                    "{} pose records after the tree",
                    records.len() - at
                )));
            }
            Ok(Some(node))
        }
        crate::characters::SavedPose::Flat(_) => Err(Error::Unsupported(
            "a legacy flat pose has no bone tree to write".into(),
        )),
    }
}

/// One pose record and its children, in the reader's depth-first order.
fn pose_node(
    records: &[crate::characters::PoseRecord],
    at: &mut usize,
) -> Result<crate::rfc_write::PoseNode> {
    let record = records
        .get(*at)
        .ok_or_else(|| Error::Malformed("a pose tree ends before its children".into()))?;
    *at += 1;
    let mut children = Vec::with_capacity(record.children as usize);
    for _ in 0..record.children {
        children.push(pose_node(records, at)?);
    }
    Ok(crate::rfc_write::PoseNode {
        matrix: matrix_cm(&record.frame),
        word: record.word,
        class: crate::rfc_write::POSE_CHILD_CLASS,
        kind: 0,
        children,
    })
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
                animation: Some(crate::rfc_write::LightAnimation {
                    amplitude: 4.0,
                    speed: 5.0,
                    emissive: 6.0,
                }),
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
        assert_eq!(l.animation.unwrap().amplitude, 4.0);
        assert_eq!(l.animation.unwrap().emissive, 6.0);

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

    fn placed(id: u32, placement: u32, object_flags: u32) -> SceneNode {
        SceneNode {
            name: name16("obj"),
            flags: crate::rfc_write::flags::OBJECT
                | crate::rfc_write::flags::SAVED
                | (object_flags & crate::rfc_write::flags::PLACEMENT_MASK),
            object: Some(crate::rfc_write::ObjectIds { id, placement }),
            ..Default::default()
        }
    }

    #[test]
    fn placed_objects_convert_with_their_id_flags_parent_and_parts() {
        let a = SceneNode {
            name: name16("A"),
            flags: class::PLAIN | flags::SAVED,
            ..Default::default()
        };
        let b = SceneNode {
            name: name16("B"),
            flags: class::PLAIN | flags::SAVED,
            ..Default::default()
        };
        let prop = SceneNode {
            name: name16("Table"),
            flags: crate::rfc_write::flags::PROP | crate::rfc_write::flags::SAVED,
            ..Default::default()
        };
        let mut first = placed(0xC000_0007, 0x0012_0000, 0x0008_0000);
        first.parent = Some(0);
        first.local = at([300.0, 0.0, 400.0]);
        first.world = first.local;
        first.tail = 0x5A;
        // A placed object's part records belong to its owned children.
        let body_part = SceneNode {
            name: name16("Top"),
            flags: crate::rfc_write::flags::OWNED | crate::rfc_write::flags::SAVED,
            parent: Some(3),
            attachment: Some(Attachment {
                class: ATTACH_BODY,
                id: 4,
                data: AttachmentData::None,
            }),
            ..Default::default()
        };
        let mut second = placed(0xC000_0008, 0x0034_0000, 0);
        second.parent = Some(3);
        second.local = at([310.0, 0.0, 400.0]);
        second.world = second.local;
        let cloth_part = SceneNode {
            name: name16("Cloth"),
            flags: crate::rfc_write::flags::OWNED | crate::rfc_write::flags::SAVED,
            parent: Some(5),
            attachment: Some(Attachment {
                class: ATTACH_CLOTH,
                id: 5,
                data: AttachmentData::Cloth {
                    marker: 0,
                    particles: vec![[7.0, 8.0, 9.0, 0.0]],
                },
            }),
            ..Default::default()
        };
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![a, b, prop, first, body_part, second, cloth_part],
            ..Default::default()
        };
        let parsed = crate::rfc::Scene::parse(&write(&scene)).unwrap();
        assert_eq!(parsed.placed_objects.len(), 2);

        let objects = export_objects(&parsed, 3).unwrap();
        // Each object node is followed by its part nodes: [first, its body,
        // second, its cloth].
        assert_eq!(objects.len(), 4);
        let ids = objects[0].object.unwrap();
        assert_eq!(ids.id, 0xC000_0007);
        assert_eq!(ids.placement, 0x0012_0000);
        assert_eq!(
            objects[0].flags & crate::rfc_write::flags::PLACEMENT_MASK,
            0x0008_0000
        );
        assert_eq!(objects[0].parent, Some(0));
        assert_eq!(objects[0].local[3], [300.0, 0.0, 400.0]);
        assert_eq!(objects[0].tail, 0x5A);
        assert!(matches!(
            objects[1].attachment,
            Some(Attachment {
                class: ATTACH_BODY,
                id: 4,
                ..
            })
        ));
        assert_eq!(objects[1].parent, Some(3));
        // The second object parents to the first (its number is 3 with two
        // nodes and one prop) and keeps its cloth part. It sits after the
        // first object's body part, at scene index 5.
        assert_eq!(
            objects[2].flags & crate::rfc_write::flags::OBJECT,
            crate::rfc_write::flags::OBJECT
        );
        assert_eq!(objects[2].object.unwrap().id, 0xC000_0008);
        assert_eq!(objects[2].parent, Some(3));
        assert!(matches!(
            objects[3].attachment,
            Some(Attachment {
                class: ATTACH_CLOTH,
                id: 5,
                ..
            })
        ));
        assert_eq!(objects[3].parent, Some(5));

        // Writing nodes, the prop and the objects again keeps every record.
        let mut replanted = vec![plain_level(), plain_level(), plain_prop()];
        replanted.extend(objects.clone());
        let again = crate::rfc::Scene::parse(&write(&crate::rfc_write::ExportScene {
            nodes: replanted,
            ..Default::default()
        }))
        .unwrap();
        assert!(again.level_errors.is_empty(), "{:?}", again.level_errors);
        assert_eq!(again.placed_objects.len(), 2);
        assert_eq!(again.placed_objects[0].id, 0xC000_0007);
        assert_eq!(again.placed_objects[0].second_word, 0x5A);
        assert_eq!(again.placed_objects[0].attachments.len(), 1);
        assert_eq!(again.placed_objects[1].attachments[0].name(), "Cloth");
        for (a, b) in parsed.placed_objects.iter().zip(&again.placed_objects) {
            assert_eq!(
                (a.id, a.flags, a.object_flags),
                (b.id, b.flags, b.object_flags)
            );
            for k in 0..3 {
                assert!((a.transform[3][k] - b.transform[3][k]).abs() < 1e-3);
            }
        }
    }

    fn plain_prop() -> SceneNode {
        SceneNode {
            name: name16("Table"),
            flags: crate::rfc_write::flags::PROP | crate::rfc_write::flags::SAVED,
            ..Default::default()
        }
    }

    #[test]
    fn a_parent_number_maps_through_the_export_numbering() {
        // Two nodes; the prop node is 2 and the object nodes 3 and 4, as the
        // props add no parts here.
        let props = [2];
        let objects = [3, 4];
        assert_eq!(parent_node(0, 2, &props, &objects), None);
        assert_eq!(parent_node(1, 2, &props, &objects), Some(0));
        assert_eq!(parent_node(2, 2, &props, &objects), Some(1));
        assert_eq!(parent_node(3, 2, &props, &objects), Some(3));
        assert_eq!(parent_node(4, 2, &props, &objects), Some(4));
        // A number past the level's nodes is no parent, not a panic.
        assert_eq!(parent_node(9, 2, &props, &objects), None);
        // Part nodes between the owners move the later indices.
        let props = [2, 5];
        let objects = [7];
        assert_eq!(parent_node(2, 2, &props, &objects), Some(1));
        assert_eq!(parent_node(3, 2, &props, &objects), Some(5));
        assert_eq!(parent_node(4, 2, &props, &objects), Some(7));
    }

    /// Every placed object of the installed level converts and writes back:
    /// the same count, ids, placement words, masked flags and placements.
    #[test]
    fn a_shipped_level_converts_its_objects() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let scene = crate::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        if scene.placed_objects.is_empty() {
            return;
        }
        let prop_start = scene.nodes.len();
        let object_start =
            prop_start + prop_nodes(&scene, prop_start).len() + attachments_nodes_of(&scene);
        let objects = export_objects(&scene, object_start).unwrap();
        let mut nodes: Vec<_> = (0..scene.nodes.len())
            .map(|i| export_node(&scene, i).unwrap())
            .collect();
        nodes.extend(export_props(&scene, prop_start).unwrap());
        nodes.extend(objects);
        let again = crate::rfc::Scene::parse(&write(&crate::rfc_write::ExportScene {
            nodes,
            ..Default::default()
        }))
        .unwrap();
        assert!(again.level_errors.is_empty(), "{:?}", again.level_errors);
        assert_eq!(again.placed_objects.len(), scene.placed_objects.len());
        for (a, b) in scene.placed_objects.iter().zip(&again.placed_objects) {
            assert_eq!(
                (a.id, a.flags, a.object_flags),
                (b.id, b.flags, b.object_flags)
            );
            assert_eq!(a.second_word, b.second_word);
            for k in 0..3 {
                assert!((a.transform[3][k] - b.transform[3][k]).abs() < 1e-3);
            }
            assert_eq!(a.attachments.len(), b.attachments.len());
        }
    }

    /// The installed level's props contribute the part nodes its records add.
    fn attachments_nodes_of(scene: &crate::rfc::Scene) -> usize {
        scene
            .props
            .iter()
            .map(|p| 1 + attachments_nodes(&p.attachments))
            .sum()
    }

    fn pose_node(
        word: u32,
        children: Vec<crate::rfc_write::PoseNode>,
    ) -> crate::rfc_write::PoseNode {
        crate::rfc_write::PoseNode {
            matrix: at([0.0, 50.0, 0.0]),
            word,
            class: crate::rfc_write::POSE_CHILD_CLASS,
            kind: 0,
            children,
        }
    }

    #[test]
    fn characters_convert_with_their_pose_motion_and_home() {
        let mut dead = SceneNode {
            name: name16("Corpse"),
            flags: crate::rfc_write::flags::CHARACTER | crate::rfc_write::flags::SAVED,
            character: Some(crate::rfc_write::ExportCharacter {
                record: 0x4000_0003,
                body_position: [500.0, 0.0, -200.0],
                floor_height: 12.0,
                heading: 1.5,
                condition: crate::rfc_write::DEAD,
                pose: Some(pose_node(5, vec![pose_node(7, Vec::new())])),
                ..Default::default()
            }),
            ..Default::default()
        };
        dead.tail = 0;
        let mut live = SceneNode {
            name: name16("Bandit"),
            flags: crate::rfc_write::flags::CHARACTER | crate::rfc_write::flags::SAVED,
            character: Some(crate::rfc_write::ExportCharacter {
                record: 0x4000_0004,
                // The AI home is written only for the home record kind.
                record_kind: crate::rfc_write::RECORD_KIND_HOME,
                body_position: [600.0, 0.0, 100.0],
                floor_height: 20.0,
                heading: -0.5,
                condition: 0,
                motion_state: 6,
                motion_value: 0.25,
                // AI mode 1 stores the pose and the motion state together.
                pose: Some(pose_node(9, Vec::new())),
                ai: Some(crate::rfc_write::ExportAi {
                    motile: true,
                    skip_home: false,
                    mode: 1,
                    home: [650.0, 20.0, 120.0],
                    home_heading: 0.75,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        live.tail = 0;
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![plain_level(), dead, live],
            ..Default::default()
        };
        // The level export writes a dead body's tree and a save the AI home.
        let options = crate::rfc_write::ExportOptions {
            save_mode: true,
            ..Default::default()
        };
        let bytes = crate::rfc_write::Exporter::new()
            .export(&scene, &options, None)
            .unwrap();
        let parsed = crate::rfc::Scene::parse(&bytes).unwrap();
        assert!(parsed.level_errors.is_empty(), "{:?}", parsed.level_errors);
        let chars = parsed.characters.as_ref().unwrap();
        assert_eq!(chars.placements.len(), 2);

        let nodes = export_characters(&parsed).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(
            nodes[0].flags & crate::rfc_write::flags::CHARACTER,
            crate::rfc_write::flags::CHARACTER
        );
        let first = nodes[0].character.as_ref().unwrap();
        assert_eq!(first.record, 0x4000_0003);
        assert_eq!(first.body_position, [500.0, 0.0, -200.0]);
        assert_eq!(first.floor_height, 12.0);
        assert_eq!(first.heading, 1.5);
        assert_eq!(first.condition, crate::rfc_write::DEAD);
        let pose = first.pose.as_ref().unwrap();
        assert_eq!(pose.word, 5);
        assert_eq!(pose.children.len(), 1);
        assert_eq!(pose.children[0].word, 7);
        assert_eq!(pose.children[0].matrix[3], [0.0, 50.0, 0.0]);
        let second = nodes[1].character.as_ref().unwrap();
        assert_eq!((second.motion_state, second.motion_value), (6, 0.25));
        assert_eq!(second.pose.as_ref().unwrap().word, 9);
        let ai = second.ai.as_ref().unwrap();
        assert_eq!(ai.mode, 1);
        for (a, b) in ai.home.iter().zip([650.0, 20.0, 120.0]) {
            assert!((a - b).abs() < 1e-3);
        }
        assert_eq!(ai.home_heading, 0.75);
        assert_eq!(second.record_kind, crate::rfc_write::RECORD_KIND_HOME);

        // Writing the converted nodes again as a save keeps the placements,
        // their pose, the motion state and the home.
        let again = crate::rfc::Scene::parse(
            &crate::rfc_write::Exporter::new()
                .export(
                    &crate::rfc_write::ExportScene {
                        nodes: nodes.clone(),
                        ..Default::default()
                    },
                    &crate::rfc_write::ExportOptions {
                        save_mode: true,
                        ..Default::default()
                    },
                    None,
                )
                .unwrap(),
        )
        .unwrap();
        let again_chars = again.characters.as_ref().unwrap();
        assert_eq!(again_chars.placements.len(), 2);
        let a = &again_chars.placements[0];
        assert_eq!(a.id, 0x4000_0003);
        assert_eq!(a.condition, crate::rfc_write::DEAD);
        assert_eq!(a.position, [5.0, 0.12, -2.0]);
        assert_eq!(a.yaw, 1.5);
        assert!(matches!(a.pose, crate::characters::SavedPose::Tree(_)));
        let b = &again_chars.placements[1];
        assert_eq!(b.id, 0x4000_0004);
        assert_eq!(b.motion.unwrap().state, 6);
        assert_eq!(b.ai_home.unwrap().heading, 0.75);
    }

    #[test]
    fn a_flat_pose_has_no_tree_to_write() {
        let flat = crate::characters::SavedPose::Flat(vec![crate::rfc_write::IDENTITY]);
        assert!(matches!(pose_tree(&flat), Err(Error::Unsupported(_))));
        // A record list with leftovers past the tree is malformed.
        let records = vec![
            crate::characters::PoseRecord {
                frame: crate::rfc_write::IDENTITY,
                word: 1,
                children: 0,
            },
            crate::characters::PoseRecord {
                frame: crate::rfc_write::IDENTITY,
                word: 2,
                children: 0,
            },
        ];
        assert!(poses_parse(&records).is_err());
    }

    fn poses_parse(records: &[crate::characters::PoseRecord]) -> Result<()> {
        pose_tree(&crate::characters::SavedPose::Tree(records.to_vec())).map(|_| ())
    }

    /// A level without a character section converts no characters, and one
    /// with placements (the installed root level, when it has one) keeps
    /// them through a second write.
    #[test]
    fn a_shipped_level_converts_its_characters() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        for name in ["exanima01.rfc", "arenao1.rfc", "exanima02.rfc"] {
            let scene = crate::rfc::Scene::parse(resource.get(name).unwrap()).unwrap();
            let Some(chars) = &scene.characters else {
                continue;
            };
            if chars.placements.is_empty() {
                continue;
            }
            let nodes = export_characters(&scene).unwrap();
            assert_eq!(nodes.len(), chars.placements.len());
            let again = crate::rfc::Scene::parse(&write(&crate::rfc_write::ExportScene {
                nodes,
                ..Default::default()
            }))
            .unwrap();
            let again_chars = again.characters.as_ref().unwrap();
            assert_eq!(again_chars.placements.len(), chars.placements.len());
            for (a, b) in chars.placements.iter().zip(&again_chars.placements) {
                assert_eq!(a.id, b.id);
                assert_eq!(a.condition, b.condition);
                for k in 0..3 {
                    assert!((a.position[k] - b.position[k]).abs() < 1e-3);
                }
            }
        }
    }

    #[test]
    fn a_level_scene_gathers_every_section() {
        // A level with a room, a mesh carrying a material, a prop, a placed
        // object, a grid, a local database, an environment and a character
        // database.
        let mut mesh = SceneNode {
            name: name16("Floor"),
            flags: class::MESH | flags::SAVED,
            body: NodeData::Mesh(Box::new(export_mesh(&read_mesh()))),
            ..Default::default()
        };
        mesh.tail = 0;
        let prop = SceneNode {
            name: name16("Table"),
            flags: crate::rfc_write::flags::PROP | crate::rfc_write::flags::SAVED,
            ..Default::default()
        };
        let mut object = placed(0xC000_0009, 0x0020_0000, 0);
        object.local = at([10.0, 20.0, 30.0]);
        object.world = object.local;
        let grid = ExportGrid {
            wall_sets: vec![name16("gews_a")],
            floor_sets: vec![name16("gefs_a")],
            width: 4,
            height: 4,
            plane_a: (0..16).collect(),
            plane_b: (16..32).collect(),
            plane_c: (32..48).collect(),
        };
        let mut payload = vec![0u8; 184];
        payload[0] = 0x5A;
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![plain_level(), mesh, prop, object],
            grid: Some(grid),
            material_library: vec![(name16("Wood"), payload.clone())],
            object_db: Some(crate::rfc_write::RdbImage {
                count: 1,
                table: vec![0u8; 32],
                data: vec![1, 2, 3],
            }),
            object_strings: vec![4, 5, 6],
            environment: Some(vec![0u8; 0xF8]),
            whole_scene: true,
            ..Default::default()
        };
        let bytes = write(&scene);
        let parsed = crate::rfc::Scene::parse(&bytes).unwrap();
        assert!(parsed.level_errors.is_empty(), "{:?}", parsed.level_errors);
        let raw = LevelRaw::scan(&bytes).unwrap();
        assert_eq!(raw.materials.len(), 1);

        let rebuilt = level_scene(&parsed, &raw).unwrap();
        // The fixture's prop and object carry no part records, so the
        // rebuilt scene holds exactly the four nodes it was given.
        assert_eq!(rebuilt.nodes.len(), 4);
        assert!(rebuilt.grid.is_some());
        assert!(rebuilt.object_db.is_some());
        assert_eq!(rebuilt.object_strings, [4, 5, 6]);
        assert_eq!(rebuilt.environment.as_deref(), Some(&vec![0u8; 0xF8][..]));
        assert!(rebuilt.whole_scene);

        // The rebuilt scene writes and parses back with every section.
        let again = write(&rebuilt);
        let reread = crate::rfc::Scene::parse(&again).unwrap();
        assert!(reread.level_errors.is_empty(), "{:?}", reread.level_errors);
        assert_eq!(reread.nodes.len(), parsed.nodes.len());
        assert_eq!(reread.props.len(), parsed.props.len());
        assert_eq!(reread.placed_objects.len(), parsed.placed_objects.len());
        assert_eq!(reread.local_records.len(), parsed.local_records.len());
        // The fixture's string table is raw bytes, not a library the reader
        // decodes; the scan shows the body itself is carried through.
        let again_raw = LevelRaw::scan(&again).unwrap();
        assert_eq!(again_raw.object_strings.as_deref(), Some(&[4, 5, 6][..]));
        assert!(reread.grid.is_some());
        // The fixture places no character, so the rebuilt section has no
        // characters chunk and hence no character database.
        assert!(again_raw.character_database.is_none());
    }

    #[test]
    fn a_written_character_database_scans_back_and_converts() {
        // The writer's characters section with a database and one placement:
        // the scan finds the body under the writer's tag, and the converted
        // scene carries it again.
        let character = crate::rfc_write::SceneNode {
            name: name16("Guard"),
            flags: crate::rfc_write::flags::CHARACTER | crate::rfc_write::flags::SAVED,
            character: Some(crate::rfc_write::ExportCharacter {
                record: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let scene = crate::rfc_write::ExportScene {
            nodes: vec![character],
            character_db: Some(crate::rfc_write::RdbImage {
                count: 1,
                table: vec![0u8; 32],
                data: vec![7, 8],
            }),
            ..Default::default()
        };
        let bytes = write(&scene);
        let raw = LevelRaw::scan(&bytes).unwrap();
        let body = raw.character_database.as_deref().unwrap();
        assert_eq!(
            body[..8],
            [
                crate::rfc_write::CHARACTER_DB_MAGIC.to_le_bytes(),
                32u32.to_le_bytes()
            ]
            .concat()[..]
        );
        let parsed = crate::rfc::Scene::parse(&bytes).unwrap();
        let rebuilt = level_scene(&parsed, &raw).unwrap();
        let db = rebuilt.character_db.unwrap();
        assert_eq!((db.count, db.data), (1, vec![7, 8]));
    }

    #[test]
    fn the_character_database_body_is_scanned() {
        let mut section = 1u32.to_le_bytes().to_vec();
        let db = chunk(CHARACTER_DB, b"magicdb");
        section.extend(db.clone());
        let bytes = crate::chunk::RFC_MAGIC.to_le_bytes().to_vec();
        let mut file = bytes;
        file.extend(chunk(CHARACTERS, &section));
        let raw = LevelRaw::scan(&file).unwrap();
        assert_eq!(raw.character_database.as_deref(), Some(&b"magicdb"[..]));

        // A database body keeps its table and data.
        let mut body = 0xDBAC_0D09u32.to_le_bytes().to_vec();
        body.extend_from_slice(&32u32.to_le_bytes());
        body.extend_from_slice(&[7u8; 32]);
        body.extend_from_slice(&[9u8; 5]);
        let image = rdb_image(&body, 0xDBAC_0D09).unwrap();
        assert_eq!(
            (image.count, image.table.len(), image.data.len()),
            (1, 32, 5)
        );
        assert!(rdb_image(&body, 0xDBAC_CD00).is_none());
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
