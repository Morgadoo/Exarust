//! The original's terrain streaming decisions: where the streamed tiles are
//! gathered from, which 6 m blocks get a composite and at what resolution,
//! and the priority each block's look-ahead work is ranked by.
//!
//! Exarust's loader builds and inserts every tile behind its loading screen,
//! as the original's "block until every tile is loaded" does, and then
//! evaluates the composite per pixel. The decisions below are therefore
//! models of the original's rules rather than the driver of a live
//! streamer; they reproduce its numbers.
//!
//! Evidence: REA on the terrain's priority slot, its block selection and
//! the 3 × 3 gather: the composites' 44.4 m / 15 m cut-offs as squared
//! centimetres, the 189-entry list, and the pools of 21 high-res and 43
//! low-res texture sets.

use bevy::math::Vec2;

/// Distance (metres) within which a block gets a high-resolution composite:
/// the original compares the squared distance in centimetres against
/// 2 250 000 (15 m).
pub const NEAR: f32 = 15.0;
/// Distance (metres) within which a block gets any composite: squared
/// centimetres at most 19 713 600.
pub const FAR: f32 = 44.4;
/// High-resolution composite texture sets in the pool.
pub const HIGH_SETS: usize = 21;
/// Low-resolution composite texture sets in the pool.
pub const LOW_SETS: usize = 43;
/// Blocks the original keeps in its selected list (189 entries).
pub const MAX_SELECTED: usize = 189;
/// Block side, centimetres: a block's centre falls on a 6 m grid.
const BLOCK_CM: i64 = 600;
/// The original's reciprocal of the block side, 1/600 cm (its literal
/// single-precision constant).
const INV_BLOCK_CM: f32 = 0.001_666_666_7;
/// The original moves the camera −1000 cm along its back axis, 10 m ahead.
const LOOK_AHEAD_CM: f32 = -1000.0;

/// The point whose blocks get composites: the camera position (`camera`,
/// metres, world X and Z) moved −10 m along the camera's back axis
/// (`back`, its local +Z in world X and Z, unnormalised in the plane), so
/// 10 m ahead of it, and snapped to the centre of its 6 m block. In the
/// original's centimetres each axis is the engine floor of `p · 1/600`,
/// times 600, plus 300.
pub fn composite_focus(camera: Vec2, back: Vec2) -> Vec2 {
    let p = camera * 100.0 + back * LOOK_AHEAD_CM;
    let snap = |v: f32| (i64::from(round_down(v * INV_BLOCK_CM)) * BLOCK_CM + BLOCK_CM / 2) as f32;
    Vec2::new(snap(p.x), snap(p.y)) / 100.0
}

/// The engine's "round" helper: a conversion with the rounding mode set
/// toward minus infinity; NaN and out-of-range values give the integer
/// indefinite (`i32::MIN`).
fn round_down(value: f32) -> i32 {
    let r = value.floor();
    if r.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&r) {
        i32::MIN
    } else {
        r as i32
    }
}

/// One block that would be composited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeBlock {
    /// Index in the list the caller passed.
    pub index: usize,
    /// High resolution (`d² < 2 250 000 cm²`); otherwise low.
    pub high: bool,
}

/// The blocks within [`FAR`] of `focus`, in the caller's order, at most
/// [`MAX_SELECTED`]. Distances are squared centimetres, as the original
/// compares them; a block exactly 15 m away is low resolution. The original
/// drops a block only when its squared distance compares above the far
/// cut-off, so a NaN distance is kept (at low resolution).
pub fn select_composites(
    focus: Vec2,
    centres: impl IntoIterator<Item = Vec2>,
) -> Vec<CompositeBlock> {
    let mut out = Vec::new();
    for (index, centre) in centres.into_iter().enumerate() {
        if out.len() >= MAX_SELECTED {
            break;
        }
        let (dx, dz) = ((centre.x - focus.x) * 100.0, (centre.y - focus.y) * 100.0);
        let d2 = dz * dz + dx * dx;
        if !(d2 > 19_713_600.0) {
            out.push(CompositeBlock {
                index,
                high: d2 < 2_250_000.0,
            });
        }
    }
    out
}

/// Where a block's look-ahead work ranks: its distance rank and the priority
/// the original writes for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockPriority {
    /// The block's centre, metres (world X and Z).
    pub centre: Vec2,
    /// `round(6000 − distance)` in centimetres while the block is within
    /// 60 m, else 0.
    pub distance: i16,
    /// The block's priority.
    pub rank: u32,
    /// The block still needs its composite drawn (the selection sets this
    /// when a block turns high resolution).
    pub wants: bool,
    /// The block's scene node is visible this frame.
    pub visible: bool,
}

