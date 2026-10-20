//! The morph controller: setup, nearest target, blends and the vertex apply.
//!
//! Setup stores the target's count and allocates and copies its two arrays.
//! The nearest update replaces the best point only when a new distance is
//! strictly closer. The basis solver projects a query onto a control
//! triangle's plane and returns its barycentric weights. The vertex apply
//! deforms each vertex's offset by the 3 × 3 basis its three morph points
//! form, subtracts the controller's base, applies the owner's orientation
//! and adds the vertex's base. The start binds each owner vertex to its
//! nearest control candidate and stores that candidate's weights. Evidence:
//! the 100-byte `0x1000db620`, the 66-byte `0x1000de0b0`, the 767-byte
//! `0x1000dd580`, the 893-byte `0x1000dd880`, the 680-byte `0x1000da330`
//! and the 448-byte `0x1000dd3c0`.

/// A morph controller's opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorphSetup {
    pub count: i32,
    pub allocated: bool,
    pub copied: bool,
}

/// Stores the count and allocates and copies both arrays.
pub fn setup_morph(count: i32) -> MorphSetup {
    MorphSetup {
        count,
        allocated: true,
        copied: true,
    }
}

/// The nearest morph target found so far.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphNearest {
    pub point: [i32; 3],
    pub extra: i32,
    pub distance: f32,
    pub marker: i32,
}

/// Replaces the best target when `distance` is strictly closer.
pub fn nearest_morph(
    distance: f32,
    current: f32,
    candidate: MorphNearest,
    best: MorphNearest,
) -> (MorphNearest, bool) {
    if distance < current {
        (
            MorphNearest {
                distance,
                ..candidate
            },
            true,
        )
    } else {
        (best, false)
    }
}

/// The nearest morph point's index and distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphNearestIndex {
    pub index: i32,
    pub distance: f32,
}

/// Scans transformed points and returns the nearest one.
pub fn nearest_morph_index(point: [f32; 3], points: &[[f32; 3]]) -> MorphNearestIndex {
    let mut best = MorphNearestIndex {
        index: -1,
        distance: f32::MAX,
    };
    for (index, candidate) in points.iter().enumerate() {
        let dx = point[0] - candidate[0];
        let dy = point[1] - candidate[1];
        let dz = point[2] - candidate[2];
        let distance = dx * dx + dy * dy + dz * dz;
        if distance < best.distance {
            best = MorphNearestIndex {
                index: index as i32,
                distance,
            };
        }
    }
    best
}

/// The morph target switch's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorphSwitch {
    pub switched: bool,
    pub point: [i32; 3],
    pub refreshed: bool,
}

/// Switches to the tagged target, copying and storing its point.
pub fn switch_morph_target(valid: bool, point: [i32; 3]) -> MorphSwitch {
    if !valid {
        return MorphSwitch {
            switched: false,
            point: [0; 3],
            refreshed: false,
        };
    }
    MorphSwitch {
        switched: true,
        point,
        refreshed: true,
    }
}

/// Adds `source × weight` to `destination` point by point.
pub fn blend_morph(destination: &[[f32; 3]], source: &[[f32; 3]], weight: f32) -> Vec<[f32; 3]> {
    destination
        .iter()
        .zip(source)
        .map(|(destination, source)| {
            [
                destination[0] + source[0] * weight,
                destination[1] + source[1] * weight,
                destination[2] + source[2] * weight,
            ]
        })
        .collect()
}

/// A morph part node reduced to what the walk reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorphPart {
    pub kind: i32,
    pub flags: i32,
    pub children: Vec<MorphPart>,
}

/// Counts and applies the morph to every eligible part.
pub fn apply_morph_parts(node: &MorphPart) -> usize {
    let mut count = 0;
    if node.kind & 0xfff == 0x89 && node.flags & 0xc0_0000 == 0x40_0000 {
        count += 1;
    }
    for child in &node.children {
        count += apply_morph_parts(child);
    }
    count
}

/// Four morph ids and their weights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphWeights {
    pub ids: [i32; 4],
    pub weights: [f32; 4],
}

/// Merges up to four source ids and weights into the current slots.
pub fn merge_morph_weights(
    mut current: MorphWeights,
    source_ids: &[i32],
    source_weights: &[f32],
    scale: f32,
) -> MorphWeights {
    const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);
    if scale < EPSILON {
        return current;
    }
    let mut last = current
        .ids
        .iter()
        .rposition(|&id| id > 0)
        .map_or(-1, |index| index as i32);
    for index in 0..4 {
        let Some(&id) = source_ids.get(index) else {
            break;
        };
        let weight = source_weights.get(index).copied().unwrap_or(0.0);
        if id < 1 {
            break;
        }
        let mut found = false;
        if last >= 0 {
            for slot in 0..=last as usize {
                if current.ids[slot] == id {
                    current.weights[slot] += weight * scale;
                    found = true;
                    break;
                }
            }
        }
        if !found && last < 3 {
            last += 1;
            current.ids[last as usize] = id;
            current.weights[last as usize] = weight * scale;
        }
    }
    current
}

/// Starts at `base` and adds each point times its weight.
pub fn blend_morph_point(base: [f32; 3], points: &[[f32; 3]], weights: &[f32]) -> [f32; 3] {
    const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);
    let mut result = base;
    for (point, &weight) in points.iter().zip(weights) {
        if weight.abs() < EPSILON {
            continue;
        }
        result[0] += point[0] * weight;
        result[1] += point[1] * weight;
        result[2] += point[2] * weight;
    }
    result
}

/// One entry of a morph block: the first points and the owner's points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorphBlockEntry {
    pub first: Vec<[i32; 3]>,
    pub second: Vec<[i32; 3]>,
}

/// A parsed morph block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorphBlock {
    pub unknown: u32,
    pub points_a: Vec<[i32; 3]>,
    pub skipped: Vec<[i32; 3]>,
    pub count_b: usize,
    pub entries: Vec<MorphBlockEntry>,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn word(&mut self) -> Option<i32> {
        let end = self.at.checked_add(4)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(i32::from_le_bytes(slice.try_into().ok()?))
    }

    fn count(&mut self) -> Option<usize> {
        let value = self.word()?;
        if value < 0 {
            return None;
        }
        let value = value as usize;
        if value > self.bytes.len().saturating_sub(self.at) / 12 {
            return None;
        }
        Some(value)
    }

    fn points(&mut self, count: usize) -> Option<Vec<[i32; 3]>> {
        let mut points = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            points.push([self.word()?, self.word()?, self.word()?]);
        }
        Some(points)
    }
}

