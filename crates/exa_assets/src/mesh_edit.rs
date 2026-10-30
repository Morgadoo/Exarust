//! The editable form of a mesh, and its conversion back to render form.
//!
//! A mesh is stored and drawn in render form: unified vertices, each naming
//! one element of every stream (position, texture coordinate, colour) and a
//! smoothing key, and triangles over them. To change its topology the
//! original turns it back into the editable form [`EditMesh`]: every
//! channel keeps one element index per triangle corner and the streams keep
//! the element values. The corners of each position element are listed in
//! an [`Incidence`], which the edits and the conversion reorder in place.
//!
//! Converting back ([`EditMesh::render`]) splits each position element's
//! corners into runs as the original does: by smoothing group (from the
//! triangles' smoothing bits, else the smoothing channel), by material
//! group, by texture coordinate in textured groups, and by texture winding
//! in normal-mapped groups. Each run becomes a unified vertex, numbered
//! group by group. The triangles are then reordered for the vertex cache
//! ([`crate::vertex_cache`]) and the vertices renumbered by first use.
//!
//! Positions stay in metres here; the original works in centimetres. The
//! rules are the same and are scale-free except for the bounds' floors,
//! which are converted. Evidence: `docs/formats/RFC.md` (Editable meshes).

use std::fmt;

use exa_formats::material::Material;
use exa_formats::rfc::{Mesh, MeshGroup, SkinWeights};

use crate::vertex_cache;

/// The corner after each corner of a triangle.
pub const NEXT: [usize; 3] = [1, 2, 0];
/// The corner before each corner of a triangle.
pub const PREV: [usize; 3] = [2, 0, 1];

/// Channel and stream keys are matched by their low 16 bits.
pub const KEY_MASK: u32 = 0xFFFF;
/// Channel key: positions (channel 0).
pub const KEY_POSITIONS: u32 = 0x0001;
/// Channel key: smoothing groups (no stream).
pub const KEY_SMOOTHING: u32 = 0x0002;
/// Channel key: texture coordinates.
pub const KEY_UVS: u32 = 0x0010;
/// Channel key bits 4..7: the channel's values are interpolated (texture
/// coordinates); a channel without them is subdivided by index only.
pub const INTERPOLATED: u32 = 0xF0;

/// Stream id bit: the stream shares its channel with an earlier stream and
/// takes that stream's element count after an edit (colours, skin weights).
pub const SHARES_CHANNEL: u32 = 0x0400_0000;
/// Stream format bit: the stream is left alone by subdivision.
pub const FIXED_STREAM: u32 = 0x0100_0000;
/// Stream format bits 16..23: the element type.
pub const TYPE_MASK: u32 = 0x00FF_0000;
pub const TYPE_VEC3: u32 = 0x00F3_0000;
pub const TYPE_VEC2: u32 = 0x00F2_0000;
pub const TYPE_RGBA8: u32 = 0x00B4_0000;

/// Group flag: the group is textured, so texture-coordinate seams split
/// its vertices (the same bit as the texture-coordinate key).
pub const SPLIT_UVS: u32 = 0x10;
/// Group flag: the group is normal mapped, so a change of texture winding
/// splits its vertices.
pub const SPLIT_TANGENTS: u32 = 0x4;
/// Texture slots whose presence makes a material textured: every slot but
/// the reflection map.
const TEXTURED_SLOTS: u32 = 0x0FEF_0000;
/// The normal-map slot.
const NORMAL_SLOT: u32 = 0x0020_0000;

/// Render-flag bit: the mesh keeps its triangle order (no vertex-cache
/// optimisation).
pub const KEEP_ORDER: u32 = 0x0200_0000;

/// Channel keys and stream words as the shipped files store them.
const POSITION_CHANNEL: u32 = 0x0001_0001;
const UV_CHANNEL: u32 = 0x0001_0010;
const POSITION_STREAM: (u32, u32) = (0x0000_0001, 0x00F3_0001);
const UV_STREAM: (u32, u32) = (0x0000_0010, 0x00F2_0010);
const COLOUR_STREAM: (u32, u32) = (0x0400_0001, 0x02B4_0100);
const SKIN_STREAM: (u32, u32) = (0x0400_0001, 0x02A4_1000);

/// The smallest half extent of a mesh's box: 0.001 cm.
const MIN_HALF_EXTENT: f32 = 0.001 * 0.01;
/// The box's own floor before it (1e-4 cm).
const BOX_FLOOR: f32 = 1e-4 * 0.01;
/// Squared radii at or below this (cm², as metres²) are kept unrooted.
const RADIUS_ROOT_MIN: f32 = 1.192_092_9e-7 * 1e-4;

/// Why a mesh could not be edited or converted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    Malformed(String),
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(m) => write!(f, "malformed mesh: {m}"),
        }
    }
}

impl std::error::Error for MeshError {}

