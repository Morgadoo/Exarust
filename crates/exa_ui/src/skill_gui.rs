//! The skill GUI's resources (`SG_GUI_Skills`): the two icon atlases cut
//! from `sgsklicons.rfi` and `skillicon.rfi`, the `skillgui.rfi` background
//! and the once-only build. The screens themselves are
//! [`crate::skill::SkillScreen`] (its page constructor and its record
//! attach are this unit's) and [`crate::compact_skill`].
//!
//! `sgsklicons` holds 96-pixel cells: the one at the origin is added first
//! and kept apart, then five groups (group 1, Armour, has no set) are cut
//! from their origins as two rows of four, the top row giving the group's
//! techniques 5 to 8 and the row below techniques 1 to 4. Its atlas draws
//! icons 8 units wide, sharpened when that is below the cell. `skillicon`
//! is cut into six 288-pixel icons for an atlas of 24-unit icons, whose
//! handles follow the cut order. `skillgui` is resized to 44 × 20 units and
//! becomes its own texture.
//!
//! Evidence: the atlas builder, the atlas setup and the once-only unit
//! initialiser, read from the instructions.

use crate::icon_atlas::IconImage;

/// The skill-icon atlas's file name.
pub const SKILL_ICON_ATLAS: &str = "sgsklicons.rfi";
/// The large single-skill atlas's file name.
pub const SINGLE_ICON_ATLAS: &str = "skillicon.rfi";
/// The skill screen's background texture.
pub const SKILL_GUI: &str = "skillgui.rfi";
/// A cell of the skill-icon atlas image, in pixels.
pub const ICON_CELL: i32 = 0x60;
/// A cell of the single-skill atlas image, in pixels.
pub const SINGLE_CELL: i32 = 0x120;
/// The cell the skill-icon atlas adds first, outside the group tables.
pub const FIRST_ICON: (i32, i32) = (0, 0);
/// The groups the atlas builder cuts, with each set's origin (x, y), in
/// the original's order. Group 1 has no set of its own.
pub const ICON_SETS: [(usize, i32, i32); 5] = [
    (0, 0, 0x60),
    (2, 0, 0x120),
    (3, 0, 0x1e0),
    (4, 0x180, 0x1e0),
    (5, 0x180, 0x120),
];
/// The six single-skill icons' origins, in the original's (handle) order.
pub const SINGLE_ICONS: [(i32, i32); 6] = [
    (0, 0x240),
    (0x120, 0x240),
    (0, 0x120),
    (0x120, 0x120),
    (0, 0),
    (0x120, 0),
];
/// The skill-icon atlas's icon side in GUI units.
pub const ICON_UNITS: i32 = 8;
/// The single-skill atlas's icon side in GUI units.
pub const SINGLE_ICON_UNITS: i32 = 24;
/// The skill-icon atlas's pixel format word.
pub const ICON_FORMAT: u32 = 0x100_6208;
/// The single-skill atlas's pixel format word (it also sharpens always and
/// sets the usual 0x203 blend).
pub const SINGLE_FORMAT: u32 = 0x100_2008;
/// The background's size in units, 44 by 20.
pub const SKILL_GUI_UNITS: (i32, i32) = (44, 20);
/// The background texture's flags: the engine's default texture flags
/// (0x33) with 0x100.
pub const SKILL_GUI_FLAGS: u32 = 0x33 | 0x100;

/// The rounded ring program's fragment source: the radial alpha from the
/// vertex texture coordinates, squared, with the shader's `Colour`.
pub const RING_SHADER: &str = "in vec2 vrTex;\r\nout vec4 FragColor;\r\nuniform vec4 Colour;\r\nvoid main() {\r\nvec2 s = abs(0.5 - vrTex);\r\nfloat a = max(1.0 - dot(s, s) * 4.0, 0.0);\r\na = min(a * 4.0, 1.0);\r\na = a * a;\r\nFragColor.rgb = Colour.rgb;FragColor.a = Colour.a * a;}";

/// The orbit bars' fragment source: a stroked arc whose cut is `Prog ×
/// 8.3` across the square, multiplied by the texture's alpha.
pub const BAR_SHADER: &str = "in vec2 vrTex;\r\nout vec4 FragColor;\r\nuniform vec4 Colour;\r\nuniform float Prog;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nfloat a = abs(vrTex.x * 2.0 - 0.5);\r\na = a * 16.0;\r\na = min(Prog * 8.3 - a, 1.0);\r\na = a * texture(Tex, vrTex).a;\r\nFragColor.rgb = Colour.rgb;FragColor.a = Colour.a * a;}";