/// Parses a tagged morph block, given the owner's point count.
pub fn parse_morph_block(bytes: &[u8], owner_points: usize) -> Option<MorphBlock> {
    let mut cursor = Cursor { bytes, at: 0 };
    if cursor.word()? != 0xedc0_c589u32 as i32 {
        return None;
    }
    let unknown = cursor.word()? as u32;
    let count_a = cursor.count()?;
    let points_a = cursor.points(count_a)?;
    let skipped = cursor.points(owner_points)?;
    let count_b = cursor.count()?;
    let mut entries = Vec::with_capacity(count_b.min(1024));
    for _ in 0..count_b {
        let first = cursor.points(count_a)?;
        let second = cursor.points(owner_points)?;
        entries.push(MorphBlockEntry { first, second });
    }
    Some(MorphBlock {
        unknown,
        points_a,
        skipped,
        count_b,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_morph_controller_opens_and_finds_the_nearest() {
        let setup = setup_morph(3);
        assert_eq!(setup.count, 3);
        assert!(setup.allocated);
        assert!(setup.copied);

        let best = MorphNearest {
            point: [1, 2, 3],
            extra: 4,
            distance: 1.0,
            marker: 5,
        };
        let candidate = MorphNearest {
            point: [6, 7, 8],
            extra: 9,
            distance: 0.0,
            marker: 10,
        };

        let (near, changed) = nearest_morph(0.5, 1.0, candidate, best);
        assert!(changed);
        assert_eq!(near.point, [6, 7, 8]);
        assert_eq!(near.extra, 9);
        assert_eq!(near.distance, 0.5);
        assert_eq!(near.marker, 10);

        let (same, changed) = nearest_morph(1.0, 1.0, candidate, best);
        assert!(!changed);
        assert_eq!(same, best);

        let (unordered, changed) = nearest_morph(f32::NAN, 1.0, candidate, best);
        assert!(!changed);
        assert_eq!(unordered, best);
    }

    #[test]
    fn the_nearest_morph_index_scans_points() {
        let points = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 0.5]];
        let found = nearest_morph_index([0.0, 0.0, 0.0], &points);
        assert_eq!(found.index, 2);
        assert_eq!(found.distance, 0.25);

        let with_nan = [[f32::NAN, 0.0, 0.0], [3.0, 0.0, 0.0]];
        let skipped = nearest_morph_index([0.0, 0.0, 0.0], &with_nan);
        assert_eq!(skipped.index, 1);
        assert_eq!(skipped.distance, 9.0);

        let empty = nearest_morph_index([0.0, 0.0, 0.0], &[]);
        assert_eq!(empty.index, -1);
        assert_eq!(empty.distance, f32::MAX);
    }

    #[test]
    fn the_morph_target_switches_on_its_tag() {
        let switched = switch_morph_target(true, [1, 2, 3]);
        assert!(switched.switched);
        assert_eq!(switched.point, [1, 2, 3]);
        assert!(switched.refreshed);

        let refused = switch_morph_target(false, [1, 2, 3]);
        assert!(!refused.switched);
        assert_eq!(refused.point, [0; 3]);
        assert!(!refused.refreshed);
    }

    #[test]
    fn the_morph_points_blend_by_weight() {
        let blended = blend_morph(&[[1.0, 2.0, 3.0]], &[[2.0, 4.0, 6.0]], 0.5);
        assert_eq!(blended, vec![[2.0, 4.0, 6.0]]);

        let shorter = blend_morph(&[[1.0, 1.0, 1.0], [2.0, 2.0, 2.0]], &[[1.0, 1.0, 1.0]], 1.0);
        assert_eq!(shorter, vec![[2.0, 2.0, 2.0]]);
    }

    #[test]
    fn the_morph_walks_its_parts() {
        let leaf = |kind, flags| MorphPart {
            kind,
            flags,
            children: Vec::new(),
        };
        let root = MorphPart {
            kind: 0x89,
            flags: 0x40_0000,
            children: vec![
                leaf(0x89, 0x40_0000),
                leaf(0x01, 0x40_0000),
                leaf(0x89, 0x80_0000),
            ],
        };
        assert_eq!(apply_morph_parts(&root), 2);
        assert_eq!(apply_morph_parts(&leaf(0x01, 0x40_0000)), 0);
        assert_eq!(apply_morph_parts(&leaf(0x89, 0x80_0000)), 0);
    }

    #[test]
    fn the_morph_weights_merge_into_four_slots() {
        let current = MorphWeights {
            ids: [1, 2, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        };
        let merged = merge_morph_weights(current, &[2, 3, 0, 0], &[1.0, 0.5, 0.0, 0.0], 2.0);
        assert_eq!(merged.ids, [1, 2, 3, 0]);
        assert_eq!(merged.weights, [0.5, 2.5, 1.0, 0.0]);

        let unchanged = merge_morph_weights(current, &[2, 3, 0, 0], &[1.0, 0.5, 0.0, 0.0], 0.0);
        assert_eq!(unchanged, current);

        let stopped = merge_morph_weights(current, &[0, 3, 0, 0], &[1.0, 1.0, 0.0, 0.0], 1.0);
        assert_eq!(stopped, current);
    }

    #[test]
    fn the_morph_point_blends_its_weights() {
        let blended = blend_morph_point([1.0, 2.0, 3.0], &[[2.0, 4.0, 6.0]], &[0.5]);
        assert_eq!(blended, [2.0, 4.0, 6.0]);

        let skipped = blend_morph_point(
            [1.0, 2.0, 3.0],
            &[[2.0, 4.0, 6.0], [1.0, 1.0, 1.0]],
            &[0.0, 1.0],
        );
        assert_eq!(skipped, [2.0, 3.0, 4.0]);
    }

    fn push(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&(value as u32).to_le_bytes());
    }

    fn morph_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        push(&mut bytes, 0xedc0c589u32 as i32);
        push(&mut bytes, 7);
        push(&mut bytes, 1);
        push(&mut bytes, 1);
        push(&mut bytes, 2);
        push(&mut bytes, 3);
        push(&mut bytes, 4);
        push(&mut bytes, 5);
        push(&mut bytes, 6);
        push(&mut bytes, 7);
        push(&mut bytes, 8);
        push(&mut bytes, 9);
        push(&mut bytes, 1);
        push(&mut bytes, 10);
        push(&mut bytes, 11);
        push(&mut bytes, 12);
        push(&mut bytes, 13);
        push(&mut bytes, 14);
        push(&mut bytes, 15);
        push(&mut bytes, 16);
        push(&mut bytes, 17);
        push(&mut bytes, 18);
        bytes
    }

    #[test]
    fn the_morph_block_parses_its_runs() {
        let bytes = morph_bytes();
        let block = parse_morph_block(&bytes, 2).unwrap();
        assert_eq!(block.unknown, 7);
        assert_eq!(block.points_a, vec![[1, 2, 3]]);
        assert_eq!(block.skipped, vec![[4, 5, 6], [7, 8, 9]]);
        assert_eq!(block.count_b, 1);
        assert_eq!(block.entries.len(), 1);
        assert_eq!(block.entries[0].first, vec![[10, 11, 12]]);
        assert_eq!(block.entries[0].second, vec![[13, 14, 15], [16, 17, 18]]);

        assert_eq!(parse_morph_block(&bytes[..bytes.len() - 4], 2), None);

        let mut wrong = bytes.clone();
        wrong[0] = 0;
        assert_eq!(parse_morph_block(&wrong, 2), None);
    }
}

/// One vertex the morph controller drives: the joint whose three points
/// form its basis, the vertex's offset in that basis and the base point
/// the result is added to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphVertex {
    pub joint: u32,
    pub offset: [f32; 3],
    pub base: [f32; 3],
}