fn malformed(m: impl Into<String>) -> MeshError {
    MeshError::Malformed(m.into())
}

/// One channel: an element index per triangle corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub key: u32,
    pub corners: Vec<[u32; 3]>,
}

/// A stream's element values.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamData {
    Vec3(Vec<[f32; 3]>),
    Vec2(Vec<[f32; 2]>),
    Rgba8(Vec<[u8; 4]>),
    Skin(Vec<SkinWeights>),
}

impl StreamData {
    pub fn len(&self) -> usize {
        match self {
            Self::Vec3(v) => v.len(),
            Self::Vec2(v) => v.len(),
            Self::Rgba8(v) => v.len(),
            Self::Skin(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets the element count, new elements zero.
    pub(crate) fn resize(&mut self, n: usize) {
        match self {
            Self::Vec3(v) => v.resize(n, [0.0; 3]),
            Self::Vec2(v) => v.resize(n, [0.0; 2]),
            Self::Rgba8(v) => v.resize(n, [0; 4]),
            Self::Skin(v) => v.resize(
                n,
                SkinWeights {
                    bones: [0; 4],
                    weights: [0.0; 4],
                },
            ),
        }
    }
}

/// A stream: its id and format words, the channel indexing it, its values.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub id: u32,
    pub format: u32,
    pub channel: usize,
    pub data: StreamData,
}

/// A run of triangles sharing one material: faces `first..end`; `flags`
/// holds [`SPLIT_UVS`] and [`SPLIT_TANGENTS`].
#[derive(Debug, Clone, PartialEq)]
pub struct FaceGroup {
    pub material: String,
    pub first: usize,
    pub end: usize,
    pub flags: u32,
}

/// One corner of a triangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Corner {
    pub face: u32,
    pub corner: u8,
}

impl Corner {
    fn at(self, channel: &[[u32; 3]]) -> u32 {
        channel[self.face as usize][self.corner as usize]
    }
}

/// The corners of every position element, element by element.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Incidence {
    pub corners: Vec<Corner>,
    /// `corners[starts[v]..starts[v + 1]]` are element `v`'s corners.
    pub starts: Vec<u32>,
}

impl Incidence {
    /// Lists every element's corners, the last corner first: the original
    /// links each new corner in right after the element's head and walks
    /// the ring from there, so the first corner met ends the list.
    pub fn build(positions: &[[u32; 3]], elements: usize) -> Result<Self, MeshError> {
        Self::build_with(positions, elements, |_, _, v| Ok(v))
    }

    /// The same incidence for a mesh whose channel stores a reference per
    /// corner instead of the element itself (the original's generic
    /// render-form variant): `elements_at[refs[face][corner]]` is the
    /// element. Exarust decodes meshes with their element per corner, so
    /// this variant has no caller in the loaders; it is kept for a
    /// generic-format import.
    pub fn build_indirect(
        refs: &[[u32; 3]],
        elements_at: &[u32],
        elements: usize,
    ) -> Result<Self, MeshError> {
        Self::build_with(refs, elements, |f, k, _| {
            let r = refs[f][k] as usize;
            elements_at
                .get(r)
                .copied()
                .ok_or_else(|| malformed(format!("face {f} corner {k} names reference {r}")))
        })
    }

    /// Builds the ring lists, mapping each corner to its element.
    fn build_with(
        corners: &[[u32; 3]],
        elements: usize,
        element: impl Fn(usize, usize, u32) -> Result<u32, MeshError>,
    ) -> Result<Self, MeshError> {
        let mut count = vec![0u32; elements + 1];
        for (f, t) in corners.iter().enumerate() {
            for (k, &raw) in t.iter().enumerate() {
                let v = element(f, k, raw)? as usize;
                if v >= elements {
                    return Err(malformed(format!(
                        "face {f} names element {v} of {elements}"
                    )));
                }
                count[v + 1] += 1;
            }
        }
        for v in 0..elements {
            count[v + 1] += count[v];
        }
        let starts = count;
        let mut fill = starts.clone();
        let mut out = vec![Corner { face: 0, corner: 0 }; corners.len() * 3];
        for (f, t) in corners.iter().enumerate().rev() {
            for k in (0..3).rev() {
                let v = element(f, k, t[k])? as usize;
                out[fill[v] as usize] = Corner {
                    face: f as u32,
                    corner: k as u8,
                };
                fill[v] += 1;
            }
        }
        Ok(Self {
            corners: out,
            starts,
        })
    }

    /// The positions in [`Incidence::corners`] of element `v`'s corners.
    pub fn of(&self, v: usize) -> std::ops::Range<usize> {
        match (self.starts.get(v), self.starts.get(v + 1)) {
            (Some(&a), Some(&b)) => a as usize..b as usize,
            _ => 0..0,
        }
    }
}

