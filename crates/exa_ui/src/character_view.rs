//! The character screen's 3D panel: the private `_Inventory` scene, its
//! camera and background, and what a drag onto or off the figure asks the
//! character's record to do.
//!
//! The figure stands at (0, 100, 0); the camera looks from a point 410 cm
//! away along (200, −60, −360), with a 30° field of view, and its first
//! choice of super sampling comes from the settings (1, then two more than
//! the setting below 4).
//! The panel's background is a vertical wash built from the skin scheme's
//! colour: boosted by 1.4, then red takes 0.38 of the green, green 0.4 and
//! blue 0.45. A drag onto the figure equips apparel, takes a weapon or a
//! shield, or takes a body's core crystal in hand; a drag off the figure
//! removes the worn item under the pointer.
//!
//! Evidence: the character-view constructor, its background callback, its
//! hover and drag procedures, and the record routines they call.

/// The private scene's name.
pub const SCENE_NAME: &str = "_Inventory";
/// The scene's view distance.
pub const SCENE_DISTANCE: f32 = 100.0;
/// The camera's distance from the figure, in centimetres.
pub const CAMERA_DISTANCE: f32 = 410.0;
/// The camera's field of view in degrees.
pub const FIELD_OF_VIEW: f32 = 30.0;
/// The figure's point.
pub const FIGURE_POINT: [f32; 3] = [0.0, 100.0, 0.0];
/// The camera's unnormalised eye point.
pub const CAMERA_EYE: [f32; 3] = [-200.0, 160.0, 360.0];
/// The background texture.
pub const BACKGROUND: &str = "wndbkg02.rfi";
/// The scheme colour's divisor.
pub const SCHEME_DIVISOR: f32 = 240.0;
/// The background's brightening factor.
pub const BACKGROUND_BOOST: f32 = 1.4;

/// The camera's eye point at the constructor's distance: the direction
/// from the eye point to the figure scaled to [`CAMERA_DISTANCE`]. The
/// original subtracts, takes the length in single precision and scales the
/// direction, so this mirrors that order.
pub fn camera_offset(eye: [f32; 3], target: [f32; 3], distance: f32) -> [f32; 3] {
    let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let scale = distance / len;
    [d[0] * scale, d[1] * scale, d[2] * scale]
}

/// The camera's offset for the constructor.
pub fn default_camera_offset() -> [f32; 3] {
    camera_offset(CAMERA_EYE, FIGURE_POINT, CAMERA_DISTANCE)
}

/// The super-sampling level the view asks for at start: 1 at or below 0,
/// two more than the setting for 1–3, and 4 above.
pub fn supersample_level(setting: i32) -> i32 {
    if setting < 1 {
        1
    } else if setting < 4 {
        setting + 2
    } else {
        4
    }
}

/// The background's colour bytes from a scheme colour: the scheme's three
/// bytes over 240, optionally brightened by [`BACKGROUND_BOOST`], then the
/// wash (red 0.38 of green, green 0.4, blue 0.45), rounded to even and
/// stored as bytes (the original stores the low byte of the rounded value).
pub fn background_rgba(scheme: [u8; 3], boost: bool) -> [u8; 4] {
    let k = if boost { BACKGROUND_BOOST } else { 1.0 };
    let g = f32::from(scheme[1]) / SCHEME_DIVISOR * k;
    let b = f32::from(scheme[2]) / SCHEME_DIVISOR * k;
    let wash = [g * 0.38, g * 0.4, b * 0.45];
    let byte = |v: f32| (v * 255.0).round_ties_even() as i64 as u8;
    [byte(wash[0]), byte(wash[1]), byte(wash[2]), 0xFF]
}

/// The background's brighter copy, drawn over the panel: each channel is
/// the same wash rounded to even at 280 instead of 255.
pub fn background_bright_rgba(scheme: [u8; 3], boost: bool) -> [u8; 4] {
    let k = if boost { BACKGROUND_BOOST } else { 1.0 };
    let g = f32::from(scheme[1]) / SCHEME_DIVISOR * k;
    let b = f32::from(scheme[2]) / SCHEME_DIVISOR * k;
    let wash = [g * 0.38, g * 0.4, b * 0.45];
    let byte = |v: f32| (v * 280.0).round_ties_even() as i64 as u8;
    [byte(wash[0]), byte(wash[1]), byte(wash[2]), 0xFF]
}