/// The distance rank of a block: `round(6000 − d)` centimetres for a block
/// within 60 m, 0 otherwise. The distance is the square root of
/// `dz² + dx²`, and the rounding is the hardware's round-to-nearest-even.
pub fn distance_rank(centre: Vec2, focus: Vec2) -> i16 {
    let (dx, dz) = ((focus.x - centre.x) * 100.0, (focus.y - centre.y) * 100.0);
    let d = (dz * dz + dx * dx).sqrt();
    if d >= 6000.0 || d.is_nan() {
        return 0;
    }
    (6000.0 - d).round_ties_even() as i16
}

/// Updates every block's priority from `focus` and returns the terrain's
/// new priority base.
///
/// Blocks that want a composite and are visible keep their relative order:
/// the base is half the spread of their old ranks, plus 6000. Every block
/// then takes `base − distance`, and those in the visible set take half
/// their old rank's offset from the lowest one, plus 6000, instead, so the
/// visible blocks stay ahead of the rest. The original scans for the lowest
/// and highest rank in one pass where a rank that lowers the lowest is not
/// also tried as the highest, and drops both (lowest and highest 0) when
/// the highest ends below the lowest: with no visible block, or with one,
/// or with ranks that only fall, the base is 6000.
pub fn update_priorities(blocks: &mut [BlockPriority], focus: Vec2) -> u32 {
    let mut lowest = u32::MAX;
    let mut highest = 0u32;
    for block in blocks.iter_mut() {
        block.distance = distance_rank(block.centre, focus);
    }
    for block in blocks.iter() {
        if block.wants && block.visible {
            if block.rank < lowest {
                lowest = block.rank;
            } else if block.rank > highest {
                highest = block.rank;
            }
        }
    }
    if highest < lowest {
        lowest = 0;
        highest = 0;
    }
    let base = ((highest - lowest) as i64 / 2) as u32 + 6000;
    for block in blocks.iter_mut() {
        let d = block.distance as u32;
        block.rank = if block.wants && block.visible {
            ((block.rank - lowest) / 2) + 6000 - d
        } else {
            base - d
        };
    }
    base
}