/// The smoothing split the mesh remembers between edits.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum Smoothing {
    /// Never worked out (a mesh as loaded).
    #[default]
    Unset,
    /// One group per position element (no smoothing data).
    PerVertex,
    /// Boundaries in the incidence of each group's corners.
    Groups(Vec<u32>),
}

/// A mesh in editable form.
#[derive(Debug, Clone, PartialEq)]
pub struct EditMesh {
    /// Channel 0 holds the positions.
    pub channels: Vec<Channel>,
    /// Stream 0 holds the positions.
    pub streams: Vec<Stream>,
    pub groups: Vec<FaceGroup>,
    pub face_flags: Option<Vec<u32>>,
    pub face_edges: Option<Vec<u8>>,
    /// A second per-face word the original only ever permutes: no reader in
    /// the mesh unit was found, so it is carried through edits untouched.
    pub face_extra: Option<Vec<u32>>,
    /// The material group of every face, as an assembly leaves it before
    /// the regroup pass sorts the faces into runs. `None` when the groups'
    /// `first..end` ranges already say it.
    pub face_group: Option<Vec<u32>>,
    /// After a subdivision: the centre child of every face before it.
    pub face_children: Option<Vec<u32>>,
    pub render_flags: u32,
    /// Times the mesh was subdivided.
    pub level: u32,
    pub(crate) incidence: Option<Incidence>,
    pub(crate) smoothing: Smoothing,
}

/// The split flags a material gives its group: textured when it has any
/// texture slot but the reflection map, normal mapped with a normal map.
/// (The original sets the second only when the normal map loads.)
pub fn material_split_flags(m: &Material) -> u32 {
    let slots = m.layers.iter().fold(0, |s, l| s | l.slot.flag());
    let mut flags = 0;
    if slots & TEXTURED_SLOTS != 0 {
        flags |= SPLIT_UVS;
    }
    if slots & NORMAL_SLOT != 0 {
        flags |= SPLIT_TANGENTS;
    }
    flags
}

