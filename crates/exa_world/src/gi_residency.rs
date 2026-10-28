//! Which of the level's GI blocks the original's voxel GI manager keeps
//! resident, builds next and steps each frame.
//!
//! The manager lays its blocks out once per level (`GiBlockLayout`) and
//! owns at most nine light textures. The first nine blocks in creation
//! order receive them. Every frame it measures each built block's distance
//! from the focus (the box distance in the ground plane, squared), sorts the
//! block list by it with a stable insertion sort, hands the background
//! builder the first block in that order that is not built yet (or was
//! marked dirty), and gives every built block among the first nine that has
//! no texture the texture of the farthest textured block beyond them. Each
//! built, textured block among the first nine then runs one light step.
//! Rules: `docs/RENDERING.md` (Voxel GI, residency).

use bevy::prelude::*;

use crate::gi_volume::GiBlockLayout;

/// The light textures the manager owns at most.
pub const RESIDENT_BLOCKS: usize = 9;

/// A block's half extent in the ground plane, centimetres (24 m).
const BLOCK_HALF_CM: f32 = 2400.0;

/// One block as the residency tracks it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidentBlock {
    /// The block's index in the layout.
    pub block: usize,
    /// Its centre in the ground plane (X, Z), world centimetres.
    pub centre_cm: Vec2,
    /// The squared distance from the last focus to its box, cm². A block
    /// that is not built keeps the value it had (0 at first).
    pub distance: f32,
    /// Its occupancy is built.
    pub built: bool,
    /// Its occupancy must be built again (a dynamic update touched it).
    pub dirty: bool,
    /// The light texture it holds (0..RESIDENT_BLOCKS), if any.
    pub texture: Option<usize>,
}

/// A texture moving from a far block to a near one: the near block takes
/// over the far block's light texture (and its light, which the steps then
/// overwrite) and needs its occupancy uploaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureHandoff {
    pub texture: usize,
    pub from: usize,
    pub to: usize,
}

/// The manager's block list in its current order.
#[derive(Resource, Clone, Debug, Default)]
pub struct GiResidency {
    /// The blocks, nearest first after [`GiResidency::rank`].
    pub order: Vec<ResidentBlock>,
    /// How many of the first blocks may hold a texture: nine, or fewer
    /// when the level has fewer blocks.
    pub resident: usize,
}

