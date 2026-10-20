//! The morph controller: setup, nearest target, blends and the vertex apply.
//!
//! Setup stores the target's count and allocates and copies its two arrays.
//! The nearest update replaces the best point only when a new distance is
//! strictly closer. The vertex apply deforms each vertex's offset by the
//! 3 × 3 basis its three morph points form, subtracts the controller's
//! base, applies the owner's orientation and adds the vertex's base.
//! Evidence: the 100-byte `0x1000db620`, the 66-byte `0x1000de0b0` and the
//! 448-byte `0x1000dd3c0`.

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