/// The 3 × 3 tiles around (`x`, `y`) as the original gathers them for a
/// terrain plugin: row-major, with the tile itself in the middle. Slots
/// outside the grid, or with no tile, are `None` (the original's empty
/// placeholder).
pub fn neighbours<T: Copy>(
    tiles: &[Option<T>],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
) -> [[Option<T>; 3]; 3] {
    let mut out = [[None; 3]; 3];
    for (row, slots) in out.iter_mut().enumerate() {
        for (col, slot) in slots.iter_mut().enumerate() {
            let (tx, ty) = (x + col as i32 - 1, y + row as i32 - 1);
            if tx < 0 || ty < 0 || tx >= width || ty >= height {
                continue;
            }
            let index = ty as usize * width as usize + tx as usize;
            *slot = tiles.get(index).copied().flatten();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(centre: Vec2, rank: u32, wants: bool, visible: bool) -> BlockPriority {
        BlockPriority {
            centre,
            distance: 0,
            rank,
            wants,
            visible,
        }
    }

    #[test]
    fn the_focus_looks_ahead_and_snaps_to_a_block_centre() {
        // Camera at (20, −4) looking along −Z (its back axis is +Z): 10 m
        // ahead is (20, −14) m, in block (3, −3) of 600 cm, whose centre is
        // (2100, −1500) cm.
        assert_eq!(
            composite_focus(Vec2::new(20.0, -4.0), Vec2::new(0.0, 1.0)),
            Vec2::new(21.0, -15.0)
        );
        // Looking down at 60°, the back axis is short in the plane and the
        // look-ahead with it: (0, 0) + 0.5 · 10 m along −X is (−5, 0),
        // block (−1, 0).
        assert_eq!(
            composite_focus(Vec2::ZERO, Vec2::new(0.5, 0.0)),
            Vec2::new(-3.0, 3.0)
        );
        // A point exactly on a block's low edge floors into that block.
        assert_eq!(
            composite_focus(Vec2::new(0.0, 0.0), Vec2::ZERO),
            Vec2::new(3.0, 3.0)
        );
        assert_eq!(
            composite_focus(Vec2::new(-6.0, -0.01), Vec2::ZERO),
            Vec2::new(-3.0, -3.0)
        );
    }

    #[test]
    fn composite_selection_uses_the_squared_centimetre_cutoffs() {
        let focus = Vec2::ZERO;
        let near = select_composites(
            focus,
            [
                Vec2::new(0.0, 0.0),   // 0 m: high
                Vec2::new(14.99, 0.0), // high
                Vec2::new(15.0, 0.0),  // exactly 15 m: low
                Vec2::new(44.4, 0.0),  // exactly 44.4 m: low
                Vec2::new(44.41, 0.0), // out
            ],
        );
        assert_eq!(
            near,
            vec![
                CompositeBlock {
                    index: 0,
                    high: true
                },
                CompositeBlock {
                    index: 1,
                    high: true
                },
                CompositeBlock {
                    index: 2,
                    high: false
                },
                CompositeBlock {
                    index: 3,
                    high: false
                },
            ]
        );
        // A NaN distance never compares above the cut-off: kept, low.
        let nan = select_composites(focus, [Vec2::new(f32::NAN, 0.0)]);
        assert_eq!(
            nan,
            vec![CompositeBlock {
                index: 0,
                high: false
            }]
        );
        // The list stops at 189 entries.
        let many = select_composites(focus, std::iter::repeat_n(Vec2::ZERO, 300));
        assert_eq!(many.len(), MAX_SELECTED);
        assert_eq!((HIGH_SETS, LOW_SETS), (21, 43));
    }

    #[test]
    fn distance_ranks_round_to_even_and_stop_at_sixty_metres() {
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::ZERO), 6000);
        // 59.99 m leaves 1 cm → rank 1.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(59.99, 0.0)), 1);
        // 0.005 cm below 60 m is not a whole centimetre either; 58 m gives
        // exactly 200.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(58.0, 0.0)), 200);
        // 30 m gives 3000.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(0.0, 30.0)), 3000);
        // Half a centimetre of remainder rounds to even: 59.995 m → the
        // distance is 5999.5 cm, 0.5 left, which rounds to 0.
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(59.995, 0.0)), 0);
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(60.0, 0.0)), 0);
        assert_eq!(distance_rank(Vec2::ZERO, Vec2::new(70.0, 0.0)), 0);
    }

    #[test]
    fn visible_blocks_keep_their_order_above_the_base() {
        let focus = Vec2::ZERO;
        let mut blocks = vec![
            block(Vec2::new(0.0, 0.0), 100, true, true),
            block(Vec2::new(6.0, 0.0), 300, true, true),
            block(Vec2::new(30.0, 0.0), 0, false, false),
            block(Vec2::new(60.0, 0.0), 0, true, false),
        ];
        let base = update_priorities(&mut blocks, focus);
        // Visible ranks 100..300 → base (300 − 100)/2 + 6000 = 6100.
        assert_eq!(base, 6100);
        // Distances: 0, 600, 3000, 6000 → ranks 0/6000/6000/6000.
        assert_eq!(blocks[0].distance, 6000);
        assert_eq!(blocks[1].distance, 5400);
        assert_eq!(blocks[2].distance, 3000);
        assert_eq!(blocks[3].distance, 0);
        // Visible: (rank − 100)/2 + 6000 − distance.
        assert_eq!(blocks[0].rank, 0);
        assert_eq!(blocks[1].rank, 700);
        // Not visible: base − distance.
        assert_eq!(blocks[2].rank, 6100 - 3000);
        assert_eq!(blocks[3].rank, 6100);
        // With no visible block, the old ranks are dropped: base 6000.
        let mut blocks = vec![block(Vec2::new(0.0, 0.0), 9, false, false)];
        assert_eq!(update_priorities(&mut blocks, focus), 6000);
        assert_eq!(blocks[0].rank, 0);
        // Ranks that only fall never reach the highest: 300 then 100 sets
        // the lowest twice, the highest stays 0 below it, and both drop to
        // 0. The visible blocks then take rank/2 + 6000 − distance.
        let mut blocks = vec![
            block(Vec2::new(0.0, 0.0), 300, true, true),
            block(Vec2::new(6.0, 0.0), 100, true, true),
            block(Vec2::new(30.0, 0.0), 0, false, false),
        ];
        assert_eq!(update_priorities(&mut blocks, focus), 6000);
        assert_eq!(blocks[0].rank, 150);
        assert_eq!(blocks[1].rank, 50 + 600);
        assert_eq!(blocks[2].rank, 3000);
    }

    #[test]
    fn the_gather_is_three_by_three_with_empty_placeholders() {
        let tiles: Vec<Option<u8>> = (0..9).map(Some).collect();
        let n = neighbours(&tiles, 3, 3, 1, 1);
        assert_eq!(
            n,
            [
                [Some(0), Some(1), Some(2)],
                [Some(3), Some(4), Some(5)],
                [Some(6), Some(7), Some(8)],
            ]
        );
        let n = neighbours(&tiles, 3, 3, 0, 0);
        assert_eq!(
            n,
            [
                [None, None, None],
                [None, Some(0), Some(1)],
                [None, Some(3), Some(4)],
            ]
        );
        // A missing tile is the placeholder too.
        let mut tiles = tiles;
        tiles[4] = None;
        let n = neighbours(&tiles, 3, 3, 1, 1);
        assert_eq!(n[1][1], None);
        assert_eq!(n[0][0], Some(0));
    }
}
