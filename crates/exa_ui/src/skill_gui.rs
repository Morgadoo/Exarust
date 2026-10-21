//! The skill GUI's resources and constructors (`SG_GUI_Skills`): the three
//! atlases built from `sgsklicons.rfi`, `skillicon.rfi` and `skillgui.rfi`,
//! the once-only atlas build and the two window constructors.
//!
//! `sgsklicons` is cut into six group tables of eight level icons: four
//! columns 96 pixels apart at two rows, five groups registered at the
//! original's atlas offsets. `skillicon` is cut into six 288-pixel icons at
//! its six offsets. `skillgui` is rescaled to 44 × 20 units and loaded as
//! its own texture. The full skill window takes the mode word, the inset
//! layout's 72-pixel radius and 1.25 scale (64 and 0.9 otherwise) and the
//! original's two group tables; the compact window copies six words from
//! the character data, or zeroes them when the caller asks.
//!
//! Evidence: the atlas builder, the atlas setup, the once-only unit
//! initialiser, the full and compact constructors and the attach routine.

use crate::icon_atlas::IconImage;

/// The skill-icon atlas's file name.
pub const SKILL_ICON_ATLAS: &str = "sgsklicons.rfi";
/// The large single-skill atlas's file name.
pub const SINGLE_ICON_ATLAS: &str = "skillicon.rfi";
/// The skill screen's background texture.
pub const SKILL_GUI: &str = "skillgui.rfi";
/// A cell of the skill-icon atlas, in pixels.
pub const ICON_CELL: i32 = 0x60;
/// A cell of the single-skill atlas, in pixels.
pub const SINGLE_CELL: i32 = 0x120;
/// The groups the atlas builder registers, with each set's atlas origin (in
/// the original's order). Group 1 has no set of its own.
pub const ICON_SETS: [(usize, i32, i32); 5] = [
    (0, 0, 0x60),
    (2, 0, 0x120),
    (3, 0, 0x1e0),
    (4, 0x180, 0x1e0),
    (5, 0x180, 0x120),
];
/// The six single-skill icons' atlas origins, in the original's order.
pub const SINGLE_ICONS: [(i32, i32); 6] = [
    (0, 0x240),
    (0x120, 0x240),
    (0, 0x120),
    (0x120, 0x120),
    (0, 0),
    (0x120, 0),
];
/// The icon size in units.
pub const ICON_UNITS: i32 = 8;
/// The single icon's size in units (24 units = the 0x120-pixel cell).
pub const SINGLE_ICON_UNITS: i32 = 24;
/// The background's size in units, 44 by 20.
pub const SKILL_GUI_UNITS: (i32, i32) = (44, 20);
/// The background texture's flags (0x33 | 0x100).
pub const SKILL_GUI_FLAGS: u32 = 0x33 | 0x100;

/// The full skill window's two group tables, 1-based, as the original
/// stores them: six groups each, the first four words the window's ring and
/// page slots.
pub const WINDOW_GROUPS: [i32; 6] = [1, 0, 2, 3, 4, 5];
pub const WINDOW_SLOTS: [i32; 6] = [1, 0, 2, 3, 2, 3];
/// How many of the table's entries a mode window uses.
pub const WINDOW_TABLE_COUNT: i32 = 4;

/// The once-only resource state: the atlases build once and the unit-change
/// hook is chained after the previous one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillResources {
    pub built: bool,
    pub hook_installed: bool,
    /// How many times the atlas build ran.
    pub builds: u32,
}

impl SkillResources {
    /// The unit initialiser: builds the atlases the first time only.
    pub fn ensure(&mut self) -> bool {
        if self.built {
            return false;
        }
        self.built = true;
        self.hook_installed = true;
        self.builds += 1;
        true
    }

    /// A unit change rebuilds the atlases.
    pub fn rebuild(&mut self) {
        self.built = true;
        self.builds += 1;
    }

    /// The unit's finalisation: the flags and the tables are dropped (the
    /// original frees the two icon managers and the background texture's
    /// name).
    pub fn finalize(&mut self) {
        self.built = false;
        self.hook_installed = false;
    }
}

/// The eight cells of one skill group at an atlas origin: four across at
/// 96-pixel steps, then the same four one row down.
pub fn group_cells(x: i32, y: i32) -> [(i32, i32); 8] {
    [
        (x, y),
        (x + ICON_CELL, y),
        (x + 2 * ICON_CELL, y),
        (x + 3 * ICON_CELL, y),
        (x, y + ICON_CELL),
        (x + ICON_CELL, y + ICON_CELL),
        (x + 2 * ICON_CELL, y + ICON_CELL),
        (x + 3 * ICON_CELL, y + ICON_CELL),
    ]
}