/// The render task's camera follow: `current` climbs towards `target` by
/// `rate · 0.003 · step`, at most the target. A current already at or past
/// the target stays.
pub fn camera_follow(current: f32, target: f32, rate: f32, step: f32) -> f32 {
    if current < target {
        (current + rate * 0.003 * step).min(target)
    } else {
        current
    }
}

/// What a drag onto the figure does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureDrop {
    /// Nothing in the item's sections matches.
    None,
    /// Wear the apparel.
    Apparel,
    /// Take the weapon into the hands.
    Weapon,
    /// Take the shield into the off hand.
    Shield,
    /// Take the body's core crystal into a hand.
    Hand,
}

/// What a drag onto the figure may do. `race_flags` and `body_flags` are
/// the figure's body words; `has_apparel`, `has_weapon`, `has_shield` and
/// `has_light` say which item sections it carries. Apparel is only
/// considered for a body flagged `4`; a weapon and a shield follow it, a
/// light-only item is left to the core check, and the core crystals are
/// `431`/`451`/`452`/`499` for a body whose race takes them.
pub fn drop_kind(input: FigureDropInput) -> FigureDrop {
    if input.race_flags & 4 != 0 && input.has_apparel {
        return FigureDrop::Apparel;
    }
    if input.has_weapon {
        return FigureDrop::Weapon;
    }
    if input.has_shield {
        return FigureDrop::Shield;
    }
    if input.takes_core && input.core_crystal {
        return FigureDrop::Hand;
    }
    FigureDrop::None
}

/// The facts a drop onto the figure needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FigureDropInput {
    pub race_flags: u32,
    pub body_flags: u32,
    pub has_apparel: bool,
    pub has_weapon: bool,
    pub has_shield: bool,
    pub has_light: bool,
    /// The body's race takes a core crystal.
    pub takes_core: bool,
    /// The dragged item is a core crystal of the body's kind.
    pub core_crystal: bool,
    /// The figure's view has the "no changes" flag.
    pub frozen: bool,
}

/// Whether the figure accepts a drag of an item with the given section
/// flags at all: an apparel drop needs a body that wears it; a weapon or
/// shield with the withheld bit needs the matching body permission; a
/// light-less item is accepted only for the AL crystal on a body that
/// takes it while nothing is shown.
pub fn accepts_drag(input: AcceptDragInput) -> bool {
    if input.refused_by_callback {
        return false;
    }
    if input.apparel {
        return !input.view_no_changes && input.race_flags & 4 != 0;
    }
    if input.weapon {
        if input.section_flags & 0x0200_0000 == 0 {
            if input.section_flags & 0x0400_0000 == 0 {
                return true;
            }
            return input.body_flags & 0x0002_0000 != 0;
        }
        return input.body_flags & 0x0001_0000 != 0;
    }
    if input.shield {
        if input.section_flags & 0x0200_0000 == 0 {
            return true;
        }
        return input.body_flags & 0x0001_0000 != 0;
    }
    input.id == 500 && input.race_flags & 0x2000 != 0 && input.shown_item == 0
}

/// The facts the drag-over test needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AcceptDragInput {
    pub refused_by_callback: bool,
    pub id: u32,
    pub apparel: bool,
    pub weapon: bool,
    pub shield: bool,
    pub section_flags: u32,
    pub race_flags: u32,
    pub body_flags: u32,
    pub view_no_changes: bool,
    /// The item the panel currently shows, 0 for none.
    pub shown_item: u32,
}

/// The 1-based worn reference under a node among the figure's worn list,
/// as the hover test finds it: the first entry whose object is `item`.
pub fn worn_index(entries: &[Option<u32>], item: u32) -> Option<usize> {
    entries
        .iter()
        .position(|entry| *entry == Some(item))
        .map(|at| at + 1)
}