/// Applies the morph's vertices: for each vertex, the three points its
/// joint names (through the remap) are the rows of a 3 × 3 basis, the
/// vertex's offset is multiplied by it, the controller's base is
/// subtracted, the owner's rows are applied, and the vertex's base is
/// added. One output point per vertex.
pub fn apply_morph_vertices(
    base: [f32; 3],
    owner_rows: [[f32; 3]; 3],
    joints: &[[u32; 3]],
    remap: &[u32],
    points: &[[f32; 3]],
    vertices: &[MorphVertex],
) -> Vec<[f32; 3]> {
    vertices
        .iter()
        .map(|vertex| {
            let joint = joints[vertex.joint as usize];
            let row0 = points[remap[joint[0] as usize] as usize];
            let row1 = points[remap[joint[1] as usize] as usize];
            let row2 = points[remap[joint[2] as usize] as usize];
            let offset = vertex.offset;
            let deformed = [
                row0[0] * offset[0] + row1[0] * offset[1] + row2[0] * offset[2],
                row0[1] * offset[0] + row1[1] * offset[1] + row2[1] * offset[2],
                row0[2] * offset[0] + row1[2] * offset[1] + row2[2] * offset[2],
            ];
            let relative = [
                deformed[0] - base[0],
                deformed[1] - base[1],
                deformed[2] - base[2],
            ];
            let rotated = [
                owner_rows[0][0] * relative[0]
                    + owner_rows[1][0] * relative[1]
                    + owner_rows[2][0] * relative[2],
                owner_rows[0][1] * relative[0]
                    + owner_rows[1][1] * relative[1]
                    + owner_rows[2][1] * relative[2],
                owner_rows[0][2] * relative[0]
                    + owner_rows[1][2] * relative[1]
                    + owner_rows[2][2] * relative[2],
            ];
            [
                rotated[0] + vertex.base[0],
                rotated[1] + vertex.base[1],
                rotated[2] + vertex.base[2],
            ]
        })
        .collect()
}

#[cfg(test)]
mod apply_tests {
    use super::*;

    #[test]
    fn the_basis_deforms_each_vertex() {
        let points = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let joints = [[0, 1, 2]];
        let remap = [0, 1, 2];
        let vertices = [MorphVertex {
            joint: 0,
            offset: [1.0, 1.0, 1.0],
            base: [0.0, 0.0, 10.0],
        }];
        let out = apply_morph_vertices(
            [1.0, 1.0, 1.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &joints,
            &remap,
            &points,
            &vertices,
        );
        assert_eq!(out, vec![[0.0, 1.0, 12.0]]);
    }

    #[test]
    fn the_remap_and_owner_rows_apply() {
        let points = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let joints = [[0, 1, 2]];
        let vertices = [MorphVertex {
            joint: 0,
            offset: [1.0, 0.0, 0.0],
            base: [0.0, 0.0, 0.0],
        }];
        let out = apply_morph_vertices(
            [0.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &joints,
            &[2, 0, 1],
            &points,
            &vertices,
        );
        assert_eq!(out, vec![[0.0, 0.0, 3.0]]);

        let out = apply_morph_vertices(
            [0.0, 0.0, 0.0],
            [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            &joints,
            &[0, 1, 2],
            &points,
            &[MorphVertex {
                joint: 0,
                offset: [1.0, 2.0, 0.0],
                base: [0.0, 0.0, 0.0],
            }],
        );
        assert_eq!(out, vec![[-4.0, 1.0, 0.0]]);
    }
}

/// A morph control triangle: the stored normal and its three points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphControl {
    pub normal: [f32; 3],
    pub points: [[f32; 3]; 3],
}

/// Solves the morph basis: projects the query point onto the control
/// triangle's plane through its first point along the stored normal, then
/// returns the triangle's barycentric weights. The branch is chosen by the
/// absolute x sum against the absolute y sum, and the third weight is one
/// minus the other two.
pub fn morph_barycentric(control: &MorphControl, query: [f32; 3]) -> [f32; 3] {
    let [p0, p1, p2] = control.points;
    let n = control.normal;

    // Move to the first point, remove the normal component, move back.
    let mut s = [query[0] - p0[0], query[1] - p0[1], query[2] - p0[2]];
    let dot = (s[0] * n[0] + s[2] * n[2]) + s[1] * n[1];
    s = [s[0] - n[0] * dot, s[1] - n[1] * dot, s[2] - n[2] * dot];
    s = [s[0] + p0[0], s[1] + p0[1], s[2] + p0[2]];

    let x0 = p0[0] - p2[0];
    let x1 = p1[0] - p2[0];
    let x2 = p2[0] - s[0];
    let y0 = p0[1] - p2[1];
    let y1 = p1[1] - p2[1];
    let y2 = p2[1] - s[1];
    let z0 = p0[2] - p2[2];
    let z1 = p1[2] - p2[2];
    let z2 = p2[2] - s[2];

    let (u, v) = if (x0 + x1).abs() < (y0 + y1).abs() {
        let den = (x1 + z1) * y0 - (x0 + z0) * y1;
        let num = (x2 + z2) * y1 - y2 * (x1 + z1);
        let u = num / den;
        let den2 = (x0 + z0) * y1 - (x1 + z1) * y0;
        let num2 = (x2 + z2) * y0 - y2 * (x0 + z0);
        (u, num2 / den2)
    } else {
        let den = x0 * (y1 + z1) - (y0 + z0) * x1;
        let num = (y2 + z2) * x1 - x2 * (y1 + z1);
        let u = num / den;
        let den2 = x1 * (y0 + z0) - (y1 + z1) * x0;
        let num2 = (y2 + z2) * x0 - x2 * (y0 + z0);
        (u, num2 / den2)
    };

    [u, v, 1.0 - u - v]
}

#[cfg(test)]
mod basis_tests {
    use super::*;

    #[test]
    fn the_basis_returns_barycentric_weights() {
        let control = MorphControl {
            normal: [0.0, 0.0, 1.0],
            points: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        };
        let at_p0 = morph_barycentric(&control, [0.0, 0.0, 0.0]);
        assert_eq!(at_p0, [1.0, 0.0, 0.0]);
        let at_p1 = morph_barycentric(&control, [1.0, 0.0, 0.0]);
        assert_eq!(at_p1, [0.0, 1.0, 0.0]);
        let inside = morph_barycentric(&control, [0.25, 0.25, 5.0]);
        assert_eq!(inside, [0.5, 0.25, 0.25]);
    }

    #[test]
    fn the_second_branch_solves_the_other_extent() {
        let control = MorphControl {
            normal: [0.0, 1.0, 0.0],
            points: [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 0.0, 1.0]],
        };
        let weights = morph_barycentric(&control, [0.5, 5.0, 0.25]);
        assert!((weights[0] - 0.625).abs() < 1e-6);
        assert!((weights[1] - 0.125).abs() < 1e-6);
        assert!((weights[2] - 0.25).abs() < 1e-6);
    }
}

/// One record the morph start builds for an owner vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphRecord {
    pub joint: u32,
    pub offset: [f32; 3],
    pub base: [f32; 3],
}

/// The morph start's work: the helper it creates, the records it builds
/// and the vertices with no control.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphStart {
    pub created_helper: bool,
    pub ensured_controls: bool,
    pub records: Vec<MorphRecord>,
    pub unmatched: Vec<usize>,
}