impl EditMesh {
    /// The editable form of a decoded mesh: each channel's corners are its
    /// map over the triangles. `group_flags` gives each group's split
    /// flags ([`material_split_flags`]).
    pub fn from_mesh(mesh: &Mesh, group_flags: &[u32]) -> Result<Self, MeshError> {
        let n = mesh.positions.len();
        if !mesh.indices.len().is_multiple_of(3) {
            return Err(malformed("index count is not a multiple of 3"));
        }
        if mesh.position_ids.len() != n {
            return Err(malformed("position ids do not match the vertices"));
        }
        let faces: Vec<[u32; 3]> = mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                if t.iter().all(|&i| (i as usize) < n) {
                    Ok(*t)
                } else {
                    Err(malformed("a triangle names a missing vertex"))
                }
            })
            .collect::<Result<_, _>>()?;
        let corners_of = |map: &[u32]| -> Vec<[u32; 3]> {
            faces.iter().map(|t| t.map(|i| map[i as usize])).collect()
        };
        let elements = mesh
            .position_ids
            .iter()
            .map(|&i| i as usize + 1)
            .max()
            .unwrap_or(0)
            .max(mesh.position_count);
        let gather = |len: usize, ids: &[u32]| -> Vec<Option<usize>> {
            let mut of = vec![None; len];
            for (v, &e) in ids.iter().enumerate() {
                of[e as usize] = Some(v);
            }
            of
        };
        let by_position = gather(elements, &mesh.position_ids);
        let mut channels = vec![Channel {
            key: POSITION_CHANNEL,
            corners: corners_of(&mesh.position_ids),
        }];
        let mut streams = vec![Stream {
            id: POSITION_STREAM.0,
            format: POSITION_STREAM.1,
            channel: 0,
            data: StreamData::Vec3(
                by_position
                    .iter()
                    .map(|v| v.map_or([0.0; 3], |v| mesh.positions[v]))
                    .collect(),
            ),
        }];
        if let Some(uvs) = &mesh.uvs
            && uvs.len() == n
            && mesh.uv_ids.len() == n
        {
            let count = mesh
                .uv_ids
                .iter()
                .map(|&i| i as usize + 1)
                .max()
                .unwrap_or(0)
                .max(mesh.uv_count);
            let by_uv = gather(count, &mesh.uv_ids);
            channels.push(Channel {
                key: UV_CHANNEL,
                corners: corners_of(&mesh.uv_ids),
            });
            streams.push(Stream {
                id: UV_STREAM.0,
                format: UV_STREAM.1,
                channel: channels.len() - 1,
                data: StreamData::Vec2(
                    by_uv
                        .iter()
                        .map(|v| v.map_or([0.0; 2], |v| uvs[v]))
                        .collect(),
                ),
            });
        }
        if mesh.smoothing.len() == n {
            channels.push(Channel {
                key: KEY_SMOOTHING,
                corners: corners_of(&mesh.smoothing),
            });
        }
        if let Some(colors) = &mesh.colors
            && colors.len() == n
        {
            streams.push(Stream {
                id: COLOUR_STREAM.0,
                format: COLOUR_STREAM.1,
                channel: 0,
                data: StreamData::Rgba8(
                    by_position
                        .iter()
                        .map(|v| v.map_or([0; 4], |v| colors[v]))
                        .collect(),
                ),
            });
        }
        if let Some(skin) = &mesh.skin
            && skin.len() == n
        {
            streams.push(Stream {
                id: SKIN_STREAM.0,
                format: SKIN_STREAM.1,
                channel: 0,
                data: StreamData::Skin(
                    by_position
                        .iter()
                        .map(|v| {
                            v.map_or(
                                SkinWeights {
                                    bones: [0; 4],
                                    weights: [0.0; 4],
                                },
                                |v| skin[v],
                            )
                        })
                        .collect(),
                ),
            });
        }
        let groups = mesh
            .groups
            .iter()
            .enumerate()
            .map(|(g, group)| {
                let first = group.first_index as usize / 3;
                FaceGroup {
                    material: group.material.clone(),
                    first,
                    end: first + group.index_count as usize / 3,
                    flags: group_flags.get(g).copied().unwrap_or(0),
                }
            })
            .collect();
        let per_face =
            |v: &Option<Vec<u32>>| v.as_ref().filter(|v| v.len() == faces.len()).cloned();
        Ok(Self {
            channels,
            streams,
            groups,
            face_flags: per_face(&mesh.triangle_flags),
            face_edges: mesh
                .triangle_edges
                .as_ref()
                .filter(|v| v.len() == faces.len())
                .cloned(),
            face_extra: None,
            face_group: None,
            face_children: None,
            render_flags: mesh.render_flags,
            level: 0,
            incidence: None,
            smoothing: Smoothing::Unset,
        })
    }

    pub fn face_count(&self) -> usize {
        self.channels.first().map_or(0, |c| c.corners.len())
    }

    /// The position element count.
    pub fn position_count(&self) -> usize {
        self.streams.first().map_or(0, |s| s.data.len())
    }

    /// The first channel whose key is `key`.
    pub fn channel_index(&self, key: u32) -> Option<usize> {
        self.channels.iter().position(|c| c.key & KEY_MASK == key)
    }

    /// Checks what the edits index: every channel has a corner triple per
    /// face, stream 0 holds Vec3 positions on channel 0, streams name an
    /// existing channel, groups lie within the faces.
    pub fn validate(&self) -> Result<(), MeshError> {
        let faces = self.face_count();
        if self.channels.is_empty() || self.streams.is_empty() {
            return Err(malformed("no position channel or stream"));
        }
        if self.channels.iter().any(|c| c.corners.len() != faces) {
            return Err(malformed("channels disagree on the face count"));
        }
        let first = &self.streams[0];
        if first.channel != 0 || !matches!(first.data, StreamData::Vec3(_)) {
            return Err(malformed("stream 0 is not the positions"));
        }
        if self
            .streams
            .iter()
            .any(|s| s.channel >= self.channels.len())
        {
            return Err(malformed("a stream names a missing channel"));
        }
        if self.groups.iter().any(|g| g.first > g.end || g.end > faces) {
            return Err(malformed("a group lies outside the faces"));
        }
        let positions = self.position_count();
        if self.channels[0]
            .corners
            .iter()
            .flatten()
            .any(|&v| v as usize >= positions)
        {
            return Err(malformed("a corner names a missing position"));
        }
        // A stream sharing its channel may be shorter (it takes the
        // channel's count after an edit), unless subdivision reads its
        // values (interpolated streams).
        for s in &self.streams {
            let len = s.data.len();
            if self.channels[s.channel]
                .corners
                .iter()
                .flatten()
                .any(|&v| v as usize >= len)
                && (s.id & SHARES_CHANNEL == 0 || s.format & INTERPOLATED != 0)
            {
                return Err(malformed("a corner names a missing stream element"));
            }
        }
        if let Some(f) = &self.face_flags
            && f.len() != faces
        {
            return Err(malformed("face flags do not match the faces"));
        }
        if let Some(e) = &self.face_edges
            && e.len() != faces
        {
            return Err(malformed("face edge bits do not match the faces"));
        }
        if let Some(x) = &self.face_extra
            && x.len() != faces
        {
            return Err(malformed("face extras do not match the faces"));
        }
        if let Some(g) = &self.face_group
            && g.len() != faces
        {
            return Err(malformed("face groups do not match the faces"));
        }
        if let Some(g) = &self.face_group
            && g.iter().any(|&g| g as usize >= self.groups.len())
        {
            return Err(malformed("a face names a missing material group"));
        }
        Ok(())
    }

    /// The original's mesh copy: every array is deep-copied (the editable
    /// channels at one index per corner, the streams at their element
    /// counts, the groups, the per-face arrays and the incidence) while the
    /// materials themselves are shared, which Rust's `String` handles by
    /// value. The copy is independent: editing it never touches `self`.
    pub fn duplicate(&self) -> Self {
        self.clone()
    }

    /// Add the colour stream the original's colour-channel helper creates:
    /// the first stream's id with the shared-channel bit, the colour format
    /// word, the first stream's channel and element count, filled opaque
    /// white unless `data` supplies the bytes. The new stream goes last and
    /// its index is returned. The original also gives the new element to
    /// every instance of the mesh; Exarust shares an instance's whole model
    /// instead, so there is nothing to broadcast.
    pub fn add_colour_channel(&mut self, data: Option<Vec<[u8; 4]>>) -> usize {
        let (id, channel, count) = self
            .streams
            .first()
            .map_or((COLOUR_STREAM.0, 0, 0), |stream| {
                (
                    stream.id | SHARES_CHANNEL,
                    stream.channel,
                    stream.data.len(),
                )
            });
        let colours = data.unwrap_or_else(|| vec![[255; 4]; count]);
        self.streams.push(Stream {
            id,
            format: COLOUR_STREAM.1,
            channel,
            data: StreamData::Rgba8(colours),
        });
        self.streams.len() - 1
    }

    /// The incidence, built when missing.
    pub(crate) fn incidence(&mut self) -> Result<&mut Incidence, MeshError> {
        if self.incidence.is_none() {
            let positions = self.position_count();
            self.incidence = Some(Incidence::build(&self.channels[0].corners, positions)?);
        }
        Ok(self.incidence.as_mut().expect("just built"))
    }

    /// Rebuilds the master incidence from the positions channel (the
    /// original's internal-mesh rebuild), dropping one that edits left
    /// behind. The position stream's element count bounds its elements.
    pub fn rebuild_incidence(&mut self) -> Result<(), MeshError> {
        self.validate()?;
        let elements = self.streams.first().map_or(0, |s| s.data.len());
        let inc = Incidence::build(&self.channels[0].corners, elements)?;
        self.incidence = Some(inc);
        self.smoothing = Smoothing::Unset;
        Ok(())
    }

    /// Reverses every face's winding: the original's flip swaps the first
    /// and third element of each corner triple on every channel. Stream
    /// values and faces keep their order.
    pub fn flip_winding(&mut self) {
        for channel in &mut self.channels {
            for corner in &mut channel.corners {
                corner.swap(0, 2);
            }
        }
        self.incidence = None;
        self.smoothing = Smoothing::Unset;
    }

    /// Replaces the smoothing channel by one numbering the smoothing
    /// groups worked out from the faces' smoothing bits, which reorders
    /// the incidence. The new channel goes last.
    pub(crate) fn smoothing_from_flags(&mut self) -> Result<(), MeshError> {
        let Some(flags) = self.face_flags.clone() else {
            return Ok(());
        };
        let faces = self.face_count();
        let inc = self.incidence()?;
        let groups = smoothing_groups(inc, &flags);
        let mut corners = vec![[0u32; 3]; faces];
        for u in 0..groups.len().saturating_sub(1) {
            for c in &inc.corners[groups[u] as usize..groups[u + 1] as usize] {
                corners[c.face as usize][c.corner as usize] = u as u32;
            }
        }
        if let Some(i) = self.channel_index(KEY_SMOOTHING) {
            self.channels.remove(i);
            for s in &mut self.streams {
                if s.channel > i {
                    s.channel -= 1;
                }
            }
        }
        self.channels.push(Channel {
            key: KEY_SMOOTHING,
            corners,
        });
        self.smoothing = Smoothing::Groups(groups);
        Ok(())
    }

    /// Converts the mesh to render form as the original does, reordering
    /// the triangles for the vertex cache when `order` is set and the
    /// mesh's render flags allow it. The edit mesh stays valid: its faces
    /// take the new order and its smoothing channel the new groups.
    pub fn render(&mut self, order: bool) -> Result<Mesh, MeshError> {
        self.validate()?;
        let faces = self.face_count();
        self.incidence()?;
        let mut inc = self.incidence.take().expect("built above");

        // Smoothing groups: from the faces' bits, else the smoothing
        // channel, else one per position element.
        let smoothing = match (&self.face_flags, self.channel_index(KEY_SMOOTHING)) {
            (Some(flags), _) => Smoothing::Groups(smoothing_groups(&mut inc, flags)),
            (None, Some(c)) => {
                let channel = &self.channels[c].corners;
                let starts = inc.starts.clone();
                Smoothing::Groups(refine(&mut inc, &starts, |c| c.at(channel)))
            }
            (None, None) => Smoothing::PerVertex,
        };
        let ranges = match &smoothing {
            Smoothing::Groups(r) => r.clone(),
            _ => inc.starts.clone(),
        };

        // Runs by material group, then by texture coordinate in textured
        // groups and by texture winding in normal-mapped groups.
        let mut material = vec![0u32; faces];
        for (g, group) in self.groups.iter().enumerate() {
            for m in &mut material[group.first..group.end] {
                *m = 1 << (g & 31);
            }
        }
        let mut runs = split_runs(&mut inc, &ranges, |c| material[c.face as usize]);
        for s in self.streams.iter().skip(1) {
            if s.id & SHARES_CHANNEL != 0 {
                continue;
            }
            let selected =
                self.group_mask(|g| s.format & KEY_MASK & g.flags != 0 && s.id & KEY_MASK & 3 == 0);
            if selected != 0 {
                let channel = &self.channels[s.channel].corners;
                runs = refine_runs(&mut inc, &runs, selected, |c| c.at(channel));
            }
        }
        let selected = self.group_mask(|g| g.flags & SPLIT_TANGENTS != 0);
        if selected != 0
            && let Some(mirrored) = self.mirrored_faces(selected)
        {
            runs = refine_runs(&mut inc, &runs, selected, |c| {
                mirrored[c.face as usize] as u32
            });
        }

        // A unified vertex per run, numbered group by group.
        let vertices = runs.len() - 1;
        let mut tri = vec![[u32::MAX; 3]; faces];
        let mut vertex_corners = Vec::with_capacity(vertices);
        let mut next = 0usize;
        for g in 0..self.groups.len() {
            let bit = 1u32 << (g & 31);
            for r in 0..vertices {
                if runs[r].0 != bit {
                    continue;
                }
                let (start, end) = (runs[r].1 as usize, runs[r + 1].1 as usize);
                for c in &inc.corners[start..end] {
                    tri[c.face as usize][c.corner as usize] = next as u32;
                }
                vertex_corners.push((start as u32, end as u32));
                next += 1;
            }
        }
        if next != vertices || tri.iter().flatten().any(|&v| v == u32::MAX) {
            return Err(malformed(
                "faces outside every material group, or more than 32 groups",
            ));
        }

        // Each channel's map over the unified vertices.
        let mut maps: Vec<Vec<u32>> = self
            .channels
            .iter()
            .map(|c| {
                let mut map = vec![0u32; vertices];
                for (t, corners) in tri.iter().zip(&c.corners) {
                    for k in 0..3 {
                        map[t[k] as usize] = corners[k];
                    }
                }
                map
            })
            .collect();
        if let Smoothing::Groups(groups) = &smoothing {
            let c = match self.channel_index(KEY_SMOOTHING) {
                Some(c) => c,
                None => {
                    self.channels.push(Channel {
                        key: KEY_SMOOTHING,
                        corners: vec![[0; 3]; faces],
                    });
                    maps.push(vec![0; vertices]);
                    self.channels.len() - 1
                }
            };
            for u in 0..groups.len() - 1 {
                for corner in &inc.corners[groups[u] as usize..groups[u + 1] as usize] {
                    maps[c][corner.at(&tri) as usize] = u as u32;
                }
            }
        }

        if order && self.render_flags & KEEP_ORDER == 0 {
            let mut faces_of = FaceArrays {
                tri: &mut tri,
                flags: self.face_flags.as_mut(),
                edges: self.face_edges.as_mut(),
                extra: self.face_extra.as_mut(),
                children: self.face_children.as_mut(),
            };
            let renumber = vertex_cache::order(
                &mut faces_of,
                &mut inc,
                &vertex_corners,
                &self.groups,
                self.render_flags,
            );
            for map in &mut maps {
                let mut new = vec![0u32; map.len()];
                for (v, &value) in map.iter().enumerate() {
                    if let Some(&n) = renumber.get(v)
                        && (n as usize) < new.len()
                    {
                        new[n as usize] = value;
                    }
                }
                *map = new;
            }
        }

        // The edit mesh follows: corners from the maps over the new order.
        for (c, map) in self.channels.iter_mut().zip(&maps) {
            for (corners, t) in c.corners.iter_mut().zip(&tri) {
                *corners = t.map(|v| map[v as usize]);
            }
        }
        self.incidence = Some(inc);
        self.smoothing = smoothing;
        Ok(self.render_mesh(&tri, &maps))
    }

    fn group_mask(&self, test: impl Fn(&FaceGroup) -> bool) -> u32 {
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, g)| test(g))
            .fold(0, |m, (g, _)| m | 1 << (g & 31))
    }

    /// Which faces of the `selected` groups have their texture mapped
    /// mirrored (clockwise in texture space), from the last texture
    /// coordinate stream. `None` without one (the original logs it).
    fn mirrored_faces(&self, selected: u32) -> Option<Vec<bool>> {
        let s = self
            .streams
            .iter()
            .rev()
            .find(|s| s.format & KEY_MASK == KEY_UVS)?;
        let StreamData::Vec2(uv) = &s.data else {
            return None;
        };
        let corners = &self.channels[s.channel].corners;
        let mut mirrored = vec![false; self.face_count()];
        for (g, group) in self.groups.iter().enumerate() {
            if selected & 1 << (g & 31) == 0 {
                continue;
            }
            for f in group.first..group.end {
                let [a, b, c] = corners[f].map(|i| uv.get(i as usize).copied().unwrap_or([0.0; 2]));
                let cross = (c[1] - a[1]) * (b[0] - a[0]) - (c[0] - a[0]) * (b[1] - a[1]);
                if cross < 0.0 {
                    mirrored[f] = true;
                }
            }
        }
        Some(mirrored)
    }

    /// The decoded-mesh form of the unified vertices.
    fn render_mesh(&self, tri: &[[u32; 3]], maps: &[Vec<u32>]) -> Mesh {
        let vertices = maps.first().map_or(0, Vec::len);
        let StreamData::Vec3(points) = &self.streams[0].data else {
            unreachable!("validated");
        };
        let position_ids = maps[0].clone();
        let positions: Vec<[f32; 3]> = position_ids
            .iter()
            .map(|&e| points.get(e as usize).copied().unwrap_or([0.0; 3]))
            .collect();
        let mut uv_ids = Vec::new();
        let mut uv_count = 0;
        let mut uvs = None;
        if let Some(s) = self
            .streams
            .iter()
            .find(|s| s.id & KEY_MASK == KEY_UVS && matches!(s.data, StreamData::Vec2(_)))
            && let StreamData::Vec2(data) = &s.data
        {
            uv_ids = maps[s.channel].clone();
            uv_count = data.len();
            uvs = Some(
                uv_ids
                    .iter()
                    .map(|&e| data.get(e as usize).copied().unwrap_or([0.0; 2]))
                    .collect(),
            );
        }
        let colors = self.streams.iter().find_map(|s| match &s.data {
            StreamData::Rgba8(data) if s.id & KEY_MASK == KEY_POSITIONS => Some(
                maps[s.channel]
                    .iter()
                    .map(|&e| data.get(e as usize).copied().unwrap_or([0xFF; 4]))
                    .collect(),
            ),
            _ => None,
        });
        let skin = self.streams.iter().find_map(|s| match &s.data {
            StreamData::Skin(data) if s.id & KEY_MASK == KEY_POSITIONS => Some(
                maps[s.channel]
                    .iter()
                    .map(|&e| {
                        data.get(e as usize).copied().unwrap_or(SkinWeights {
                            bones: [0; 4],
                            weights: [0.0; 4],
                        })
                    })
                    .collect(),
            ),
            _ => None,
        });
        let smoothing = match self.channel_index(KEY_SMOOTHING) {
            Some(c) => maps[c].clone(),
            None => position_ids.clone(),
        };
        let groups = self
            .groups
            .iter()
            .filter(|g| g.end > g.first)
            .map(|g| MeshGroup {
                material: g.material.clone(),
                first_index: (g.first * 3) as u32,
                index_count: ((g.end - g.first) * 3) as u32,
            })
            .collect();
        let mut mesh = Mesh {
            positions,
            normals: Vec::new(),
            uvs,
            colors,
            skin,
            position_ids,
            indices: tri.iter().flatten().copied().collect(),
            groups,
            triangle_flags: self.face_flags.clone(),
            triangle_edges: self.face_edges.clone(),
            gi_points: Vec::new(),
            render_flags: self.render_flags,
            position_count: points.len(),
            smoothing,
            uv_ids,
            uv_count,
        };
        debug_assert_eq!(mesh.positions.len(), vertices);
        mesh.recompute_normals();
        mesh
    }
}