impl GiResidency {
    /// The residency of a freshly laid out level: nothing built, the first
    /// nine blocks (creation order) holding textures 0..8.
    pub fn new(layout: &GiBlockLayout) -> Self {
        let resident = RESIDENT_BLOCKS.min(layout.blocks.len());
        let order = layout
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| ResidentBlock {
                block: i,
                centre_cm: layout.centre_cm(b),
                distance: 0.0,
                built: false,
                dirty: false,
                texture: (i < resident).then_some(i),
            })
            .collect();
        Self { order, resident }
    }

    /// Measures every built block from `focus` (metres; height ignored)
    /// and sorts the list nearest first. The distance is
    /// `max(|f − c| − 2400, 0)` per axis in centimetres, squared and summed
    /// Z first; a NaN axis is not clamped. The insertion sort swaps a block
    /// forward only while its predecessor is strictly farther, so equal
    /// (or NaN) distances keep their order.
    pub fn rank(&mut self, focus: Vec3) {
        let f = focus.xz() * 100.0;
        for b in self.order.iter_mut().filter(|b| b.built) {
            let axis = |f: f32, c: f32| {
                let d = (f - c).abs() - BLOCK_HALF_CM;
                if d < 0.0 { 0.0 } else { d }
            };
            let dx = axis(f.x, b.centre_cm.x);
            let dz = axis(f.y, b.centre_cm.y);
            b.distance = dz * dz + dx * dx;
        }
        for i in 1..self.order.len() {
            let mut j = i;
            while j > 0 && self.order[j - 1].distance > self.order[j].distance {
                self.order.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    /// The block the background builder takes next: the first in the
    /// current order that is not built or is dirty. Its dirty mark is
    /// cleared, as the manager clears it when it hands the block over.
    pub fn take_next_build(&mut self) -> Option<usize> {
        let b = self.order.iter_mut().find(|b| !b.built || b.dirty)?;
        b.dirty = false;
        Some(b.block)
    }

    /// Records a finished build. Returns whether the block holds a
    /// texture, so its occupancy must be uploaded now.
    pub fn mark_built(&mut self, block: usize) -> bool {
        match self.order.iter_mut().find(|b| b.block == block) {
            Some(b) => {
                b.built = true;
                b.texture.is_some()
            }
            None => false,
        }
    }

    /// Marks a block for rebuilding (a dynamic occupancy change).
    pub fn mark_dirty(&mut self, block: usize) {
        if let Some(b) = self.order.iter_mut().find(|b| b.block == block) {
            b.dirty = true;
        }
    }

    /// Gives every built block among the first [`GiResidency::resident`]
    /// that has no texture the texture of the farthest textured block
    /// beyond them, walking the list from its end. A near block with no
    /// such donor stays without one.
    pub fn hand_out_textures(&mut self) -> Vec<TextureHandoff> {
        let mut handoffs = Vec::new();
        let n = self.order.len();
        for i in 0..self.resident {
            if !self.order[i].built || self.order[i].texture.is_some() {
                continue;
            }
            let Some(j) = (self.resident..n)
                .rev()
                .find(|&j| self.order[j].texture.is_some())
            else {
                continue;
            };
            let texture = self.order[j].texture.take().unwrap_or_default();
            self.order[i].texture = Some(texture);
            handoffs.push(TextureHandoff {
                texture,
                from: self.order[j].block,
                to: self.order[i].block,
            });
        }
        handoffs
    }

    /// The blocks that run a light step this frame: built and textured,
    /// among the first [`GiResidency::resident`], nearest first.
    pub fn stepping(&self) -> impl Iterator<Item = &ResidentBlock> {
        self.order
            .iter()
            .take(self.resident)
            .filter(|b| b.built && b.texture.is_some())
    }

    /// Whether `block` is among the blocks stepping this frame.
    pub fn steps(&self, block: usize) -> bool {
        self.stepping().any(|b| b.block == block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A residency of `n` blocks in a row along X, 48 m apart, block 0's
    /// centre at the origin.
    fn row(n: usize) -> GiResidency {
        GiResidency {
            order: (0..n)
                .map(|i| ResidentBlock {
                    block: i,
                    centre_cm: Vec2::new(i as f32 * 4800.0, 0.0),
                    distance: 0.0,
                    built: false,
                    dirty: false,
                    texture: (i < RESIDENT_BLOCKS.min(n)).then_some(i),
                })
                .collect(),
            resident: RESIDENT_BLOCKS.min(n),
        }
    }

    fn blocks(r: &GiResidency) -> Vec<usize> {
        r.order.iter().map(|b| b.block).collect()
    }

    /// Inside a block's 24 m half extent the distance is 0; beyond it, the
    /// overshoot squared per axis.
    #[test]
    fn the_distance_is_the_squared_overshoot_past_the_box() {
        let mut r = row(2);
        r.order.iter_mut().for_each(|b| b.built = true);
        r.rank(Vec3::new(30.0, 5.0, 25.0));
        // Block 0: |3000| − 2400 = 600, |2500| − 2400 = 100.
        assert_eq!(r.order[1].distance, 600.0 * 600.0 + 100.0 * 100.0);
        // Block 1: |3000 − 4800| − 2400 < 0 → 0; Z 100.
        assert_eq!(r.order[0].block, 1);
        assert_eq!(r.order[0].distance, 100.0 * 100.0);
    }

    /// Only built blocks are measured: unbuilt ones keep 0 and therefore
    /// sort ahead, in their order, so the builder takes them in list order.
    #[test]
    fn unbuilt_blocks_keep_their_distance_and_build_in_list_order() {
        let mut r = row(4);
        r.order[0].built = true;
        let focus = Vec3::new(-100.0, 0.0, 0.0);
        r.rank(focus);
        assert_eq!(blocks(&r), vec![1, 2, 3, 0]);
        assert_eq!(r.take_next_build(), Some(1));
        r.mark_built(1);
        r.rank(focus);
        assert_eq!(blocks(&r), vec![2, 3, 0, 1]);
        // Block 1 now measured (far), so block 2 leads the unbuilt.
        assert_eq!(r.take_next_build(), Some(2));
    }

    /// The sort is stable: equal distances keep their order, and a NaN
    /// distance never swaps.
    #[test]
    fn the_sort_keeps_equal_and_nan_distances_in_place() {
        let mut r = row(3);
        r.order[0].distance = 5.0;
        r.order[1].distance = f32::NAN;
        r.order[2].distance = 1.0;
        r.rank(Vec3::ZERO);
        // 2 cannot pass the NaN; 1 (NaN) cannot pass 0.
        assert_eq!(blocks(&r), vec![0, 1, 2]);
        r.order[1].distance = 5.0;
        r.rank(Vec3::ZERO);
        assert_eq!(blocks(&r), vec![2, 0, 1]);
    }

    /// A built near block without a texture takes the farthest textured
    /// block's texture beyond the first nine.
    #[test]
    fn near_blocks_take_the_farthest_texture() {
        let mut r = row(12);
        r.order.iter_mut().for_each(|b| b.built = true);
        // Focus at the far end: blocks 11, 10, 9 lead and have no texture.
        r.rank(Vec3::new(11.0 * 48.0, 0.0, 0.0));
        assert_eq!(&blocks(&r)[..4], &[11, 10, 9, 8]);
        let handoffs = r.hand_out_textures();
        assert_eq!(
            handoffs,
            vec![
                TextureHandoff {
                    texture: 0,
                    from: 0,
                    to: 11
                },
                TextureHandoff {
                    texture: 1,
                    from: 1,
                    to: 10
                },
                TextureHandoff {
                    texture: 2,
                    from: 2,
                    to: 9
                },
            ]
        );
        let stepping: Vec<usize> = r.stepping().map(|b| b.block).collect();
        assert_eq!(stepping, vec![11, 10, 9, 8, 7, 6, 5, 4, 3]);
        assert!(r.order[9..].iter().all(|b| b.texture.is_none()));
    }

    /// Unbuilt blocks neither take textures nor step; a finished build of a
    /// textured block asks for an upload.
    #[test]
    fn unbuilt_blocks_neither_take_textures_nor_step() {
        let mut r = row(10);
        r.order[9].built = true;
        r.rank(Vec3::new(9.0 * 48.0, 0.0, 0.0));
        // Block 9 is measured (0) and the rest stay 0: no move.
        assert!(r.hand_out_textures().is_empty());
        assert_eq!(r.stepping().count(), 0);
        assert!(r.mark_built(0));
        assert!(!r.mark_built(9));
        assert!(r.steps(0));
    }

    /// A dirty block is rebuilt, and its mark clears when it is taken.
    #[test]
    fn a_dirty_block_is_taken_again() {
        let mut r = row(2);
        r.order.iter_mut().for_each(|b| b.built = true);
        assert_eq!(r.take_next_build(), None);
        r.mark_dirty(1);
        assert_eq!(r.take_next_build(), Some(1));
        assert_eq!(r.take_next_build(), None);
    }

    /// A level with fewer than nine blocks keeps all of them resident.
    #[test]
    fn small_levels_keep_every_block() {
        let r = row(3);
        assert_eq!(r.resident, 3);
        assert_eq!(
            r.order.iter().map(|b| b.texture).collect::<Vec<_>>(),
            vec![Some(0), Some(1), Some(2)]
        );
    }
}
