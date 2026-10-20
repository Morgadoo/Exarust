//! The morph controller's setup and nearest target.
//!
//! Setup stores the target's count and allocates and copies its two arrays.
//! The nearest update replaces the best point only when a new distance is
//! strictly closer. Evidence: the 100-byte `0x1000db620` and the 66-byte
//! `0x1000de0b0`.

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
}