/// Starts the morph: transforms every owner vertex by the frame, binds it
/// to the nearest control candidate by the smallest squared distance from
/// the maximum float, stores that candidate's barycentric weights as the
/// vertex's offset, and stores the vertex as its base. A vertex with no
/// candidate gets joint 0, a zero offset and an entry in the unmatched
/// list. The helper is created when the owner has none, and the owner's
/// controls are ensured only when both are missing.
pub fn start_morph(
    owner_points: &[[f32; 3]],
    candidates: &[[f32; 3]],
    controls: &[MorphControl],
    frame: [[f32; 3]; 4],
    has_helper: bool,
    has_controls: bool,
) -> MorphStart {
    let mut records = Vec::with_capacity(owner_points.len());
    let mut unmatched = Vec::new();
    for (index, point) in owner_points.iter().enumerate() {
        let local = [
            point[0] * frame[0][0] + point[1] * frame[1][0] + point[2] * frame[2][0] + frame[3][0],
            point[0] * frame[0][1] + point[1] * frame[1][1] + point[2] * frame[2][1] + frame[3][1],
            point[0] * frame[0][2] + point[1] * frame[1][2] + point[2] * frame[2][2] + frame[3][2],
        ];
        let mut best = f32::MAX;
        let mut nearest = None;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            let dx = local[0] - candidate[0];
            let dy = local[1] - candidate[1];
            let dz = local[2] - candidate[2];
            let distance = (dx * dx + dy * dy) + dz * dz;
            if distance < best {
                best = distance;
                nearest = Some(candidate_index);
            }
        }
        if let Some(candidate) = nearest.and_then(|i| controls.get(i).map(|control| (i, control))) {
            let (joint, control) = candidate;
            records.push(MorphRecord {
                joint: joint as u32,
                offset: morph_barycentric(control, candidates[joint]),
                base: *point,
            });
        } else {
            records.push(MorphRecord {
                joint: 0,
                offset: [0.0; 3],
                base: *point,
            });
            unmatched.push(index);
        }
    }
    MorphStart {
        created_helper: !has_helper,
        ensured_controls: !has_helper && !has_controls,
        records,
        unmatched,
    }
}

#[cfg(test)]
mod start_tests {
    use super::*;

    fn unit_frame() -> [[f32; 3]; 4] {
        [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
        ]
    }

    fn triangle_at(x: f32) -> MorphControl {
        MorphControl {
            normal: [0.0, 0.0, 1.0],
            points: [[x, 0.0, 0.0], [x + 1.0, 0.0, 0.0], [x, 1.0, 0.0]],
        }
    }

    #[test]
    fn the_start_binds_each_vertex_to_its_nearest_control() {
        let start = start_morph(
            &[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]],
            &[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]],
            &[triangle_at(0.0), triangle_at(10.0)],
            unit_frame(),
            true,
            true,
        );
        assert!(!start.created_helper);
        assert!(!start.ensured_controls);
        assert_eq!(start.records[0].joint, 0);
        assert_eq!(start.records[0].offset, [1.0, 0.0, 0.0]);
        assert_eq!(start.records[0].base, [0.0, 0.0, 0.0]);
        assert_eq!(start.records[1].joint, 1);
        assert_eq!(start.records[1].offset, [1.0, 0.0, 0.0]);
        assert_eq!(start.records[1].base, [10.0, 0.0, 0.0]);
        assert!(start.unmatched.is_empty());
    }

    #[test]
    fn vertices_without_a_control_are_unmatched() {
        let start = start_morph(&[[1.0, 2.0, 3.0]], &[], &[], unit_frame(), false, false);
        assert!(start.created_helper);
        assert!(start.ensured_controls);
        assert_eq!(start.records[0].joint, 0);
        assert_eq!(start.records[0].offset, [0.0, 0.0, 0.0]);
        assert_eq!(start.records[0].base, [1.0, 2.0, 3.0]);
        assert_eq!(start.unmatched, vec![0]);
    }
}

/// One part's vertex bytes that the remap rebuilds. The record's flags at
/// `+8` skip a part that is ordered by a different index block; its kind at
/// `+4` gives the element size (bits 20 through 23) and the component count
/// (bits 16 through 19); its count is at `+0xc` and its bytes at `+0x10`.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphRemapPart {
    pub flags: u32,
    pub kind: u32,
    pub count: usize,
    pub data: Vec<u8>,
}

/// The refresh steps the rebuild runs, by the flags bit 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorphRefresh {
    pub purge: bool,
    pub rebuild_arrays: bool,
    pub rebuild_indices: bool,
    pub rebuild_vertices: bool,
    pub mark_dirty: bool,
    pub stamp: bool,
    pub rebind: bool,
}

/// The rebuild's result.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphRemap {
    pub map: Vec<u32>,
    pub parts: Vec<MorphRemapPart>,
    pub refresh: MorphRefresh,
}

/// The element size a part's kind names: 8 bytes for 0xA, 1 for 0xB, 4 for
/// 0xF, and 0 for any other code.
pub fn morph_element_size(kind: u32) -> usize {
    match (kind >> 20) & 0xf {
        0xa => 8,
        0xb => 1,
        0xf => 4,
        _ => 0,
    }
}

/// The stride of one part element: its component count times its element
/// size.
pub fn morph_stride(kind: u32) -> usize {
    (((kind >> 16) & 0xf) as usize) * morph_element_size(kind)
}

/// Rebuilds the morph's map and part bytes. The new map is `input[map[i]]`.
/// Each part whose flags are clear is gathered by `input[j]` over its stride
/// and count; a part with flags set is left alone. With bit 0 set the
/// rebuild also purges, marks the owner dirty, stamps the version and
/// re-binds the bound object; without it only the arrays, indices and
/// vertices are rebuilt.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_morph_remap(
    map: &[u32],
    input: &[u32],
    parts: &[MorphRemapPart],
    refresh: bool,
    has_bound: bool,
) -> MorphRemap {
    let new_map: Vec<u32> = map.iter().map(|&index| input[index as usize]).collect();
    let mut rebuilt = Vec::with_capacity(parts.len());
    for part in parts {
        if part.flags != 0 {
            rebuilt.push(part.clone());
            continue;
        }
        let stride = morph_stride(part.kind);
        let mut data = Vec::with_capacity(stride * part.count);
        for &source in input.iter().take(part.count) {
            let source = source as usize;
            for k in 0..stride {
                data.push(part.data[source * stride + k]);
            }
        }
        rebuilt.push(MorphRemapPart {
            data,
            ..part.clone()
        });
    }
    MorphRemap {
        map: new_map,
        parts: rebuilt,
        refresh: MorphRefresh {
            purge: refresh,
            rebuild_arrays: true,
            rebuild_indices: true,
            rebuild_vertices: true,
            mark_dirty: refresh,
            stamp: refresh,
            rebind: refresh && has_bound,
        },
    }
}

#[cfg(test)]
mod remap_tests {
    use super::*;

