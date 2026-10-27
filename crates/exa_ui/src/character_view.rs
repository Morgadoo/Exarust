//! The character screen's 3D panel: the private `_Inventory` scene, its
//! camera and background, and what a drag onto or off the figure asks the
//! character's record to do.
//!
//! The figure stands at (0, 100, 0); the camera sits 410 cm from it along
//! (−200, 60, 360) and so looks along (200, −60, −360), with a 30° field of
//! view, and its first choice of super sampling comes from the settings (1,
//! then two more than the setting below 4, then 4). The panel's background
//! is a vertical wash built from the skin scheme's colour: each byte over
//! 240, decoded from sRGB when the renderer's sRGB targets are on (the
//! default) or brightened by 1.4 when they are off, then red takes 0.38 of
//! the green, green 0.4 and blue 0.45; the top corners are those shares of
//! 255 and the bottom ones of 280. A drag onto the figure wears apparel,
//! takes a weapon or a shield into a hand, or takes the body's core crystal;
//! a drag off the figure takes a held item out of its hand or takes a worn
//! item off.
//!
//! Evidence: the character-view constructor, its background and task
//! callbacks, its hover pick, drag procedures and drop, and the record
//! routines they call.

/// The private scene's name.
pub const SCENE_NAME: &str = "_Inventory";
/// The scene's view distance.
pub const SCENE_DISTANCE: f32 = 100.0;
/// The camera's distance from the figure, in centimetres.
pub const CAMERA_DISTANCE: f32 = 410.0;
/// The camera's field of view in degrees.
pub const FIELD_OF_VIEW: f32 = 30.0;
/// The figure's point, which the camera looks at.
pub const FIGURE_POINT: [f32; 3] = [0.0, 100.0, 0.0];
/// The point whose direction from the figure the camera takes.
pub const CAMERA_EYE: [f32; 3] = [-200.0, 160.0, 360.0];
/// The background texture.
pub const BACKGROUND: &str = "wndbkg02.rfi";
/// The scheme byte's scale, 1/240, as the double the original multiplies by.
pub const SCHEME_SCALE: f64 = 0.004_166_666_666_666_667;
/// The background's brightening factor without sRGB targets.
pub const BACKGROUND_BOOST: f32 = 1.4;

/// The camera's position: the figure plus its offset towards `eye` scaled
/// to `distance`. The offset is `eye − target`, its length is the single
/// precision `√((x² + z²) + y²)` the engine's vector length takes, and each
/// component is `target + offset · (distance / length)`.
pub fn camera_position(eye: [f32; 3], target: [f32; 3], distance: f32) -> [f32; 3] {
    let d = [eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]];
    let len = ((d[0] * d[0] + d[2] * d[2]) + (d[1] * d[1] + 0.0)).sqrt();
    let scale = distance / len;
    [
        target[0] + d[0] * scale,
        target[1] + d[1] * scale,
        target[2] + d[2] * scale,
    ]
}

/// The constructor's camera position.
pub fn default_camera_position() -> [f32; 3] {
    camera_position(CAMERA_EYE, FIGURE_POINT, CAMERA_DISTANCE)
}

/// The super-sampling level the view asks for at start: 1 at or below 0,
/// two more than the setting for 1–3 (so 5 for 3, as the comparison chain
/// gives), and 4 above.
pub fn supersample_level(setting: i32) -> i32 {
    if setting < 1 {
        1
    } else if setting < 4 {
        setting + 2
    } else {
        4
    }
}