/// The per-face arrays the vertex-cache order permutes.
pub(crate) struct FaceArrays<'a> {
    pub tri: &'a mut Vec<[u32; 3]>,
    pub flags: Option<&'a mut Vec<u32>>,
    pub edges: Option<&'a mut Vec<u8>>,
    pub extra: Option<&'a mut Vec<u32>>,
    pub children: Option<&'a mut Vec<u32>>,
}

/// Splits every range `ranges[i]..ranges[i + 1]` of the incidence into
/// groups of equal `key`, in the order of first appearance, moving each
/// group's corners together; returns the boundaries.
pub(crate) fn refine(inc: &mut Incidence, ranges: &[u32], key: impl Fn(Corner) -> u32) -> Vec<u32> {
    let mut out = vec![0u32];
    for w in ranges.windows(2) {
        let (mut s, end) = (w[0] as usize, w[1] as usize);
        while s < end {
            let k = key(inc.corners[s]);
            s = gather(&mut inc.corners, s, end, |c| key(c) == k);
            out.push(s as u32);
        }
    }
    out
}

/// Moves the corners of `corners[start + 1..end]` that are `same` as the
/// seed at `start` to just after it, keeping their order, by swapping each
/// into the next free place; returns the end of the group.
fn gather(
    corners: &mut [Corner],
    start: usize,
    end: usize,
    same: impl Fn(Corner) -> bool,
) -> usize {
    let mut next = start + 1;
    for j in start + 1..end {
        if same(corners[j]) {
            corners.swap(j, next);
            next += 1;
        }
    }
    next
}