/// The projection uniform both skill programs share.
pub const M_PRJ: &str = "MPrj";
/// The colour uniform both skill programs share.
pub const COLOUR: &str = "Colour";
/// The bar program's texture sampler.
pub const TEX: &str = "Tex";
/// The bar program's progress uniform.
pub const PROG: &str = "Prog";
/// The value `MPrj` is set to: 2, 0, 0, 0.
pub const M_PRJ_VALUE: [f32; 4] = [2.0, 0.0, 0.0, 0.0];
/// The reveal sound the screen's setup loads.
pub const SKILL_SHOW_SOUND: &str = "SkillShow_01";
/// The reveal sound's play arguments: the environment handle, the sound,
/// a zero position record, 0.03, 1.0 and the flags word 0x4060 (the last
/// three are guesses at the sound call's gain, pitch and flags).
pub const SKILL_SHOW_GAIN: f32 = 0.03;
pub const SKILL_SHOW_PITCH: f32 = 1.0;
pub const SKILL_SHOW_FLAGS: u32 = 0x4060;

/// One of the screen's two shader programs and the uniforms it looks up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillShaderProgram {
    pub source: &'static str,
    pub projection: &'static str,
    pub colour: &'static str,
    /// The texture sampler, when the program has one.
    pub texture: Option<&'static str>,
    /// The progress uniform, when the program has one.
    pub progress: Option<&'static str>,
}

/// The setup routine's two programs and the reveal sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillPrograms {
    pub ring: SkillShaderProgram,
    pub bar: SkillShaderProgram,
    pub sound: &'static str,
}

/// The skill screen's one-time program setup as [`crate::skill_gui`] runs
/// it: the rounded ring program with `MPrj` = (2, 0, 0, 0) and `Colour`,
/// then the bar program with the same projection, its `Tex` set to unit 0,
/// `Colour` and `Prog`, then the reveal sound handle.
pub fn skill_programs() -> SkillPrograms {
    SkillPrograms {
        ring: SkillShaderProgram {
            source: RING_SHADER,
            projection: M_PRJ,
            colour: COLOUR,
            texture: None,
            progress: None,
        },
        bar: SkillShaderProgram {
            source: BAR_SHADER,
            projection: M_PRJ,
            colour: COLOUR,
            texture: Some(TEX),
            progress: Some(PROG),
        },
        sound: SKILL_SHOW_SOUND,
    }
}

/// Whether the skill-icon atlas sharpens its resized icons: its icon side
/// (8 units) is below the 96-pixel cell.
pub fn icon_sharpened(unit: i32) -> bool {
    ICON_UNITS * unit < ICON_CELL
}

/// The once-only resource state: the first initialisation builds the
/// atlases and chains the build after the previous unit-change hook, which
/// a later unit change runs again (the hook calls the previous one first).
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

    /// A unit change rebuilds the atlases (the existing atlases are
    /// cleared and take the new icon size).
    pub fn rebuild(&mut self) {
        self.built = true;
        self.builds += 1;
    }
}

/// The eight cells of one skill group at an atlas origin, by technique
/// (index 0 is technique 1): techniques 1 to 4 are the row 96 pixels
/// down, 96 pixels apart, and techniques 5 to 8 the row at the origin.
pub fn group_cells(x: i32, y: i32) -> [(i32, i32); 8] {
    let low = y + ICON_CELL;
    [
        (x, low),
        (x + ICON_CELL, low),
        (x + 2 * ICON_CELL, low),
        (x + 3 * ICON_CELL, low),
        (x, y),
        (x + ICON_CELL, y),
        (x + 2 * ICON_CELL, y),
        (x + 3 * ICON_CELL, y),
    ]
}

/// The handles of the skill-icon atlas: six groups of eight techniques
/// (group 1 keeps 0, as the original never cuts a set for it).
pub type SkillIcons = [[i32; 8]; 6];

/// The skill-icon atlas's handles: the first cell's and the groups'.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillIconHandles {
    pub first: i32,
    pub groups: SkillIcons,
}

/// Whether a `side`-pixel cell at (x, y) lies inside the image.
fn fits(image: &IconImage, x: i32, y: i32, side: i32) -> bool {
    x >= 0 && y >= 0 && x + side <= i32::from(image.width) && y + side <= i32::from(image.height)
}

/// Cuts the skill-icon atlas image as the original does and hands each
/// cell to `add`, which returns its handle: the first cell, then every
/// set's eight cells in the original's order (the origin row first). A
/// cell outside the image is skipped and keeps handle 0, so a short image
/// is total (the original copies whatever the region holds).
pub fn build_group_icons(
    image: &IconImage,
    mut add: impl FnMut(&IconImage) -> i32,
) -> SkillIconHandles {
    let side = ICON_CELL as u16;
    let mut cut = |x: i32, y: i32| {
        if fits(image, x, y, ICON_CELL) {
            add(&image.region(x, y, side, side))
        } else {
            0
        }
    };
    let mut handles = SkillIconHandles {
        first: cut(FIRST_ICON.0, FIRST_ICON.1),
        ..SkillIconHandles::default()
    };
    for &(group, x, y) in &ICON_SETS {
        let cells = group_cells(x, y);
        for technique in (4..8).chain(0..4) {
            let (cell_x, cell_y) = cells[technique];
            handles.groups[group][technique] = cut(cell_x, cell_y);
        }
    }
    handles
}