/// What the figure shows after hovering the item under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hover {
    /// The item and flags to remember while hovering, `None` when the
    /// pointer left the items.
    pub shown: Option<(u32, u32)>,
    /// The icon handle the panel holds, 0 when it resolved none.
    pub icon: usize,
    /// The worn entry index the hovered node is, 0 for none.
    pub worn: usize,
}

/// The hover state for a hovered item: an item whose icon resolves is
/// remembered and shown; an item whose icon does not resolve clears the
/// shown item and asks for a redraw.
pub fn hover(showing: Option<(u32, u32)>, icon: usize, worn: usize) -> Hover {
    if icon < 1 {
        Hover {
            shown: None,
            icon: 0,
            worn: 0,
        }
    } else {
        Hover {
            shown: showing,
            icon,
            worn,
        }
    }
}

/// What a drag release over the figure does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureDragEnd {
    /// Nothing was being dragged.
    None,
    /// Take the worn item at the 1-based index off.
    TakeOffWorn(usize),
    /// The dragged item is the one the panel shows: take it off by id.
    TakeOffShown,
    /// Give the dragged item back to the pack.
    GiveBack,
}

/// The drag release decision. `worn` is the 1-based worn entry under the
/// pointer (0 for none), `from_pack` says the drag started in the pack.
pub fn drag_end(worn: usize, shows_dragged: bool, from_pack: bool) -> FigureDragEnd {
    if worn > 0 {
        return FigureDragEnd::TakeOffWorn(worn);
    }
    if shows_dragged {
        FigureDragEnd::TakeOffShown
    } else if from_pack {
        FigureDragEnd::GiveBack
    } else {
        FigureDragEnd::None
    }
}

/// A view form in the GUI's list, as the character-view lookup sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewRef {
    pub visible: bool,
    pub owner: Option<u32>,
    pub record: Option<u32>,
}

/// The first visible form of the character view's class owned by `owner`.
pub fn find_owner(views: &[ViewRef], owner: u32) -> Option<usize> {
    views
        .iter()
        .position(|view| view.visible && view.owner == Some(owner))
}

/// The record of the first visible character view.
pub fn first_record(views: &[ViewRef]) -> Option<u32> {
    views.iter().find(|view| view.visible)?.record
}

/// The item a view's card callback opens: the shown item and its flags.
pub fn current_card(shown_item: u32, flags: u32) -> Option<(u32, u32)> {
    (shown_item > 0).then_some((shown_item, flags))
}