/// The background's two vertex colours from a scheme colour: the top
/// corners' and the brighter bottom corners'. Each scheme byte is scaled
/// by [`SCHEME_SCALE`] in double precision and narrowed; with
/// `srgb_targets` each channel is then decoded from sRGB by the engine's
/// decode, otherwise multiplied by [`BACKGROUND_BOOST`]. The wash is red
/// `0.38·g`, green `0.4·g` and blue `0.45·b` (double products narrowed), and
/// each channel byte is `round(v · 255)` (or `· 280`), the single-precision
/// product rounded half to even with its low byte kept.
pub fn background_colours(scheme: [u8; 3], srgb_targets: bool) -> ([u8; 4], [u8; 4]) {
    let channel = |byte: u8| {
        let v = (f64::from(byte) * SCHEME_SCALE) as f32;
        if srgb_targets {
            exa_assets::human_skin::srgb_decode(v)
        } else {
            v * BACKGROUND_BOOST
        }
    };
    let (g, b) = (channel(scheme[1]), channel(scheme[2]));
    let wash = [
        (f64::from(g) * 0.38) as f32,
        (f64::from(g) * 0.4) as f32,
        (f64::from(b) * 0.45) as f32,
    ];
    let bytes = |scale: f32| {
        let byte = |v: f32| (v * scale).round_ties_even() as i64 as u8;
        [byte(wash[0]), byte(wash[1]), byte(wash[2]), 0xFF]
    };
    (bytes(255.0), bytes(280.0))
}

/// The view task's easing of the figure's race share (the value its
/// balance blend reads) back to the race's own: below the target it climbs
/// by `frame_ms · 0.003` (summed in double precision, narrowed) and never
/// passes the target (the processor's minimum, which gives the target for
/// a NaN sum); at or above the target, or for a NaN, it stays.
pub fn ease_race_share(current: f32, target: f32, frame_ms: f32) -> f32 {
    if current < target {
        let v = (f64::from(frame_ms) * 0.003 + f64::from(current)) as f32;
        if v < target { v } else { target }
    } else {
        current
    }
}

/// What a drop onto the figure does.
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

/// The facts a drop onto the figure needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FigureDropInput {
    /// The body's race word that carries the "wears apparel" bit `4`.
    pub race_flags: u32,
    pub has_apparel: bool,
    pub has_weapon: bool,
    pub has_shield: bool,
    /// The body takes the dragged item as its core crystal
    /// (`exa_items::possession::takes_core`).
    pub takes_core: bool,
}

/// What a drop onto the figure does, by the item's sections in the
/// original's order: apparel (only for a body whose race wears it), a
/// weapon, a shield, then the body's core crystal.
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
    if input.takes_core {
        return FigureDrop::Hand;
    }
    FigureDrop::None
}

/// Whether the figure accepts an item drag at all. Apparel needs a view
/// that allows changes and a race that wears it; a weapon flagged
/// `0x2000000` needs the body's permission `0x10000`, one flagged
/// `0x4000000` its permission `0x20000`, others pass; a shield flagged
/// `0x2000000` needs the permission `0x10000`; an item with none of those
/// sections passes only as the AL crystal (item 500) on a body whose race
/// core word has bit `0x2000`, while no item is shown.
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
    input.id == 500 && input.race_core_flags & 0x2000 != 0 && input.shown_icon == 0
}

/// The facts the drag-over test needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AcceptDragInput {
    pub refused_by_callback: bool,
    pub id: u32,
    pub apparel: bool,
    pub weapon: bool,
    pub shield: bool,
    /// The weapon's or shield's section flags.
    pub section_flags: u32,
    /// The race word with the "wears apparel" bit `4`.
    pub race_flags: u32,
    /// The race word with the core-crystal bits (`0x1000`, `0x2000`).
    pub race_core_flags: u32,
    /// The body's permission word.
    pub body_flags: u32,
    pub view_no_changes: bool,
    /// The icon of the item the panel currently shows, 0 for none.
    pub shown_icon: usize,
}

/// The 1-based hand of the held item whose scene node the pointer is
/// over, as the hover pick asks it for a node marked as a held item: the
/// first of the body's held attachments carrying `node`; `None` (0 in the
/// original) when none does.
pub fn held_index(held: &[Option<u32>], node: u32) -> Option<usize> {
    held.iter()
        .position(|entry| *entry == Some(node))
        .map(|at| at + 1)
}