/// Cuts the six single-skill icons out of their atlas image and hands each
/// to `add`. The original keeps no handle (they follow the cut order);
/// the returned ones are `add`'s, 0 for a cell outside the image.
pub fn build_single_icons(image: &IconImage, mut add: impl FnMut(&IconImage) -> i32) -> [i32; 6] {
    let side = SINGLE_CELL as u16;
    let mut table = [0i32; 6];
    for (slot, (x, y)) in SINGLE_ICONS.into_iter().enumerate() {
        if fits(image, x, y, SINGLE_CELL) {
            table[slot] = add(&image.region(x, y, side, side));
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_s_first_techniques_are_the_lower_row() {
        assert_eq!(
            group_cells(0, 0x60),
            [
                (0, 0xc0),
                (0x60, 0xc0),
                (0xc0, 0xc0),
                (0x120, 0xc0),
                (0, 0x60),
                (0x60, 0x60),
                (0xc0, 0x60),
                (0x120, 0x60),
            ]
        );
        assert_eq!(group_cells(0x180, 0x120)[3], (0x180 + 0x120, 0x180));
    }

    #[test]
    fn the_atlas_is_cut_into_the_first_cell_and_the_sets() {
        // An atlas 0x300 wide and 0x2a0 tall holds every set.
        let image = IconImage::new(0x300, 0x2a0, 4);
        let mut seen = Vec::new();
        let handles = build_group_icons(&image, |cell| {
            seen.push((cell.width, cell.height));
            seen.len() as i32
        });
        assert_eq!(seen.len(), 41);
        assert!(seen.iter().all(|&size| size == (0x60, 0x60)));
        assert_eq!(handles.first, 1);
        // The origin row (techniques 5–8) is cut before the row below.
        assert_eq!(handles.groups[0], [6, 7, 8, 9, 2, 3, 4, 5]);
        // Group 1 has no set: its whole row stays 0.
        assert_eq!(handles.groups[1], [0; 8]);
        assert_eq!(handles.groups[2][4], 10);
        assert_eq!(handles.groups[3][4], 18);
        assert_eq!(handles.groups[4][4], 26);
        assert_eq!(handles.groups[5][4], 34);
        assert_eq!(handles.groups[5][3], 41);
        // A short image skips the cells it does not hold.
        let small = IconImage::new(0x60, 0x120, 4);
        let handles = build_group_icons(&small, |_| 1);
        assert_eq!(handles.first, 1);
        // Group 0's first column: techniques 1 (y 0xc0) and 5 (y 0x60).
        assert_eq!(handles.groups[0], [1, 0, 0, 0, 1, 0, 0, 0]);
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
        assert_eq!((ICON_UNITS, SINGLE_ICON_UNITS), (8, 24));
        assert_eq!(SKILL_GUI_UNITS, (44, 20));
        assert_eq!(SKILL_GUI_FLAGS, 0x133);
        // 8 units of 11 pixels are below the 96-pixel cell; of 12 they are not.
        assert!(icon_sharpened(11));
        assert!(!icon_sharpened(12));
    }

    /// The installed skill atlases hold the cells the cut reads (skipped
    /// without `EXANIMA_DIR`).
    #[test]
    fn the_installed_skill_atlases_hold_their_cells() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let small = exa_formats::rfi::Rfi::parse(game.texture(SKILL_ICON_ATLAS).unwrap()).unwrap();
        assert!(small.header.width >= 0x300);
        assert!(small.header.height >= 0x240);
        let single =
            exa_formats::rfi::Rfi::parse(game.texture(SINGLE_ICON_ATLAS).unwrap()).unwrap();
        assert!(single.header.width >= 0x240);
        assert!(single.header.height >= 0x360);
    }

    #[test]
    fn the_program_setup_binds_the_two_sources_and_the_sound() {
        let programs = skill_programs();
        assert!(programs.ring.source.contains("dot(s, s) * 4.0"));
        assert!(programs.ring.source.ends_with("Colour.a * a;}"));
        assert!(programs.bar.source.contains("Prog * 8.3 - a"));
        assert_eq!(programs.ring.projection, "MPrj");
        assert_eq!(programs.ring.colour, "Colour");
        assert_eq!(programs.ring.texture, None);
        assert_eq!(programs.bar.texture, Some("Tex"));
        assert_eq!(programs.bar.progress, Some("Prog"));
        assert_eq!(M_PRJ_VALUE, [2.0, 0.0, 0.0, 0.0]);
        assert_eq!(programs.sound, "SkillShow_01");
        assert_eq!(SKILL_SHOW_GAIN, 0.03);
        assert_eq!(SKILL_SHOW_FLAGS, 0x4060);
    }
}