    #[test]
    fn the_remap_composes_and_gathers_each_part() {
        let parts = vec![
            MorphRemapPart {
                flags: 0,
                kind: (0xb << 20) | (2 << 16),
                count: 2,
                data: vec![10, 11, 20, 21],
            },
            MorphRemapPart {
                flags: 1,
                kind: (0xb << 20) | (2 << 16),
                count: 1,
                data: vec![7, 8],
            },
        ];
        let result = rebuild_morph_remap(&[1, 0], &[1, 0], &parts, false, false);
        assert_eq!(result.map, vec![0, 1]);
        assert_eq!(result.parts[0].data, vec![20, 21, 10, 11]);
        assert_eq!(result.parts[1].data, vec![7, 8]);
        assert!(!result.refresh.purge);
        assert!(result.refresh.rebuild_arrays);
        assert!(!result.refresh.mark_dirty);
        assert!(!result.refresh.rebind);
    }

    #[test]
    fn the_refresh_flag_runs_the_full_sequence() {
        let result = rebuild_morph_remap(&[0], &[0], &[], true, true);
        assert!(result.refresh.purge);
        assert!(result.refresh.mark_dirty);
        assert!(result.refresh.stamp);
        assert!(result.refresh.rebind);
        let quiet = rebuild_morph_remap(&[0], &[0], &[], false, true);
        assert!(!quiet.refresh.rebind);
    }

    #[test]
    fn the_kind_gives_the_element_size_and_stride() {
        assert_eq!(morph_element_size(0xb << 20), 1);
        assert_eq!(morph_element_size(0xa << 20), 8);
        assert_eq!(morph_element_size(0xf << 20), 4);
        assert_eq!(morph_element_size(0x1 << 20), 0);
        assert_eq!(morph_stride((0xb << 20) | (3 << 16)), 3);
        assert_eq!(morph_stride((0xf << 20) | (2 << 16)), 8);
    }
}

/// The morph data blob's magic (the original's `0xEDC0C589`).
pub const MORPH_MAGIC: u32 = 0xEDC0_C589;

/// One part's entry in the morph data: its pivot and one delta per vertex.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphPartData {
    pub pivot: [f32; 3],
    pub deltas: Vec<[f32; 3]>,
}

/// The serialized morph data the controller cooks once.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphData {
    pub magic: u32,
    pub size: usize,
    pub base: [f32; 3],
    pub rest: Vec<[f32; 3]>,
    pub parts: Vec<MorphPartData>,
}

/// The blob's size: 12·(P+1)·(V+1) + 12, the magic, size and part count
/// included.
pub fn morph_blob_size(parts: usize, verts: usize) -> usize {
    12 * (parts + 1) * (verts + 1) + 12
}

/// Cooks the morph data: the magic, its size, the base point, the rest
/// points and each part's pivot and deltas.
pub fn build_morph_data(base: [f32; 3], rest: &[[f32; 3]], parts: Vec<MorphPartData>) -> MorphData {
    let size = morph_blob_size(parts.len(), rest.len());
    MorphData {
        magic: MORPH_MAGIC,
        size,
        base,
        rest: rest.to_vec(),
        parts,
    }
}