/// The item the panel shows after a hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    /// The item and flags shown, `None` when the icon did not resolve.
    pub item: Option<(u32, u32)>,
    /// The icon the panel holds, 0 for none.
    pub icon: usize,
    /// Whether the icon's draw callback is added (`Some(true)`, the panel
    /// showed nothing before) or removed (`Some(false)`, it showed one).
    pub draw: Option<bool>,
}

/// Shows an item over the figure: an item whose icon resolves (`icon` > 0;
/// item 0 resolves none) is remembered with its flags, otherwise the shown
/// item is cleared. `previous` is the icon shown before.
pub fn show_item(previous: usize, item: u32, flags: u32, icon: usize) -> Shown {
    let icon = if item == 0 { 0 } else { icon };
    if icon < 1 {
        Shown {
            item: None,
            icon: 0,
            draw: (previous > 0).then_some(false),
        }
    } else {
        Shown {
            item: Some((item, flags)),
            icon,
            draw: (previous == 0).then_some(true),
        }
    }
}

/// The payload a drag started on the figure carries, if one starts. A
/// frozen view starts drags only from a held item (`hand` > 0); an item
/// whose icon does not resolve starts none. In a view with the item lock
/// on, an item flagged `0x80000000` is dragged as a payload no panel takes
/// (`u32::MAX`); everything else is an item payload (`0xFB01`).
pub fn drag_start(frozen: bool, hand: usize, icon: usize, locked: bool, flags: u32) -> Option<u32> {
    if frozen && hand == 0 {
        return None;
    }
    if icon < 1 {
        return None;
    }
    Some(if locked && flags & 0x8000_0000 != 0 {
        u32::MAX
    } else {
        0xFB01
    })
}

/// What the end of a drag from the figure does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureDragEnd {
    /// The drag was cancelled (a trade view refunds its price).
    Cancelled,
    /// Take the item in this 0-based hand out (the pack's held entry with
    /// the dragged icon goes when the pack is not the drop's source).
    TakeOutOfHand(usize),
    /// The dragged icon is the shown one: take the shown item off.
    TakeOffShown,
    /// Take the dragged item off; the pack gains its bulk when
    /// `charge_pack`.
    TakeOffDragged { charge_pack: bool },
}

/// The drag-end decision. `hand` is the 1-based hand under the drag (0 for
/// none), `shown_is_dragged` says the dragged icon is the one shown, and
/// `pack_takes` says a pack panel is open and is not where the drag went.
pub fn drag_end(
    cancelled: bool,
    hand: usize,
    shown_is_dragged: bool,
    pack_takes: bool,
) -> FigureDragEnd {
    if cancelled {
        FigureDragEnd::Cancelled
    } else if hand >= 1 {
        FigureDragEnd::TakeOutOfHand(hand - 1)
    } else if shown_is_dragged {
        FigureDragEnd::TakeOffShown
    } else {
        FigureDragEnd::TakeOffDragged {
            charge_pack: pack_takes,
        }
    }
}

/// A view form in the GUI's list, as the character-view lookup sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewRef {
    pub visible: bool,
    pub owner: Option<u32>,
    /// The view's pack panel.
    pub pack: Option<u32>,
    /// The view's shown record.
    pub record: Option<u32>,
    /// The view's panel has been loaded (its contents may be saved).
    pub loaded: bool,
}

/// The first visible form of the character view's class owned by `owner`.
pub fn find_owner(views: &[ViewRef], owner: u32) -> Option<usize> {
    views
        .iter()
        .position(|view| view.visible && view.owner == Some(owner))
}

/// The player's pack panel: the pack of the player's open view.
pub fn player_pack(views: &[ViewRef], player: u32) -> Option<u32> {
    views[find_owner(views, player)?].pack
}

/// The shown record of the first view of `owner`: the first visible view's
/// record, or `None` when no view is open.
pub fn first_record(views: &[ViewRef], owner: u32) -> Option<u32> {
    views[find_owner(views, owner)?].record
}