/// Runs: `(key, start)` pairs and a closing `(0, end)`.
pub(crate) type Runs = Vec<(u32, u32)>;

/// Splits every range of the incidence into runs of equal `key`.
pub(crate) fn split_runs(inc: &mut Incidence, ranges: &[u32], key: impl Fn(Corner) -> u32) -> Runs {
    let mut out = Vec::new();
    for w in ranges.windows(2) {
        let (start, end) = (w[0] as usize, w[1] as usize);
        let mut s = start;
        while s < end {
            let k = key(inc.corners[s]);
            out.push((k, s as u32));
            s = gather(&mut inc.corners, s, end, |c| key(c) == k);
        }
    }
    out.push((0, ranges.last().copied().unwrap_or(0)));
    out
}

/// Splits the runs whose key meets `selected` further by `key`; the
/// others are kept whole.
pub(crate) fn refine_runs(
    inc: &mut Incidence,
    runs: &Runs,
    selected: u32,
    key: impl Fn(Corner) -> u32,
) -> Runs {
    let mut out = Vec::with_capacity(runs.len());
    for w in runs.windows(2) {
        let (mask, start, end) = (w[0].0, w[0].1 as usize, w[1].1 as usize);
        if mask & selected == 0 {
            out.push((mask, start as u32));
            continue;
        }
        let mut s = start;
        while s < end {
            let k = key(inc.corners[s]);
            out.push((mask, s as u32));
            s = gather(&mut inc.corners, s, end, |c| key(c) == k);
        }
    }
    out.push((0, runs.last().map_or(0, |r| r.1)));
    out
}

