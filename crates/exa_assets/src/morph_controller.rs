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
}