/// Whether the pointer is on the figure's right half while the character
/// knows technique 8 of the mind skill (the insight the view tests before an
/// item is taken). `skill_total` is the skill point word; levels are 5000
/// points each and the technique bytes start two bytes after its level
/// count.
pub fn insight_blocks(skill_total: u16, techniques: &[u8], cursor_x: i32, half_width: i32) -> bool {
    let levels = i32::from(skill_total) / 5000;
    let known = levels >= 1
        && techniques
            .iter()
            .take(levels as usize)
            .any(|technique| *technique == 8);
    known && cursor_x > half_width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_looks_up_its_owner_and_opens_its_shown_item() {
        let views = [
            ViewRef {
                visible: false,
                owner: Some(3),
                record: Some(9),
            },
            ViewRef {
                visible: true,
                owner: Some(4),
                record: Some(9),
            },
            ViewRef {
                visible: true,
                owner: Some(3),
                record: Some(8),
            },
        ];
        assert_eq!(find_owner(&views, 3), Some(2));
        assert_eq!(find_owner(&views, 7), None);
        assert_eq!(first_record(&views), Some(9));
        assert_eq!(current_card(0, 0x10), None);
        assert_eq!(current_card(7, 0x10), Some((7, 0x10)));
        assert!(!insight_blocks(0, &[8], 100, 50));
        assert!(insight_blocks(5000, &[8], 100, 50));
        assert!(!insight_blocks(5000, &[8], 50, 50));
        assert!(!insight_blocks(10000, &[7, 7], 100, 50));
    }

    #[test]
    fn the_camera_sits_410_centimetres_from_the_figure() {
        let offset = default_camera_offset();
        let len = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        assert!((len - CAMERA_DISTANCE).abs() < 0.001);
        assert!(offset[0] > 190.0 && offset[0] < 200.0);
        assert!(offset[1] < 0.0 && offset[1] > -65.0);
        assert!(offset[2] < -350.0);
        assert_eq!(supersample_level(0), 1);
        assert_eq!(supersample_level(1), 3);
        assert_eq!(supersample_level(3), 5);
        assert_eq!(supersample_level(7), 4);
    }

    #[test]
    fn the_background_wash_comes_from_the_green_channel() {
        // 240 / 240 = 1, boosted to 1.4: red 0.532, green 0.56, blue 0.63.
        let c = background_rgba([10, 240, 200], true);
        assert_eq!(c[0], 136);
        assert_eq!(c[1], 143);
        assert_eq!(c[2], 134);
        assert_eq!(c[3], 255);
        // Without the boost: 0.38, 0.4 and 0.375.
        assert_eq!(background_rgba([0, 240, 200], false), [97, 102, 96, 255]);
        let b = background_bright_rgba([0, 240, 200], false);
        assert_eq!(&b[..3], &[106, 112, 105]);
        assert_eq!(camera_follow(1.0, 5.0, 100.0, 2.0), 1.6);
        assert_eq!(camera_follow(4.9, 5.0, 100.0, 2.0), 5.0);
        assert_eq!(camera_follow(6.0, 5.0, 100.0, 2.0), 6.0);
    }

    #[test]
    fn a_drop_picks_the_item_section() {
        let mut input = FigureDropInput::default();
        assert_eq!(drop_kind(input), FigureDrop::None);
        input.race_flags = 4;
        input.has_apparel = true;
        assert_eq!(drop_kind(input), FigureDrop::Apparel);
        input.has_apparel = false;
        input.has_weapon = true;
        assert_eq!(drop_kind(input), FigureDrop::Weapon);
        input.has_weapon = false;
        input.has_shield = true;
        assert_eq!(drop_kind(input), FigureDrop::Shield);
        input.has_shield = false;
        input.takes_core = true;
        input.core_crystal = true;
        assert_eq!(drop_kind(input), FigureDrop::Hand);
        input.race_flags = 0;
        input.has_apparel = true;
        input.takes_core = false;
        input.core_crystal = false;
        assert_eq!(drop_kind(input), FigureDrop::None);
    }

    #[test]
    fn the_drag_over_test_follows_the_sections() {
        let mut a = AcceptDragInput::default();
        assert!(!accepts_drag(a));
        a.refused_by_callback = true;
        a.weapon = true;
        assert!(!accepts_drag(a));
        a.refused_by_callback = false;
        assert!(accepts_drag(a));
        a.section_flags = 0x0200_0000;
        assert!(!accepts_drag(a));
        a.body_flags = 0x0001_0000;
        assert!(accepts_drag(a));
        a = AcceptDragInput {
            apparel: true,
            race_flags: 4,
            ..Default::default()
        };
        assert!(accepts_drag(a));
        a.view_no_changes = true;
        assert!(!accepts_drag(a));
        let al = AcceptDragInput {
            id: 500,
            race_flags: 0x2000,
            ..Default::default()
        };
        assert!(accepts_drag(al));
        assert!(!accepts_drag(AcceptDragInput {
            id: 500,
            race_flags: 0x2000,
            shown_item: 3,
            ..Default::default()
        }));
        assert_eq!(worn_index(&[None, Some(7), Some(8)], 8), Some(3));
        assert_eq!(worn_index(&[None], 8), None);
    }

    #[test]
    fn hovering_and_releasing_on_the_figure() {
        assert_eq!(
            hover(Some((7, 0x10)), 4, 2),
            Hover {
                shown: Some((7, 0x10)),
                icon: 4,
                worn: 2
            }
        );
        assert_eq!(
            hover(Some((7, 0x10)), 0, 2),
            Hover {
                shown: None,
                icon: 0,
                worn: 0
            }
        );
        assert_eq!(drag_end(2, false, true), FigureDragEnd::TakeOffWorn(2));
        assert_eq!(drag_end(0, true, false), FigureDragEnd::TakeOffShown);
        assert_eq!(drag_end(0, false, true), FigureDragEnd::GiveBack);
        assert_eq!(drag_end(0, false, false), FigureDragEnd::None);
    }
}