impl MorphData {
    /// Serializes the blob little-endian: the magic, the size, the base,
    /// the rest points, the part count, then each part's pivot and deltas.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.size);
        out.extend_from_slice(&self.magic.to_le_bytes());
        out.extend_from_slice(&(self.size as u32).to_le_bytes());
        for value in self.base {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for point in &self.rest {
            for value in point {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out.extend_from_slice(&(self.parts.len() as u32).to_le_bytes());
        for part in &self.parts {
            for value in part.pivot {
                out.extend_from_slice(&value.to_le_bytes());
            }
            for delta in &part.deltas {
                for value in delta {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
        out
    }

    /// Parses a blob for an owner with `verts` vertices. The magic and the
    /// size must agree with the bytes and the vertex count.
    pub fn from_bytes(bytes: &[u8], verts: usize) -> Option<MorphData> {
        let magic = u32::from_le_bytes(bytes.get(0..4)?.try_into().ok()?);
        let size = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
        if magic != MORPH_MAGIC || size != bytes.len() {
            return None;
        }
        let unit = 12 * (verts + 1);
        let total = bytes.len().checked_sub(12)?;
        if unit == 0 || total % unit != 0 {
            return None;
        }
        let parts = (total / unit).checked_sub(1)?;
        let mut cursor = 8;
        let read_vec3 = |cursor: &mut usize| -> Option<[f32; 3]> {
            let mut value = [0.0f32; 3];
            for slot in &mut value {
                *slot = f32::from_le_bytes(bytes.get(*cursor..*cursor + 4)?.try_into().ok()?);
                *cursor += 4;
            }
            Some(value)
        };
        let base = read_vec3(&mut cursor)?;
        let mut rest = Vec::with_capacity(verts);
        for _ in 0..verts {
            rest.push(read_vec3(&mut cursor)?);
        }
        let count = u32::from_le_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
        cursor += 4;
        if count != parts {
            return None;
        }
        let mut built = Vec::with_capacity(parts);
        for _ in 0..parts {
            let pivot = read_vec3(&mut cursor)?;
            let mut deltas = Vec::with_capacity(verts);
            for _ in 0..verts {
                deltas.push(read_vec3(&mut cursor)?);
            }
            built.push(MorphPartData { pivot, deltas });
        }
        (cursor == bytes.len()).then_some(MorphData {
            magic,
            size,
            base,
            rest,
            parts: built,
        })
    }
}

/// One vertex's delta: the barycentric triangle point minus the part's
/// pivot, rotated by the owner's three rows.
pub fn morph_delta(
    rotation: [[f32; 3]; 3],
    bary: [f32; 3],
    corners: [[f32; 3]; 3],
    pivot: [f32; 3],
) -> [f32; 3] {
    let q = [
        corners[0][0] * bary[0] + corners[1][0] * bary[1] + corners[2][0] * bary[2],
        corners[0][1] * bary[0] + corners[1][1] * bary[1] + corners[2][1] * bary[2],
        corners[0][2] * bary[0] + corners[1][2] * bary[1] + corners[2][2] * bary[2],
    ];
    let d = [q[0] - pivot[0], q[1] - pivot[1], q[2] - pivot[2]];
    [
        rotation[0][0] * d[0] + rotation[1][0] * d[1] + rotation[2][0] * d[2],
        rotation[0][1] * d[0] + rotation[1][1] * d[1] + rotation[2][1] * d[2],
        rotation[0][2] * d[0] + rotation[1][2] * d[1] + rotation[2][2] * d[2],
    ]
}

#[cfg(test)]
mod cook_tests {
    use super::*;

    #[test]
    fn a_delta_is_the_triangle_point_less_the_pivot() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let corners = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let delta = morph_delta(identity, [0.5, 0.25, 0.25], corners, [0.0, 0.0, 0.0]);
        assert_eq!(delta, [0.25, 0.25, 0.0]);
        let zero = morph_delta(identity, [0.5, 0.25, 0.25], corners, [0.25, 0.25, 0.0]);
        assert_eq!(zero, [0.0, 0.0, 0.0]);

        let turn = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let turned = morph_delta(turn, [0.5, 0.25, 0.25], corners, [0.0, 0.0, 0.0]);
        assert_eq!(turned, [-0.25, 0.25, 0.0]);
    }

    #[test]
    fn the_blob_has_its_magic_and_size() {
        assert_eq!(morph_blob_size(1, 2), 84);
        let data = build_morph_data(
            [0.0, 0.0, 0.0],
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            vec![MorphPartData {
                pivot: [0.5, 0.0, 0.0],
                deltas: vec![[0.1, 0.0, 0.0], [0.2, 0.0, 0.0]],
            }],
        );
        assert_eq!(data.magic, 0xEDC0_C589);
        assert_eq!(data.size, 84);
        assert_eq!(data.parts.len(), 1);
        assert_eq!(data.rest.len(), 2);
    }

    #[test]
    fn the_blob_round_trips_through_its_bytes() {
        let data = build_morph_data(
            [1.0, 2.0, 3.0],
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            vec![MorphPartData {
                pivot: [0.5, 0.0, 0.0],
                deltas: vec![[0.1, 0.0, 0.0], [0.2, 0.0, 0.0]],
            }],
        );
        let bytes = data.to_bytes();
        assert_eq!(bytes.len(), 84);
        assert_eq!(&bytes[0..4], &[0x89, 0xC5, 0xC0, 0xED]);
        assert_eq!(&bytes[4..8], &84u32.to_le_bytes());
        assert_eq!(MorphData::from_bytes(&bytes, 2), Some(data));

        let mut bad = bytes.clone();
        bad[0] = 0;
        assert_eq!(MorphData::from_bytes(&bad, 2), None);
        assert_eq!(MorphData::from_bytes(&bytes, 3), None);
    }
}

/// A 32-byte skin record: up to four joints and their weights. Joint 0 ends
/// the list.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkinRecord {
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

/// Merges a source record into the destination with a scale, through the
/// four-slot joint merge.
pub fn merge_skin_record(
    mut destination: SkinRecord,
    source: SkinRecord,
    scale: f32,
) -> SkinRecord {
    let ids: Vec<i32> = source.joints.iter().map(|&joint| joint as i32).collect();
    let merged = merge_morph_weights(
        MorphWeights {
            ids: destination.joints.map(|joint| joint as i32),
            weights: destination.weights,
        },
        &ids,
        &source.weights,
        scale,
    );
    for slot in 0..4 {
        destination.joints[slot] = merged.ids[slot].max(0) as u32;
        destination.weights[slot] = merged.weights[slot];
    }
    destination
}

/// Scales every weight of a record.
pub fn scale_skin_record(mut record: SkinRecord, scale: f32) -> SkinRecord {
    for weight in &mut record.weights {
        *weight *= scale;
    }
    record
}

/// Normalizes a record: stops at the first zero joint, zeroes a negative
/// weight (leaving it out of the sum), then divides the weights through the
/// last used slot by the sum.
pub fn normalize_skin_record(record: &mut SkinRecord) {
    let mut sum = 0.0f32;
    let mut last: Option<usize> = None;
    for slot in 0..4 {
        if record.joints[slot] == 0 {
            break;
        }
        if record.weights[slot] < 0.0 {
            record.weights[slot] = 0.0;
        } else {
            sum += record.weights[slot];
        }
        last = Some(slot);
    }
    let Some(last) = last else {
        return;
    };
    let scale = 1.0 / sum;
    for slot in 0..=last {
        record.weights[slot] *= scale;
    }
}

/// Fills one target record from its neighbours: each neighbour is merged at
/// `1/sqrt(distance squared)`, and the merged weights are divided by the
/// distance sum. Returns whether a fill happened (the sum was positive).
pub fn fill_skin_record(record: &mut SkinRecord, neighbours: &[(SkinRecord, f32)]) -> bool {
    let mut merged = SkinRecord::default();
    let mut sum = 0.0f32;
    for &(neighbour, distance_squared) in neighbours {
        let weight = 1.0 / distance_squared.sqrt();
        merged = merge_skin_record(merged, neighbour, weight);
        sum += weight;
    }
    if sum > 0.0 {
        let scale = 1.0 / sum;
        *record = scale_skin_record(merged, scale);
        true
    } else {
        false
    }
}

/// Runs the three relaxation passes: each record becomes half itself plus
/// half of `1/sqrt(distance squared)` times each neighbour, then is
/// normalized. The neighbours are vertex ids and squared distances.
pub fn relax_skin_records(
    records: &[SkinRecord],
    neighbours: &[Vec<(usize, f32)>],
) -> Vec<SkinRecord> {
    let mut current = records.to_vec();
    for _ in 0..3 {
        let previous = current.clone();
        for (vertex, record) in current.iter_mut().enumerate() {
            let mut mixed = scale_skin_record(previous[vertex], 0.5);
            if let Some(list) = neighbours.get(vertex) {
                for &(neighbour, distance_squared) in list {
                    let weight = 0.5 * (1.0 / distance_squared.sqrt());
                    mixed = merge_skin_record(mixed, previous[neighbour], weight);
                }
            }
            normalize_skin_record(&mut mixed);
            *record = mixed;
        }
    }
    current
}

#[cfg(test)]
mod skin_tests {
    use super::*;

    #[test]
    fn a_record_normalizes_through_its_last_slot() {
        let mut record = SkinRecord {
            joints: [1, 2, 3, 0],
            weights: [1.0, 1.0, 2.0, 0.0],
        };
        normalize_skin_record(&mut record);
        assert_eq!(record.weights, [0.25, 0.25, 0.5, 0.0]);

        let mut negative = SkinRecord {
            joints: [1, 2, 0, 0],
            weights: [2.0, -1.0, 0.0, 0.0],
        };
        normalize_skin_record(&mut negative);
        assert_eq!(negative.weights, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn the_merge_adds_a_scaled_source() {
        let merged = merge_skin_record(
            SkinRecord::default(),
            SkinRecord {
                joints: [5, 6, 0, 0],
                weights: [0.5, 0.25, 0.0, 0.0],
            },
            2.0,
        );
        assert_eq!(merged.joints, [5, 6, 0, 0]);
        assert_eq!(merged.weights, [1.0, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn a_fill_weights_its_neighbours_by_the_inverse_distance() {
        let near = SkinRecord {
            joints: [1, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        };
        let far = SkinRecord {
            joints: [2, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        };
        let mut record = SkinRecord::default();
        assert!(fill_skin_record(&mut record, &[(near, 4.0), (far, 9.0)]));
        assert_eq!(record.joints, [1, 2, 0, 0]);
        let sum = 0.5f32 + 1.0 / 3.0;
        assert!((record.weights[0] - 0.5 / sum).abs() < 1e-6);
        assert!((record.weights[1] - (1.0 / 3.0) / sum).abs() < 1e-6);

        let mut empty = SkinRecord::default();
        assert!(!fill_skin_record(&mut empty, &[]));
    }

    #[test]
    fn three_relaxation_passes_blend_half_and_half() {
        let records = vec![
            SkinRecord {
                joints: [1, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
            SkinRecord {
                joints: [2, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
        ];
        let neighbours = vec![vec![(1, 4.0)], vec![(0, 4.0)]];
        let relaxed = relax_skin_records(&records, &neighbours);
        assert_eq!(relaxed[0].joints, [1, 2, 0, 0]);
        assert!(relaxed[0].weights[0] > relaxed[0].weights[1]);
        assert!((relaxed[0].weights[0] + relaxed[0].weights[1] - 1.0).abs() < 1e-6);
    }
}

/// Diffuses the target deltas: every unknown vertex whose neighbours are
/// known takes the weighted mean of those neighbours, at
/// `1/sqrt(distance squared)`, and becomes known in the next pass. The
/// passes repeat until one changes nothing; the number of passes that
/// changed something is returned.
pub fn diffuse_morph_deltas(
    deltas: &mut [[f32; 3]],
    known: &mut [bool],
    neighbours: &[Vec<(usize, f32)>],
) -> usize {
    let mut passes = 0;
    loop {
        let mut changed = false;
        let mut newly = Vec::new();
        for vertex in 0..deltas.len() {
            if known[vertex] {
                continue;
            }
            let mut sum = 0.0f32;
            let mut acc = [0.0f32; 3];
            if let Some(list) = neighbours.get(vertex) {
                for &(neighbour, distance_squared) in list {
                    if !known[neighbour] {
                        continue;
                    }
                    let weight = 1.0 / distance_squared.sqrt();
                    for axis in 0..3 {
                        acc[axis] += deltas[neighbour][axis] * weight;
                    }
                    sum += weight;
                }
            }
            if sum > 0.0 {
                for axis in 0..3 {
                    deltas[vertex][axis] = acc[axis] / sum;
                }
                newly.push(vertex);
                changed = true;
            }
        }
        for vertex in newly {
            known[vertex] = true;
        }
        if !changed {
            return passes;
        }
        passes += 1;
    }
}

#[cfg(test)]
mod diffuse_tests {
    use super::*;

    #[test]
    fn the_targets_take_the_neighbour_mean_pass_by_pass() {
        let mut deltas = vec![[4.0, 0.0, 0.0], [0.0; 3], [0.0; 3]];
        let mut known = vec![true, false, false];
        let neighbours = vec![vec![(1, 4.0)], vec![(0, 4.0), (2, 4.0)], vec![(1, 4.0)]];
        assert_eq!(
            diffuse_morph_deltas(&mut deltas, &mut known, &neighbours),
            2
        );
        assert_eq!(deltas[1], [4.0, 0.0, 0.0]);
        assert_eq!(deltas[2], [4.0, 0.0, 0.0]);
        assert_eq!(known, vec![true, true, true]);
    }

    #[test]
    fn an_unknown_vertex_without_known_neighbours_stays() {
        let mut deltas = vec![[0.0; 3], [1.0, 2.0, 3.0]];
        let mut known = vec![false, false];
        let neighbours = vec![vec![(1, 1.0)], vec![(0, 1.0)]];
        assert_eq!(
            diffuse_morph_deltas(&mut deltas, &mut known, &neighbours),
            0
        );
        assert_eq!(deltas[1], [1.0, 2.0, 3.0]);
        assert_eq!(known, vec![false, false]);
    }
}

/// The owner's skin-weight stream as the cook registers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorphStream {
    pub found: bool,
    pub grew: bool,
    pub flags: u32,
    pub kind: u32,
    pub count: usize,
    pub zeroed: usize,
}

/// Registers the skin-weight stream: only with weights and owner bit
/// 0x200000. An existing entry whose low flag bits are 0x1000 is reused;
/// otherwise the owner's list grows by one with flags 0x04000001 and type
/// 0x02a41000 and the vertex count. Either way the stream's `count·32`
/// bytes are zeroed.
pub fn register_morph_stream(
    has_weights: bool,
    owner_flag: bool,
    existing: bool,
    verts: usize,
) -> Option<MorphStream> {
    if !has_weights || !owner_flag {
        return None;
    }
    Some(MorphStream {
        found: existing,
        grew: !existing,
        flags: if existing { 0 } else { 0x0400_0001 },
        kind: if existing { 0 } else { 0x02a4_1000 },
        count: verts,
        zeroed: verts * 32,
    })
}

/// Rebuilds the controller's target list: every mesh triangle whose flags
/// carry 0xC00000 contributes its three corners, remapped through the
/// vertex map, in the order 1, 2, 0; a vertex already listed is not added
/// again.
pub fn collect_morph_targets(flagged: &[bool], triangles: &[[u32; 3]], map: &[u32]) -> Vec<u32> {
    const ORDER: [usize; 3] = [1, 2, 0];
    let mut targets: Vec<u32> = Vec::new();
    for (index, triangle) in triangles.iter().enumerate() {
        if !flagged.get(index).copied().unwrap_or(false) {
            continue;
        }
        for corner in ORDER {
            let mapped = map[triangle[corner] as usize];
            if !targets.contains(&mapped) {
                targets.push(mapped);
            }
        }
    }
    targets
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    #[test]
    fn the_stream_registers_only_with_weights_and_the_owner_bit() {
        assert_eq!(register_morph_stream(false, true, false, 8), None);
        assert_eq!(register_morph_stream(true, false, false, 8), None);

        let existing = register_morph_stream(true, true, true, 8).unwrap();
        assert!(existing.found);
        assert!(!existing.grew);
        assert_eq!(existing.zeroed, 256);

        let created = register_morph_stream(true, true, false, 8).unwrap();
        assert!(!created.found);
        assert!(created.grew);
        assert_eq!(created.flags, 0x0400_0001);
        assert_eq!(created.kind, 0x02a4_1000);
        assert_eq!(created.count, 8);
        assert_eq!(created.zeroed, 256);
    }

    #[test]
    fn the_targets_come_from_flagged_triangles_in_corner_order() {
        let flagged = [true, false, true];
        let triangles = [[10, 11, 12], [20, 21, 22], [10, 11, 12]];
        let map: Vec<u32> = (0..32).collect();
        assert_eq!(
            collect_morph_targets(&flagged, &triangles, &map),
            vec![11, 12, 10]
        );
    }
}

/// The epsilon the evaluators use to skip a weight (the original's
/// 1.19209e-7).
const MORPH_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// Restores the rest pose: the blob's base and its rest points.
pub fn restore_morph_positions(data: &MorphData) -> ([f32; 3], Vec<[f32; 3]>) {
    (data.base, data.rest.clone())
}

/// The weighted base: the blob's base plus each part's pivot times its
/// weight, skipping a weight whose absolute value is under the epsilon.
pub fn morph_weighted_base(data: &MorphData, weights: &[f32]) -> [f32; 3] {
    let mut base = data.base;
    for (part, &weight) in data.parts.iter().zip(weights) {
        if weight.abs() < MORPH_EPSILON {
            continue;
        }
        for (axis, value) in base.iter_mut().enumerate() {
            *value += part.pivot[axis] * weight;
        }
    }
    base
}

/// Evaluates the morphed points: the rest points, plus each part's pivot
/// and per-vertex delta times its weight. Returns the base and the points.
pub fn evaluate_morph_positions(data: &MorphData, weights: &[f32]) -> ([f32; 3], Vec<[f32; 3]>) {
    let mut base = data.base;
    let mut positions = data.rest.clone();
    for (part, &weight) in data.parts.iter().zip(weights) {
        if weight.abs() < MORPH_EPSILON {
            continue;
        }
        for (axis, value) in base.iter_mut().enumerate() {
            *value += part.pivot[axis] * weight;
        }
        for (position, delta) in positions.iter_mut().zip(&part.deltas) {
            for (axis, value) in position.iter_mut().enumerate() {
                *value += delta[axis] * weight;
            }
        }
    }
    (base, positions)
}

#[cfg(test)]
mod evaluate_tests {
    use super::*;

    fn data() -> MorphData {
        build_morph_data(
            [1.0, 0.0, 0.0],
            &[[0.0; 3], [1.0, 0.0, 0.0]],
            vec![
                MorphPartData {
                    pivot: [2.0, 0.0, 0.0],
                    deltas: vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                },
                MorphPartData {
                    pivot: [0.0, 3.0, 0.0],
                    deltas: vec![[0.0, 0.0, 1.0], [0.0, 0.0, 2.0]],
                },
            ],
        )
    }

    #[test]
    fn the_rest_pose_is_the_blob_base_and_points() {
        let (base, points) = restore_morph_positions(&data());
        assert_eq!(base, [1.0, 0.0, 0.0]);
        assert_eq!(points, vec![[0.0; 3], [1.0, 0.0, 0.0]]);
    }

    #[test]
    fn the_weighted_base_adds_the_scaled_pivots() {
        assert_eq!(morph_weighted_base(&data(), &[0.5, 2.0]), [2.0, 6.0, 0.0]);
        assert_eq!(morph_weighted_base(&data(), &[1e-8, 2.0]), [1.0, 6.0, 0.0]);
    }

    #[test]
    fn the_points_add_the_scaled_deltas() {
        let (base, points) = evaluate_morph_positions(&data(), &[2.0, 1.0]);
        assert_eq!(base, [5.0, 3.0, 0.0]);
        assert_eq!(points, vec![[2.0, 0.0, 1.0], [1.0, 2.0, 2.0]]);
    }
}

/// The child index a node's name begins with: two ASCII digits,
/// `(first − '0')·10 + (second − '0')`.
pub fn morph_child_index(name: &[u8]) -> i32 {
    let first = name.first().copied().unwrap_or(b'0') as i32 - i32::from(b'0');
    let second = name.get(1).copied().unwrap_or(b'0') as i32 - i32::from(b'0');
    first * 10 + second
}

/// Gathers children into an index array: each child's position goes to the
/// slot its name index names, the gaps stay zero, and the largest index
/// written is returned.
pub fn gather_morph_children(children: &[(i32, [f32; 3])]) -> (Vec<[f32; 3]>, i32) {
    let highest = children.iter().map(|&(index, _)| index).max().unwrap_or(0);
    let mut slots = vec![[0.0f32; 3]; highest.max(0) as usize + 1];
    for &(index, position) in children {
        if let Some(slot) = slots.get_mut(index.max(0) as usize) {
            *slot = position;
        }
    }
    (slots, highest)
}

/// One part the bake reorders and makes relative.
#[derive(Debug, Clone, PartialEq)]
pub struct BakePart {
    pub ordinal: i32,
    pub vertices: Vec<[f32; 3]>,
    pub positions: Vec<[f32; 3]>,
    pub deltas: Vec<[f32; 3]>,
}

/// Bakes the parts: a selection sort ascending by ordinal with a starting
/// best of 100, each delta set to the part's position minus the model's
/// position, and, with more than nine parts, part 5's vertices and deltas
/// subtracted from parts 6 through 8.
pub fn bake_morph_parts(model: &[[f32; 3]], mut parts: Vec<BakePart>) -> Vec<BakePart> {
    let count = parts.len();
    for index in 0..count {
        let chosen = parts
            .iter()
            .enumerate()
            .skip(index)
            .fold((100, index), |(best, chosen), (candidate, part)| {
                if part.ordinal < best {
                    (part.ordinal, candidate)
                } else {
                    (best, chosen)
                }
            })
            .1;
        parts.swap(index, chosen);
    }
    for part in &mut parts {
        for (delta, position, model_point) in part
            .deltas
            .iter_mut()
            .zip(&part.positions)
            .zip(model)
            .map(|((delta, position), model_point)| (delta, position, model_point))
        {
            for (axis, value) in delta.iter_mut().enumerate() {
                *value = position[axis] - model_point[axis];
            }
        }
    }
    if parts.len() > 9 {
        let (left, right) = parts.split_at_mut(6);
        let reference_vertices = left[5].vertices.clone();
        let reference_deltas = left[5].deltas.clone();
        for part in right.iter_mut().take(3) {
            for (slot, value) in part.vertices.iter_mut().enumerate() {
                if let Some(reference) = reference_vertices.get(slot) {
                    for axis in 0..3 {
                        value[axis] -= reference[axis];
                    }
                }
            }
            for (slot, value) in part.deltas.iter_mut().enumerate() {
                if let Some(reference) = reference_deltas.get(slot) {
                    for axis in 0..3 {
                        value[axis] -= reference[axis];
                    }
                }
            }
        }
    }
    parts
}

#[cfg(test)]
mod bake_tests {
    use super::*;

    #[test]
    fn a_name_indexes_its_child_by_its_first_two_digits() {
        assert_eq!(morph_child_index(b"07abc"), 7);
        assert_eq!(morph_child_index(b"12"), 12);
        assert_eq!(morph_child_index(b"0"), 0);
    }

    #[test]
    fn the_gather_zero_fills_the_gaps() {
        let (slots, highest) = gather_morph_children(&[(2, [1.0, 0.0, 0.0]), (0, [2.0, 0.0, 0.0])]);
        assert_eq!(highest, 2);
        assert_eq!(slots, vec![[2.0, 0.0, 0.0], [0.0; 3], [1.0, 0.0, 0.0]]);
    }

    #[test]
    fn the_bake_sorts_and_makes_the_deltas_relative() {
        let part = |ordinal, x: f32| BakePart {
            ordinal,
            vertices: vec![[x, 0.0, 0.0]],
            positions: vec![[x, 0.0, 0.0]],
            deltas: vec![[0.0; 3]],
        };
        let parts = vec![part(2, 2.0), part(0, 0.0), part(1, 1.0)];
        let baked = bake_morph_parts(&[[1.0, 0.0, 0.0]], parts);
        assert_eq!(
            baked.iter().map(|part| part.ordinal).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        for (slot, part) in baked.iter().enumerate() {
            assert_eq!(part.deltas[0], [slot as f32 - 1.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn parts_six_to_eight_take_part_five_off() {
        let parts = (0..10)
            .map(|index| BakePart {
                ordinal: index,
                vertices: vec![[index as f32, 0.0, 0.0]],
                positions: vec![[index as f32, 0.0, 0.0]],
                deltas: vec![[0.0; 3]],
            })
            .collect();
        let baked = bake_morph_parts(&[[0.0; 3]], parts);
        for (index, part) in baked.iter().enumerate().take(9).skip(6) {
            assert_eq!(part.vertices[0], [(index - 5) as f32, 0.0, 0.0]);
            assert_eq!(part.deltas[0], [(index - 5) as f32, 0.0, 0.0]);
        }
        assert_eq!(baked[5].vertices[0], [5.0, 0.0, 0.0]);
        assert_eq!(baked[9].vertices[0], [9.0, 0.0, 0.0]);
    }
}