/// The handles of the skill-icon atlas: six groups of eight levels (group 1
/// keeps the default 0, as the original never registers a set for it).
pub type SkillIcons = [[i32; 8]; 6];

/// Cuts the eight cells of every registered group out of the atlas image
/// and hands each to `add`, which returns its handle. Cells outside the
/// image are skipped (their handle stays 0), so a short image is total.
pub fn build_group_icons(image: &IconImage, mut add: impl FnMut(&IconImage) -> i32) -> SkillIcons {
    let mut table = [[0i32; 8]; 6];
    for &(group, x, y) in &ICON_SETS {
        for (level, (cell_x, cell_y)) in group_cells(x, y).into_iter().enumerate() {
            if cell_x < 0
                || cell_y < 0
                || cell_x as u16 + ICON_CELL as u16 > image.width
                || cell_y as u16 + ICON_CELL as u16 > image.height
            {
                continue;
            }
            let cell = image.region(cell_x, cell_y, ICON_CELL as u16, ICON_CELL as u16);
            table[group][level] = add(&cell);
        }
    }
    table
}

/// Cuts the six single-skill icons out of their atlas image and hands each
/// to `add`.
pub fn build_single_icons(image: &IconImage, mut add: impl FnMut(&IconImage) -> i32) -> [i32; 6] {
    let mut table = [0i32; 6];
    for (slot, (x, y)) in SINGLE_ICONS.into_iter().enumerate() {
        if x < 0
            || y < 0
            || x as u16 + SINGLE_CELL as u16 > image.width
            || y as u16 + SINGLE_CELL as u16 > image.height
        {
            continue;
        }
        let cell = image.region(x, y, SINGLE_CELL as u16, SINGLE_CELL as u16);
        table[slot] = add(&cell);
    }
    table
}

/// The skill window's constructor arguments, as the original fills them in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillWindowPlan {
    /// The mode word (0 for the free skill screen).
    pub mode: i32,
    /// The number of group-table entries in use.
    pub group_count: i32,
    /// The ring's radius in units: 72 with the inset layout, else 64.
    pub radius: i32,
    /// The ring's scale: 1.25 with the inset layout, else 0.9.
    pub scale: f32,
    /// Whether the window was created with a mode.
    pub mode_set: bool,
    /// The window's layout flag (the constructor's 1).
    pub layout: i32,
    /// The two group tables.
    pub groups: [i32; 6],
    pub slots: [i32; 6],
    pub shown: bool,
}

impl SkillWindowPlan {
    /// The destructor: the tables and the window's link go.
    pub fn destroy(&mut self) {
        self.shown = false;
        self.mode = 0;
    }

    /// The constructor: the tables and the layout flag always, the mode and
    /// the ring geometry only for a positive mode. `inset` is the
    /// original's global inset-layout flag.
    pub fn new(mode: i32, inset: bool) -> Self {
        let mode_set = mode > 0;
        Self {
            mode,
            group_count: WINDOW_TABLE_COUNT,
            radius: if inset { 0x48 } else { 0x40 },
            scale: if inset { 1.25 } else { 0.9 },
            mode_set,
            layout: 1,
            groups: WINDOW_GROUPS,
            slots: WINDOW_SLOTS,
            shown: true,
        }
    }
}

/// The compact skill window's attached data: six little-endian words the
/// original copies from the character data at offsets 0xc0, 0xc8, 0xd0,
/// 0xd8, 0xe0 and 0xe8.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompactSkillPlan {
    pub words: [u16; 6],
    /// Whether the attach cleared the words instead of copying them.
    pub cleared: bool,
    pub shown: bool,
}

/// The offsets the compact window reads its six words from.
pub const COMPACT_WORD_OFFSETS: [usize; 6] = [0xc0, 0xc8, 0xd0, 0xd8, 0xe0, 0xe8];

impl CompactSkillPlan {
    /// Attaches character data: with `clear` the six words are zero, else
    /// each is the little-endian word at its offset. A data block shorter
    /// than an offset gives 0 for that word.
    pub fn attach(data: &[u8], clear: bool) -> Self {
        let mut words = [0u16; 6];
        if !clear {
            for (word, offset) in words.iter_mut().zip(COMPACT_WORD_OFFSETS) {
                if let Some(bytes) = data.get(offset..offset + 2) {
                    *word = u16::from_le_bytes([bytes[0], bytes[1]]);
                }
            }
        }
        Self {
            words,
            cleared: clear,
            shown: true,
        }
    }