/// The views whose loaded panel is saved when the character view closes:
/// the open forms of the view's class with their loaded byte set. The
/// caller saves each one's pack panel.
pub fn panels_to_save(views: &[ViewRef]) -> Vec<usize> {
    views
        .iter()
        .enumerate()
        .filter(|(_, view)| view.loaded && view.pack.is_some())
        .map(|(index, _)| index)
        .collect()
}

/// The 1-based worn entry whose attachment is the pointer's scene node. A
/// node not marked as a held item (`marker` false) or a body with no worn
/// entries has none.
pub fn worn_index(worn: &[Option<u32>], node: u32, marker: bool) -> Option<usize> {
    if !marker {
        return None;
    }
    worn.iter()
        .position(|entry| *entry == Some(node))
        .map(|at| at + 1)
}

/// The card a press on the figure opens: the item under the pointer and its
/// flags, when there is one.
pub fn current_card(item: u32, flags: u32) -> Option<(u32, u32)> {
    (item != 0).then_some((item, flags))
}

/// Whether the pointer is on the figure's right half while the character
/// knows technique 8 of the skill the view reads (`points` and its
/// technique bytes; the block at the record's fifth skill slot, Insight, by
/// its place, a guess): one technique per 5000 points is looked at.
pub fn insight_blocks(points: u16, techniques: &[u8], cursor_x: i32, width: u32) -> bool {
    let levels = usize::from(points / 5000);
    let known = techniques
        .iter()
        .take(levels)
        .any(|technique| *technique == 8);
    known && ((width >> 1) as i32) < cursor_x
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
                pack: Some(9),
                record: Some(90),
                loaded: false,
            },
            ViewRef {
                visible: true,
                owner: Some(4),
                pack: Some(9),
                record: Some(91),
                loaded: true,
            },
            ViewRef {
                visible: true,
                owner: Some(3),
                pack: Some(8),
                record: Some(92),
                loaded: true,
            },
        ];
        assert_eq!(find_owner(&views, 3), Some(2));
        assert_eq!(find_owner(&views, 7), None);
        assert_eq!(player_pack(&views, 3), Some(8));
        assert_eq!(player_pack(&views, 7), None);
        assert_eq!(current_card(0, 0x10), None);
        assert_eq!(current_card(7, 0x10), Some((7, 0x10)));
        assert_eq!(first_record(&views, 3), Some(92));
        assert_eq!(first_record(&views, 7), None);
        // The invisible view is not open, so its loaded flag does not count.
        assert_eq!(panels_to_save(&views), vec![1, 2]);
        assert_eq!(worn_index(&[None, Some(4), Some(5)], 5, true), Some(3));
        assert_eq!(worn_index(&[None, Some(4)], 4, false), None);
        assert_eq!(worn_index(&[None, Some(4)], 9, true), None);
        assert!(!insight_blocks(0, &[8], 100, 100));
        assert!(insight_blocks(5000, &[8], 100, 100));
        assert!(!insight_blocks(5000, &[8], 50, 100));
        assert!(!insight_blocks(10000, &[7, 7], 100, 100));
    }

    #[test]
    fn the_camera_sits_410_centimetres_from_the_figure() {
        let eye = default_camera_position();
        let d = [eye[0], eye[1] - 100.0, eye[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!((len - CAMERA_DISTANCE).abs() < 0.001);
        // Behind and above the figure on the (−200, 60, 360) side.
        assert!(eye[0] < -190.0 && eye[0] > -200.0);
        assert!(eye[1] > 100.0 && eye[1] < 165.0);
        assert!(eye[2] > 350.0);
        assert_eq!(supersample_level(0), 1);
        assert_eq!(supersample_level(1), 3);
        assert_eq!(supersample_level(3), 5);
        assert_eq!(supersample_level(7), 4);
    }

    #[test]
    fn the_background_wash_comes_from_the_green_channel() {
        // Without sRGB targets: 240 × 1/240 = 1, boosted to 1.4: red 0.532,
        // green 0.56, blue (200/240 · 1.4) · 0.45 = 0.525.
        let (top, bottom) = background_colours([10, 240, 200], false);
        assert_eq!(top, [136, 143, 134, 255]);
        assert_eq!(&bottom[..3], &[149, 157, 147]);
        // With sRGB targets the full byte decodes to 1: red 0.38 · 255 =
        // 96.9, green 102, blue (0.6038…) · 0.45 · 255 = 69.3.
        let (top, _) = background_colours([0, 240, 200], true);
        assert_eq!(&top[..2], &[97, 102]);
        let blue = exa_assets::human_skin::srgb_decode((200.0 * SCHEME_SCALE) as f32);
        let expect = (((f64::from(blue) * 0.45) as f32) * 255.0).round_ties_even() as u8;
        assert_eq!(top[2], expect);
    }

    #[test]
    fn the_race_share_eases_up_to_its_target() {
        // 1 + 100 ms · 0.003 = 1.3.
        assert_eq!(
            ease_race_share(1.0, 5.0, 100.0),
            (100.0f64 * 0.003 + 1.0) as f32
        );
        assert_eq!(ease_race_share(4.9, 5.0, 100.0), 5.0);
        assert_eq!(ease_race_share(6.0, 5.0, 100.0), 6.0);
        assert!(ease_race_share(f32::NAN, 5.0, 1.0).is_nan());
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
        assert_eq!(drop_kind(input), FigureDrop::Hand);
        // A race that wears nothing ignores the apparel section.
        input = FigureDropInput {
            has_apparel: true,
            has_weapon: true,
            ..Default::default()
        };
        assert_eq!(drop_kind(input), FigureDrop::Weapon);
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
            race_core_flags: 0x2000,
            ..Default::default()
        };
        assert!(accepts_drag(al));
        // The AL test reads the core word, not the apparel one.
        assert!(!accepts_drag(AcceptDragInput {
            id: 500,
            race_flags: 0x2000,
            ..Default::default()
        }));
        assert!(!accepts_drag(AcceptDragInput {
            shown_icon: 3,
            ..al
        }));
        assert_eq!(held_index(&[None, Some(7), Some(8)], 8), Some(3));
        assert_eq!(held_index(&[None], 8), None);
    }

    #[test]
    fn showing_dragging_and_releasing_on_the_figure() {
        assert_eq!(
            show_item(0, 7, 0x10, 4),
            Shown {
                item: Some((7, 0x10)),
                icon: 4,
                draw: Some(true)
            }
        );
        assert_eq!(show_item(4, 7, 0x10, 5).draw, None);
        assert_eq!(
            show_item(4, 7, 0x10, 0),
            Shown {
                item: None,
                icon: 0,
                draw: Some(false)
            }
        );
        assert_eq!(show_item(0, 0, 0, 9).item, None);
        assert_eq!(drag_start(false, 0, 3, false, 0), Some(0xFB01));
        assert_eq!(drag_start(true, 0, 3, false, 0), None);
        assert_eq!(drag_start(true, 2, 3, false, 0), Some(0xFB01));
        assert_eq!(drag_start(false, 0, 0, false, 0), None);
        assert_eq!(drag_start(false, 0, 3, true, 0x8000_0000), Some(u32::MAX));
        assert_eq!(drag_end(true, 2, true, true), FigureDragEnd::Cancelled);
        assert_eq!(
            drag_end(false, 2, true, true),
            FigureDragEnd::TakeOutOfHand(1)
        );
        assert_eq!(drag_end(false, 0, true, true), FigureDragEnd::TakeOffShown);
        assert_eq!(
            drag_end(false, 0, false, true),
            FigureDragEnd::TakeOffDragged { charge_pack: true }
        );
    }
}
