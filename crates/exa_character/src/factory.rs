//! The character factory's start-up data: the per-slot child table and the
//! screen-format side of the skin texture job.
//!
//! The original builds its factory once: it allocates the object-database
//! table and fills each of the eighteen slots' child pointers from the
//! static slot table, and it reads the active pixel-format setting to pick
//! the side of the generated skin maps (512 or 1024). Exarust keeps the
//! table static ([`crate::layers::SLOTS`]) and reproduces the derived data
//! here so a job can be described without a renderer.

use crate::layers::SLOTS;

/// The pixel-format setting Exarust runs at: the original's 512-pixel skin
/// maps (the middle detail level). Exarust has no settings screen writing
/// it yet.
pub const PIXEL_FORMAT: u32 = 2;

/// The side of a generated skin map: the original uses 512 while the pixel
/// format setting is below 3 and 1024 from 3 up.
pub fn skin_side(pixel_format: u32) -> u32 {
    if pixel_format < 3 { 0x200 } else { 0x400 }
}

/// The side of the player's inventory preview job: the original takes 512
/// unless its high-quality flag is set and the pixel-format setting is 3 or
/// more, when it takes 1024.
pub fn preview_side(pixel_format: u32, high_quality: bool) -> u32 {
    if high_quality && pixel_format >= 3 {
        0x400
    } else {
        0x200
    }
}

/// The player's inventory preview job: the side its maps are drawn at and
/// the model it paints. The original starts it only when the shown item's
/// model differs from the one the last job painted (it compares the model
/// records, which two items share when their model names match).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewJob {
    /// The side of the generated maps ([`preview_side`]).
    pub side: u32,
    /// The model the job paints.
    pub model: String,
}

impl PreviewJob {
    /// The job to start for `model`, or `None` when `shown` is already that
    /// model.
    pub fn start(
        pixel_format: u32,
        high_quality: bool,
        model: &str,
        shown: Option<&str>,
    ) -> Option<Self> {
        (shown != Some(model)).then(|| Self {
            side: preview_side(pixel_format, high_quality),
            model: model.to_string(),
        })
    }
}

/// The two format words a texture job carries: its side, then the same word
/// as the target format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkinFormat {
    pub side: u32,
    pub format: u32,
}

impl SkinFormat {
    pub fn of_setting(pixel_format: u32) -> Self {
        let side = skin_side(pixel_format);
        Self { side, format: side }
    }
}

/// What the factory derives from the slot table at start-up: each slot's
/// child slots, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotChildren {
    pub slot: usize,
    pub children: [usize; 3],
    pub count: usize,
}

/// The eighteen slots' child tables, filled as the factory's constructor
/// does from the static table.
pub const SLOT_CHILDREN: [SlotChildren; 18] = build_slot_children();

const fn build_slot_children() -> [SlotChildren; 18] {
    let mut out = [SlotChildren {
        slot: 0,
        children: [0; 3],
        count: 0,
    }; 18];
    let mut i = 0;
    while i < 18 {
        let children = SLOTS[i].children;
        let mut count = children.len();
        if count > 3 {
            count = 3;
        }
        let mut table = [0usize; 3];
        let mut k = 0;
        while k < count {
            table[k] = children[k];
            k += 1;
        }
        out[i] = SlotChildren {
            slot: i + 1,
            children: table,
            count,
        };
        i += 1;
    }
    out
}

/// The factory's mutable state: the item database override the spawns and
/// rebuilds set (a placement's own database, or the rebuild's) and clear
/// when done. The original keeps it in the singleton; a factory owns it
/// here. The unit's finalizer, which frees the apparel model cache, is
/// Rust's drop.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterFactory {
    /// The database the equip pass resolves world/global item ids against.
    pub item_database: Option<String>,
}

impl CharacterFactory {
    /// Clears the item-database override.
    pub fn clear_item_database(&mut self) {
        self.item_database = None;
    }
}

/// The material manager's lock, which several generator routines take and
/// release. Exarust's asset loader owns materials exclusively; the
/// lock is kept as data so the acquire/release order can be tested.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaterialLock {
    pub held: bool,
}

impl MaterialLock {
    pub fn acquire(&mut self) {
        self.held = true;
    }

    pub fn release(&mut self) {
        self.held = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skin_side_is_512_below_three_and_1024_after() {
        assert_eq!(skin_side(0), 0x200);
        assert_eq!(skin_side(2), 0x200);
        assert_eq!(skin_side(3), 0x400);
        assert_eq!(
            SkinFormat::of_setting(2),
            SkinFormat {
                side: 0x200,
                format: 0x200
            }
        );
        assert_eq!(
            SkinFormat::of_setting(5),
            SkinFormat {
                side: 0x400,
                format: 0x400
            }
        );
    }

    #[test]
    fn the_preview_is_512_unless_high_quality_and_format_three() {
        assert_eq!(preview_side(2, true), 0x200);
        assert_eq!(preview_side(3, false), 0x200);
        assert_eq!(preview_side(3, true), 0x400);
        assert_eq!(preview_side(5, true), 0x400);
    }

    /// The preview starts only when the model changes, at the side the
    /// setting gives.
    #[test]
    fn the_preview_job_starts_for_a_new_model_only() {
        assert_eq!(
            PreviewJob::start(3, true, "BrPlate A", None),
            Some(PreviewJob {
                side: 0x400,
                model: "BrPlate A".into()
            })
        );
        assert_eq!(
            PreviewJob::start(3, true, "BrPlate A", Some("BrPlate A")),
            None
        );
        let job = PreviewJob::start(2, true, "BrPlate B", Some("BrPlate A")).unwrap();
        assert_eq!(job.side, 0x200);
        assert_eq!(job.model, "BrPlate B");
    }

    #[test]
    fn the_material_lock_is_acquired_and_released() {
        let mut lock = MaterialLock::default();
        lock.acquire();
        assert!(lock.held);
        lock.release();
        assert!(!lock.held);
    }

    #[test]
    fn an_item_database_override_clears() {
        let mut f = CharacterFactory {
            item_database: Some("arenaactors".into()),
        };
        f.clear_item_database();
        assert_eq!(f, CharacterFactory::default());
    }

    /// The constructor's child table: the pelvis has three children (the
    /// two legs and the belly), the chest three (head and both shoulders),
    /// a foot none.
    #[test]
    fn the_slot_child_table_is_the_static_one() {
        assert_eq!(SLOT_CHILDREN[0].slot, 1);
        assert_eq!(
            (SLOT_CHILDREN[0].children, SLOT_CHILDREN[0].count),
            ([2, 5, 8], 3)
        );
        assert_eq!(
            (SLOT_CHILDREN[8].children, SLOT_CHILDREN[8].count),
            ([10, 11, 14], 3)
        );
        assert_eq!(SLOT_CHILDREN[3].count, 0);
        assert_eq!(
            (SLOT_CHILDREN[15].children, SLOT_CHILDREN[15].count),
            ([18, 0, 0], 1)
        );
        for (i, s) in SLOT_CHILDREN.iter().enumerate() {
            assert_eq!(s.slot, i + 1);
        }
    }
}