/// Splits every position element's corners into smoothing groups: from a
/// seed corner, the smoothing bits (low 16 bits of the face flags) of the
/// corners that share a bit with them are merged until nothing changes,
/// and every corner sharing a bit with the result joins the group. A face
/// without bits is a group of its own. Returns the boundaries.
pub(crate) fn smoothing_groups(inc: &mut Incidence, flags: &[u32]) -> Vec<u32> {
    let bits = |c: Corner| flags.get(c.face as usize).copied().unwrap_or(0);
    let mut out = vec![0u32];
    for v in 0..inc.starts.len().saturating_sub(1) {
        let end = inc.starts[v + 1] as usize;
        let mut s = *out.last().expect("never empty") as usize;
        while s < end {
            let mut mask = bits(inc.corners[s]) & 0xFFFF;
            loop {
                let mut grown = false;
                for c in &inc.corners[s..end] {
                    let m = bits(*c) & 0xFFFF;
                    if mask & m != 0 && mask | m != mask {
                        mask |= m;
                        grown = true;
                    }
                }
                if !grown {
                    break;
                }
            }
            s = gather(&mut inc.corners, s, end, |c| mask & bits(c) != 0);
            out.push(s as u32);
        }
    }
    out
}

/// A mesh's box and sphere as the original refits them after an edit:
/// the box of every position element (half extents at least 0.001 cm)
/// and the farthest element from its centre.
pub fn bounds(points: &[[f32; 3]]) -> Option<([f32; 3], [f32; 3], f32)> {
    let first = *points.first()?;
    let (mut lo, mut hi) = (first, first);
    for p in &points[1..] {
        for d in 0..3 {
            // As the vector min/max: the second operand when unordered.
            lo[d] = if lo[d] < p[d] { lo[d] } else { p[d] };
            hi[d] = if hi[d] > p[d] { hi[d] } else { p[d] };
        }
    }
    let centre: [f32; 3] = std::array::from_fn(|d| (lo[d] + hi[d]) * 0.5);
    let half: [f32; 3] = std::array::from_fn(|d| {
        let h = hi[d] - centre[d];
        let h = if h > BOX_FLOOR { h } else { BOX_FLOOR };
        if h < MIN_HALF_EXTENT {
            MIN_HALF_EXTENT
        } else {
            h
        }
    });
    let mut radius = 0.0f32;
    for p in points {
        let (x, y, z) = (p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]);
        let d = x * x + y * y + z * z;
        if radius <= d {
            radius = d;
        }
    }
    if radius > RADIUS_ROOT_MIN {
        radius = radius.sqrt();
    }
    Some((centre, half, radius))
}

#[cfg(test)]
#[path = "mesh_edit_test.rs"]
mod tests;