    /// The six words as one packed value, in the original's order.
    pub fn packed(&self) -> [u16; 6] {
        self.words
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_is_four_cells_over_two_rows() {
        assert_eq!(
            group_cells(0, 0x60),
            [
                (0, 0x60),
                (0x60, 0x60),
                (0xc0, 0x60),
                (0x120, 0x60),
                (0, 0xc0),
                (0x60, 0xc0),
                (0xc0, 0xc0),
                (0x120, 0xc0),
            ]
        );
        assert_eq!(group_cells(0x180, 0x120)[7], (0x180 + 0x120, 0x180));
    }

    #[test]
    fn the_atlas_is_cut_into_the_registered_groups() {
        // An atlas 0x300 wide and 0x2a0 tall holds every set.
        let image = IconImage::new(0x300, 0x2a0, 4);
        let mut seen = Vec::new();
        let table = build_group_icons(&image, |cell| {
            seen.push((cell.width, cell.height));
            seen.len() as i32
        });
        assert_eq!(seen.len(), 40);
        assert!(seen.iter().all(|&size| size == (0x60, 0x60)));
        assert_eq!(table[0], [1, 2, 3, 4, 5, 6, 7, 8]);
        // Group 1 has no set: its whole row stays 0.
        assert_eq!(table[1], [0; 8]);
        assert_eq!(table[2][0], 9);
        assert_eq!(table[3][0], 17);
        assert_eq!(table[4][0], 25);
        assert_eq!(table[5][0], 33);
        assert_eq!(table[5][7], 40);
        // A short image skips the cells it does not hold.
        let small = IconImage::new(0x60, 0x120, 4);
        let table = build_group_icons(&small, |_| 1);
        // The first and fifth cells (the second row's start) fit.
        assert_eq!(table[0], [1, 0, 0, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn the_single_icons_are_six_big_cells() {
        let image = IconImage::new(0x240, 0x360, 4);
        let mut sizes = Vec::new();
        let table = build_single_icons(&image, |cell| {
            sizes.push((cell.width, cell.height));
            sizes.len() as i32
        });
        assert_eq!(sizes, vec![(0x120, 0x120); 6]);
        assert_eq!(table, [1, 2, 3, 4, 5, 6]);
        let empty = build_single_icons(&IconImage::new(1, 1, 4), |_| 9);
        assert_eq!(empty, [0; 6]);
    }

    #[test]
    fn the_resources_build_once_and_rebuild_on_a_unit_change() {
        let mut resources = SkillResources::default();
        assert!(resources.ensure());
        assert!(!resources.ensure());
        assert_eq!(resources.builds, 1);
        assert!(resources.built && resources.hook_installed);
        resources.rebuild();
        assert_eq!(resources.builds, 2);
        resources.finalize();
        assert!(!resources.built);
        assert!(resources.ensure());
        assert_eq!((ICON_UNITS, SINGLE_ICON_UNITS), (8, 24));
        assert_eq!(SKILL_GUI_UNITS, (44, 20));
        assert_eq!(SKILL_GUI_FLAGS, 0x133);
    }

    #[test]
    fn the_window_constructor_takes_the_mode_and_the_ring() {
        let plan = SkillWindowPlan::new(0, false);
        assert_eq!((plan.radius, plan.scale), (0x40, 0.9));
        assert!(!plan.mode_set);
        assert_eq!((plan.layout, plan.group_count), (1, 4));
        assert_eq!(plan.groups, [1, 0, 2, 3, 4, 5]);
        assert_eq!(plan.slots, [1, 0, 2, 3, 2, 3]);
        let inset = SkillWindowPlan::new(3, true);
        assert_eq!((inset.radius, inset.scale, inset.mode), (0x48, 1.25, 3));
        assert!(inset.mode_set);
        // A negative mode is not set, as the original tests `0 < mode`.
        assert!(!SkillWindowPlan::new(-1, false).mode_set);
    }

    #[test]
    fn the_compact_window_copies_or_clears_six_words() {
        let mut data = vec![0u8; 0x100];
        for (index, offset) in COMPACT_WORD_OFFSETS.iter().enumerate() {
            data[*offset] = index as u8 + 1;
            data[*offset + 1] = 0x10;
        }
        let plan = CompactSkillPlan::attach(&data, false);
        assert_eq!(plan.words, [0x1001, 0x1002, 0x1003, 0x1004, 0x1005, 0x1006]);
        assert!(!plan.cleared);
        let cleared = CompactSkillPlan::attach(&data, true);
        assert_eq!(cleared.words, [0; 6]);
        assert!(cleared.cleared);
        // A short data block reads zero past its end.
        let short = CompactSkillPlan::attach(&data[..0xc4], false);
        assert_eq!(short.words, [0x1001, 0, 0, 0, 0, 0]);
        assert_eq!(short.packed(), short.words);
    }
}
