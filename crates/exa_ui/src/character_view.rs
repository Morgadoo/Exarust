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

/// The 1-based item slot of the doll's attachments whose node is the
/// pointer's node. A node not marked as an attached item (`marker` false)
/// has none. The original walks the item's slots (the hands and their
/// further attached items) and answers the slot; the record's hand flags it
/// then reads are the four hand entries' flag words.
pub fn hand_index(worn: &[Option<u32>], node: u32, marker: bool) -> Option<usize> {
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

/// Whether the character knows technique 8 of the skill the view reads: one
/// technique per 5000 points is looked at, up to the techniques stored.
pub fn insight_known(points: u16, techniques: &[u8]) -> bool {
    let levels = usize::from(points / 5000);
    techniques
        .iter()
        .take(levels)
        .any(|technique| *technique == 8)
}

/// Whether the pointer is on the figure's right half while the character
/// knows technique 8 of the skill the view reads (`points` and its
/// technique bytes; the block at the record's fifth skill slot, Insight, by
/// its place, a guess).
pub fn insight_blocks(points: u16, techniques: &[u8], cursor_x: i32, width: u32) -> bool {
    insight_known(points, techniques) && ((width >> 1) as i32) < cursor_x
}

/// The direction, in the camera's space, the figure's node is turned to
/// each frame: a unit vector within the original's rounding. [?] The
/// routine that composes it with the camera's matrix is not traced beyond
/// its call, so the meaning is inferred.
pub const DOLL_FACING: [f32; 3] = [0.5, 0.7, 0.5];

/// The private scene's settings the constructor gives it: its view
/// distance (already [`SCENE_DISTANCE`]), the camera's field of view, the
/// two colours its environment is given, the environment's mode word and
/// the level written through the environment's light block. The colours'
/// roles are [?]: they are the two 12-byte colours the constructor passes
/// to the scene environment, and the last value is the one float the
/// constructor writes into its light block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DollScene {
    pub distance: f32,
    pub field_of_view: f32,
    pub ambient: [f32; 3],
    pub sky: [f32; 3],
    pub environment_mode: u32,
    pub light_level: f32,
}

/// The doll's private scene: a 100-unit view, a 30° camera, grey 0.5 and
/// 0.75 colours, environment mode 2 and a light level of 1.
pub fn doll_scene() -> DollScene {
    DollScene {
        distance: SCENE_DISTANCE,
        field_of_view: FIELD_OF_VIEW,
        ambient: [0.5; 3],
        sky: [0.75; 3],
        environment_mode: 2,
        light_level: 1.0,
    }
}

/// The figure's node direction each frame: the camera's rotation applied
/// to [`DOLL_FACING`] and normalised. The camera is at the view's eye and
/// the figure turns its node into that space, so the doll keeps facing the
/// camera as the camera orbits.
pub fn doll_facing(camera: bevy::math::Quat) -> bevy::math::Vec3 {
    let facing = bevy::math::Vec3::from(DOLL_FACING);
    (camera * facing).normalize_or_zero()
}

/// The view's per-frame counter after a frame: one less, never below zero.
pub fn frame_counter(value: i32) -> i32 {
    (value.wrapping_sub(1)).max(0)
}

/// A node of the figure's scene, as the hover pick walks it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickNode {
    /// The node's identity.
    pub node: u32,
    /// Its flags.
    pub flags: u32,
    /// The item it carries, 0 for none.
    pub item: u32,
    /// The carried item's flags.
    pub item_flags: u32,
    /// The carried item's settle word: a negative one is still settling.
    pub item_settle: i32,
    /// How far along the pick's ray the node's nearest face is, `None` when
    /// the ray misses it ([`nearest_face`] over the node's triangles).
    pub distance: Option<f32>,
    /// The doll's item slot the node belongs to, 1-based (the original
    /// follows the node's parent chain to a settled node and looks it up
    /// among the attached items).
    pub slot: Option<usize>,
}

/// The flags a pickable node carries: all of `0x89`.
pub const PICK_REQUIRED: u32 = 0x89;
/// A node flagged this is not picked.
pub const PICK_HIDDEN: u32 = 0x0400_0000;
/// An item flagged both of these, and settled, makes its node unpickable.
pub const PICK_ITEM_HIDDEN: u32 = 0x1200_0000;

/// Whether the hover pick looks at a node: its flags carry all of
/// [`PICK_REQUIRED`], it is not [`PICK_HIDDEN`], and its item is not
/// [`PICK_ITEM_HIDDEN`] while settled.
pub fn pickable(node: &PickNode) -> bool {
    node.flags & PICK_REQUIRED == PICK_REQUIRED
        && node.flags & PICK_HIDDEN != PICK_HIDDEN
        && !(node.item != 0
            && node.item_flags & PICK_ITEM_HIDDEN == PICK_ITEM_HIDDEN
            && node.item_settle >= 0)
}

/// The smallest determinant the meshes' ray callback takes (the float word
/// `0x33ffffd7`, just under 2⁻²³): a face seen edge-on, or from behind, is
/// not hit.
pub const FACE_DETERMINANT: f32 = f32::from_bits(0x33ff_ffd7);

/// The scene meshes' ray test of one face, as their ray callback runs it:
/// the unnormalised barycentric test, one-sided (the determinant
/// `e1 · (dir × e2)` must reach [`FACE_DETERMINANT`], so only a face whose
/// front turns toward the ray is met), `0 ≤ u ≤ det`, `v ≥ 0`,
/// `u + v ≤ det`, and the distance `(e2 · q) / det` strictly between 0 and
/// `within`. `dir` is the ray's unit direction, so the distance is in the
/// scene's units.
pub fn ray_face(
    origin: bevy::math::Vec3,
    dir: bevy::math::Vec3,
    [v0, v1, v2]: [bevy::math::Vec3; 3],
    within: f32,
) -> Option<f32> {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    // NaN fails as the original's unordered compare does.
    if det
        .partial_cmp(&FACE_DETERMINANT)
        .is_none_or(|order| order.is_lt())
    {
        return None;
    }
    let s = origin - v0;
    let u = s.dot(p);
    if !(0.0..=det).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q);
    if v < 0.0 || u + v > det {
        return None;
    }
    let t = e2.dot(q) / det;
    (t > 0.0 && t < within).then_some(t)
}

/// The mesh callback's walk: every face in order, each tested within the
/// nearest distance found so far ([`ray_face`]), answering the nearest
/// face's index and distance, `None` when no face is nearer than `within`.
pub fn nearest_face(
    origin: bevy::math::Vec3,
    dir: bevy::math::Vec3,
    faces: impl IntoIterator<Item = [bevy::math::Vec3; 3]>,
    within: f32,
) -> Option<(usize, f32)> {
    let mut best = within;
    let mut found = None;
    for (index, face) in faces.into_iter().enumerate() {
        if let Some(t) = ray_face(origin, dir, face, best) {
            best = t;
            found = Some(index);
        }
    }
    found.map(|index| (index, best))
}

/// The merged body mesh's answer to the pick: the item its per-face table
/// gives the face under the pointer (0 for the body's own skin) and how far
/// along the ray that face is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyHit {
    pub item: u32,
    pub distance: f32,
}

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hover {
    /// The item under the pointer, 0 for none.
    pub item: u32,
    /// Its flags: the record hand entry's when the node is an attached
    /// item, else the first worn entry's with that item.
    pub flags: u32,
    /// The item slot, 1-based.
    pub slot: Option<usize>,
}

/// The item under the pointer. A hit on the item the view already shows
/// answers that item and its flags. Otherwise the ray first meets the
/// merged body mesh, which answers the item its per-face table gives the
/// face (`body`; the search distance starts at the largest float), then
/// every node in order, each only when it is nearer than the nearest hit so
/// far (`reach`, the ray's own length, when the body was missed): the
/// nearest wins. A node hit takes the node's item and sets the slot to the
/// node's (cleared when the node is none of the doll's attached items).
/// With a slot, its record hand entry's flag word (`hand_flags`, the
/// record's hand entries) gives the flags; else the first worn entry
/// naming the item does (`worn`, the record's worn list), else 0.
pub fn hover_pick(
    shown: Option<(u32, u32)>,
    body: Option<BodyHit>,
    reach: f32,
    nodes: &[PickNode],
    hand_flags: &[u32],
    worn: &[(u32, u32)],
) -> Hover {
    if let Some((item, flags)) = shown {
        return Hover {
            item,
            flags,
            slot: None,
        };
    }
    let (mut item, mut best) = body.map_or((0, reach), |hit| (hit.item, hit.distance));
    let mut slot = None;
    for node in nodes.iter().filter(|node| pickable(node)) {
        if let Some(distance) = node.distance
            && distance < best
        {
            best = distance;
            item = node.item;
            slot = node.slot;
        }
    }
    let slot = slot.filter(|index| *index >= 1);
    let flags = match slot {
        Some(index) => hand_flags.get(index - 1).copied().unwrap_or(0),
        None => worn
            .iter()
            .find(|(id, _)| *id == item)
            .map_or(0, |(_, flags)| *flags),
    };
    Hover { item, flags, slot }
}

/// The plan a drop onto the figure makes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropPlan {
    /// The section the drop picked.
    pub kind: FigureDrop,
    /// The mind cost the drop charges, when the view charges costs: the
    /// apparel's wear room, or (only with a second character shown) twice
    /// the weapon's desirability less what it displaces. A charge the mind
    /// cannot pay cancels the drop.
    pub cost: Option<f32>,
    /// The item is attached on the doll and on the shown character.
    pub attach: [bool; 2],
    /// The doll's and the shown character's mesh and textures are rebuilt.
    pub refit: [bool; 2],
    /// The shown character's pack takes the item at its centre, marked
    /// held (a weapon or a shield): the view is not locked and the pack is
    /// not where the drag began.
    pub give_to_panel: bool,
    /// The shown character's pack is charged the item's bulk (worn
    /// apparel, whether or not the view is locked).
    pub charge_panel: bool,
    /// The item is shown under the pointer (the core crystal).
    pub show: bool,
    /// The shown character's held mark is cleared (the core crystal).
    pub clear_mark: bool,
    /// The mind cost the drag-over test stored is charged (a drop back on
    /// the view the drag came from).
    pub charge_cost: bool,
    /// A weapon goes into the off hand: the pointer was on the figure's
    /// right half and the character knows Insight's technique 8.
    pub off_wanted: bool,
}

impl DropPlan {
    const fn nothing() -> Self {
        Self {
            kind: FigureDrop::None,
            cost: None,
            attach: [false; 2],
            refit: [false; 2],
            give_to_panel: false,
            charge_panel: false,
            show: false,
            clear_mark: false,
            charge_cost: false,
            off_wanted: false,
        }
    }
}

/// The facts a drop onto the figure needs beyond the section test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropFacts {
    /// The dragged item and its flags.
    pub id: u32,
    pub flags: u32,
    /// The drag began in this view's own panel.
    pub from_self: bool,
    /// The drag began in the shown character's pack.
    pub from_panel: bool,
    pub locked: bool,
    /// A second character is shown over the doll.
    pub shown: bool,
    /// That character's pack is open.
    pub other_panel: bool,
    pub mind_cost: bool,
    /// The apparel's wear room: its value less that of the worn pieces it
    /// overlaps (`exa_ai::search::wear_room` computes the same figure).
    pub apparel_cost: f32,
    /// The weapon's desirability and what it displaces.
    pub weapon_value: f32,
    pub displaced: f32,
    /// The sections the item has, and whether the body takes its core.
    pub has_apparel: bool,
    pub has_weapon: bool,
    pub has_shield: bool,
    pub takes_core: bool,
    /// The doll's race wears apparel.
    pub race_wears_apparel: bool,
    /// A weapon goes into the off hand ([`insight_blocks`]).
    pub insight: bool,
}

impl Default for DropFacts {
    fn default() -> Self {
        Self {
            id: 0,
            flags: 0,
            from_self: false,
            from_panel: false,
            locked: false,
            shown: false,
            other_panel: false,
            mind_cost: false,
            apparel_cost: 0.0,
            weapon_value: 0.0,
            displaced: 0.0,
            has_apparel: false,
            has_weapon: false,
            has_shield: false,
            takes_core: false,
            race_wears_apparel: false,
            insight: false,
        }
    }
}

/// The drop onto the figure. A drag from the view's own panel does nothing
/// but charge back the mind cost its drag start refunded. Otherwise the
/// item's sections pick the action: apparel is worn (the doll and the shown
/// character rebuilt, the shown pack charged its bulk), a weapon is taken
/// into the doll's hands and mirrored on the shown character, a shield the
/// same, and the body's core crystal is shown and the shown character
/// unmarked. A weapon or shield from another pack is also handed to the
/// shown character's pack, marked held, when the view is not locked.
pub fn figure_drop(facts: DropFacts) -> DropPlan {
    if facts.from_self {
        // Dropped back where it came from: the stored cost is charged and
        // nothing else happens.
        return DropPlan {
            charge_cost: facts.mind_cost,
            ..DropPlan::nothing()
        };
    }
    let kind = drop_kind(FigureDropInput {
        race_flags: if facts.race_wears_apparel { 4 } else { 0 },
        has_apparel: facts.has_apparel,
        has_weapon: facts.has_weapon,
        has_shield: facts.has_shield,
        takes_core: facts.takes_core,
    });
    let from_other = facts.other_panel && !facts.from_panel;
    let hand_over = !facts.locked && from_other;
    match kind {
        FigureDrop::None => DropPlan::nothing(),
        FigureDrop::Apparel => DropPlan {
            kind,
            cost: facts.mind_cost.then_some(facts.apparel_cost),
            attach: [false; 2],
            refit: [true, facts.shown],
            give_to_panel: false,
            charge_panel: from_other,
            show: false,
            clear_mark: false,
            charge_cost: false,
            off_wanted: false,
        },
        FigureDrop::Weapon => DropPlan {
            kind,
            cost: (facts.mind_cost && facts.shown)
                .then_some((facts.weapon_value - facts.displaced) * 2.0),
            attach: [true, facts.shown],
            refit: [false; 2],
            give_to_panel: hand_over,
            charge_panel: false,
            show: false,
            clear_mark: false,
            charge_cost: false,
            off_wanted: facts.insight,
        },
        FigureDrop::Shield => DropPlan {
            kind,
            cost: None,
            attach: [true, facts.shown],
            refit: [false; 2],
            give_to_panel: hand_over,
            charge_panel: false,
            show: false,
            clear_mark: false,
            charge_cost: false,
            off_wanted: false,
        },
        FigureDrop::Hand => DropPlan {
            kind,
            cost: None,
            attach: [false; 2],
            refit: [false; 2],
            give_to_panel: false,
            charge_panel: false,
            show: true,
            clear_mark: facts.shown,
            charge_cost: false,
            off_wanted: false,
        },
    }
}

/// The mind cost a drag off the figure refunds for the item it picks up
/// (and a drop back or a cancelled release charges again): twice a
/// weapon's desirability, else an apparel's protection value; a shield and
/// the core crystal have none. The weapon section is read first.
pub fn drag_over_cost(
    has_apparel: bool,
    has_weapon: bool,
    apparel_cost: f32,
    weapon_value: f32,
) -> Option<f32> {
    if has_weapon {
        Some(weapon_value * 2.0)
    } else if has_apparel {
        Some(apparel_cost)
    } else {
        None
    }
}

/// What a release on the figure asks of the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReleasePlan {
    /// The item slot (0-based) whose take-out is scheduled: on the doll
    /// when [`doll_take_out`], on the shown character when
    /// [`shown_take_out`].
    ///
    /// [`doll_take_out`]: Self::doll_take_out
    /// [`shown_take_out`]: Self::shown_take_out
    pub hand: Option<usize>,
    pub doll_take_out: bool,
    pub shown_take_out: bool,
    /// The answer handed on to the owner's drop callback: the drag's own,
    /// or -1 for a locked view's drag of an item flagged `0x80000000`.
    pub answer: i32,
    /// The dragged icon is the shown item: it is taken off.
    pub take_off_shown: bool,
    /// The dragged item is taken off the worn list.
    pub take_off: bool,
    /// The shown character's pack drops its held icon of the dragged item.
    pub remove_held_icon: bool,
    /// The shown character's pack is charged the dragged item's bulk.
    pub charge_panel: bool,
    /// The doll's and the shown character's mesh and textures are rebuilt.
    pub refit: [bool; 2],
    /// The shown character's mark (the one a shown core crystal clears)
    /// is set again.
    pub set_mark: bool,
    /// The player's controls are refreshed.
    pub refresh_player: bool,
    /// The mind cost the drag start refunded is charged again (a
    /// cancelled drag).
    pub charge_cost: bool,
}

/// The facts a release on the figure needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReleaseFacts {
    /// The slot the drag answered for (`i32::MIN` for a cancelled drag).
    pub slot: i32,
    pub locked: bool,
    /// The dragged item's payload flags.
    pub payload_flags: u32,
    pub mind_cost: bool,
    /// The item slot under the pointer, 0-based (the hand whose take-out is
    /// scheduled).
    pub hand: Option<usize>,
    pub shown: bool,
    /// The dragged icon is the item the view shows.
    pub shown_is_dragged: bool,
    pub other_panel: bool,
    pub from_panel: bool,
    pub player_shown: bool,
    /// [?] The view's control byte that leaves the doll's own hands alone
    /// on a release over a slot (the shown character's take-out still
    /// runs). Its name in the original is not known.
    pub keep_doll: bool,
}

/// The release on the figure. A cancelled drag (`i32::MIN`) charges the
/// refunded mind cost again and does nothing else; a locked view's drag of
/// an item flagged `0x80000000` goes on with the answer -1. A release over
/// an item slot schedules that slot's take-out on the doll (unless the view
/// keeps the doll's hands) and, when a second character is shown, on it
/// too; the other pack drops its held icon and nothing is refit. A release
/// of the shown item takes it off the worn list, sets the shown
/// character's mark again and clears the shown item; anything else takes
/// the dragged item off, charges the other pack its bulk, and rebuilds both
/// characters.
pub fn release_on_figure(facts: ReleaseFacts) -> ReleasePlan {
    let mut plan = ReleasePlan::default();
    let answer = if facts.locked && facts.payload_flags & 0x8000_0000 != 0 {
        -1
    } else {
        facts.slot
    };
    if answer == i32::MIN {
        plan.charge_cost = facts.mind_cost;
        return plan;
    }
    plan.answer = answer;
    if let Some(index) = facts.hand {
        plan.hand = Some(index);
        plan.doll_take_out = !facts.keep_doll;
        plan.shown_take_out = facts.shown;
        plan.remove_held_icon = facts.other_panel && !facts.from_panel;
        plan.refresh_player = facts.player_shown;
        return plan;
    }
    if facts.shown_is_dragged {
        plan.take_off_shown = true;
        plan.set_mark = facts.shown;
        return plan;
    }
    plan.take_off = true;
    plan.charge_panel = facts.other_panel && !facts.from_panel;
    plan.refit = [true, facts.shown];
    plan.refresh_player = facts.player_shown;
    plan
}

/// Which drag-over answer the view gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragAnswer {
    /// The item resolves to an icon: the panel accepts it (`0x105`).
    Accept,
    /// The item's icon does not resolve, or the view is frozen and not
    /// over a worn entry.
    Refuse,
}

/// The drag-over answer. A view that allows changes answers for any item; a
/// frozen one only while it is over a worn entry. An item whose icon does
/// not resolve is refused.
pub fn drag_over(frozen: bool, worn: Option<usize>, icon: usize) -> DragAnswer {
    if frozen && worn.is_none() {
        return DragAnswer::Refuse;
    }
    if icon < 1 {
        DragAnswer::Refuse
    } else {
        DragAnswer::Accept
    }
}

/// The payload a drag started on the figure carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragPayload {
    /// The drag's kind: `0xFB01` (an item payload) or `u32::MAX` (a locked
    /// view's item flagged `0x8000_0000`, which no panel takes); see
    /// [`drag_start`].
    pub kind: u32,
    /// The icon the item resolves to.
    pub icon: usize,
    /// The item's own flags from the hover pick.
    pub flags: u32,
    /// The payload's half extent: half the current icon size (80 at the
    /// default 160).
    pub half: u32,
}

/// What a drag started on the figure does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragStartPlan {
    /// The mind cost the view stores (for a drop back or a cancelled release
    /// to charge again) and charges as `-cost` now: only with the mind-cost
    /// flag on and a character shown. `Some(0.0)` for an item with neither a
    /// weapon nor an apparel section.
    pub cost: Option<f32>,
    /// The character could not take the charge: the drag is cancelled and
    /// nothing else happens.
    pub cancel: bool,
    /// The payload, answered with `0x105`; `None` leaves the answer at 1.
    pub payload: Option<DragPayload>,
}

/// The drag start on the figure, with its mind cost and payload. A frozen
/// view starts drags only from a held item. With the mind-cost flag on and a
/// character shown, the item's cost ([`drag_over_cost`], 0 without a
/// section) is stored and refunded first; a character who cannot take the
/// charge (`refuses`) cancels the drag. The charge stands even when the
/// item's icon then does not resolve and no drag starts. Otherwise the
/// payload carries the [`drag_start`] kind, the icon, the item's flags and
/// half the icon size.
///
/// `cost` is `Some` only when the mind cost is charged (the flag on and a
/// character shown): the item's [`drag_over_cost`], or 0 without a section.
#[allow(clippy::too_many_arguments)]
pub fn drag_start_plan(
    frozen: bool,
    hand: usize,
    icon: usize,
    locked: bool,
    flags: u32,
    cost: Option<f32>,
    refuses: bool,
    icon_size: u32,
) -> DragStartPlan {
    let mut plan = DragStartPlan {
        cost: None,
        cancel: false,
        payload: None,
    };
    if frozen && hand == 0 {
        return plan;
    }
    if let Some(cost) = cost {
        plan.cost = Some(cost);
        if refuses {
            plan.cancel = true;
            return plan;
        }
    }
    plan.payload = drag_start(false, 1, icon, locked, flags).map(|kind| DragPayload {
        kind,
        icon,
        flags,
        half: icon_size >> 1,
    });
    plan
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
        assert_eq!(hand_index(&[None, Some(4), Some(5)], 5, true), Some(3));
        assert_eq!(hand_index(&[None, Some(4)], 4, false), None);
        assert_eq!(hand_index(&[None, Some(4)], 9, true), None);
        assert!(!insight_blocks(0, &[8], 100, 100));
        assert!(insight_blocks(5000, &[8], 100, 100));
        assert!(!insight_blocks(5000, &[8], 50, 100));
        assert!(!insight_blocks(10000, &[7, 7], 100, 100));
        // The technique check alone (the drop's near-hand gate).
        assert!(!insight_known(0, &[8]));
        assert!(insight_known(5000, &[8]));
        assert!(!insight_known(5000, &[7]));
        // Only the levels the points fund are read.
        assert!(!insight_known(5000, &[7, 8]));
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

    #[test]
    fn the_doll_scene_and_its_facing() {
        let scene = doll_scene();
        assert_eq!(scene.distance, 100.0);
        assert_eq!(scene.field_of_view, 30.0);
        assert_eq!(scene.ambient, [0.5; 3]);
        assert_eq!(scene.sky, [0.75; 3]);
        assert_eq!(scene.environment_mode, 2);
        assert_eq!(scene.light_level, 1.0);
        // The camera's identity leaves the direction as it is, normalised:
        // its length is 0.9975, so the length changes.
        let facing = doll_facing(bevy::math::Quat::IDENTITY);
        assert!((facing.length() - 1.0).abs() < 1e-6);
        assert!((facing.x / facing.y - 0.5 / 0.7).abs() < 1e-6);
        // A quarter turn about Y puts it on another axis.
        let turned = doll_facing(bevy::math::Quat::from_rotation_y(
            std::f32::consts::FRAC_PI_2,
        ));
        let share = 0.5 / 0.99f32.sqrt();
        assert!((turned.x - share).abs() < 1e-5, "{turned:?}");
        assert!((turned.z + share).abs() < 1e-5, "{turned:?}");
        assert!((turned.y - 0.7 / 0.99f32.sqrt()).abs() < 1e-5, "{turned:?}");
        assert_eq!(frame_counter(3), 2);
        assert_eq!(frame_counter(0), 0);
        // The original subtracts and clamps: a minimum wraps to the
        // maximum, which is not below zero.
        assert_eq!(frame_counter(i32::MIN), i32::MAX);
    }

    fn node(flags: u32, item: u32, distance: Option<f32>) -> PickNode {
        PickNode {
            node: item + 1,
            flags,
            item,
            item_flags: 0,
            item_settle: -1,
            distance,
            slot: None,
        }
    }

    #[test]
    fn the_hover_pick_takes_the_nearest_hit() {
        let far = 1000.0;
        // Nothing under the pointer.
        assert_eq!(
            hover_pick(None, None, far, &[], &[], &[]),
            Hover {
                item: 0,
                flags: 0,
                slot: None
            }
        );
        // The shown icon wins wherever the pointer is.
        assert_eq!(
            hover_pick(
                Some((7, 0x10)),
                None,
                far,
                &[node(0x89, 8, Some(1.0))],
                &[],
                &[]
            ),
            Hover {
                item: 7,
                flags: 0x10,
                slot: None
            }
        );
        // A node must carry all of 0x89 and must not be hidden.
        assert!(!pickable(&node(0x88, 8, Some(1.0))));
        assert!(!pickable(&node(0x89 | PICK_HIDDEN, 8, Some(1.0))));
        let mut hidden = node(0x89, 8, Some(1.0));
        hidden.item_flags = PICK_ITEM_HIDDEN;
        hidden.item_settle = 0;
        assert!(!pickable(&hidden));
        hidden.item_settle = -1;
        assert!(pickable(&hidden));
        // The body's face gives the worn item and the worn entry's flags.
        let worn = [(12u32, 0x20u32), (30, 0x40)];
        let body = BodyHit {
            item: 30,
            distance: 4.0,
        };
        assert_eq!(
            hover_pick(None, Some(body), far, &[], &[1, 2], &worn),
            Hover {
                item: 30,
                flags: 0x40,
                slot: None
            }
        );
        // A held item in front of the body wins with its hand's flags; one
        // behind it does not, nor does a missed node.
        let mut held_node = node(0x89, 12, Some(3.0));
        held_node.slot = Some(2);
        assert_eq!(
            hover_pick(
                None,
                Some(body),
                far,
                &[node(0x89, 8, Some(5.0)), held_node, node(0x89, 9, None)],
                &[1, 2],
                &worn
            ),
            Hover {
                item: 12,
                flags: 2,
                slot: Some(2)
            }
        );
        // A nearer node that is no attached item (the hair) clears both the
        // item and the slot.
        let hair = node(0x89, 0, Some(2.0));
        assert_eq!(
            hover_pick(None, Some(body), far, &[held_node, hair], &[1, 2], &worn),
            Hover {
                item: 0,
                flags: 0,
                slot: None
            }
        );
        // Without the body, a node within the ray's reach is met; one past
        // it is not. Equal distances keep the first.
        assert_eq!(
            hover_pick(None, None, 2.5, &[node(0x89, 12, Some(3.0))], &[], &worn).item,
            0
        );
        assert_eq!(
            hover_pick(
                None,
                None,
                far,
                &[node(0x89, 12, Some(3.0)), node(0x89, 30, Some(3.0))],
                &[],
                &worn
            ),
            Hover {
                item: 12,
                flags: 0x20,
                slot: None
            }
        );
    }

    #[test]
    fn the_face_test_is_one_sided_and_nearest_first() {
        use bevy::math::Vec3;
        // A face in the plane z = 0, its front (e1 × e2) toward +z.
        let face = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let origin = Vec3::new(0.25, 0.25, 5.0);
        assert_eq!(ray_face(origin, Vec3::NEG_Z, face, 100.0), Some(5.0));
        // From behind it is not met.
        let behind = Vec3::new(0.25, 0.25, -5.0);
        assert_eq!(ray_face(behind, Vec3::Z, face, 100.0), None);
        // Nor past the search distance, nor outside its edges.
        assert_eq!(ray_face(origin, Vec3::NEG_Z, face, 5.0), None);
        let outside = Vec3::new(0.75, 0.75, 5.0);
        assert_eq!(ray_face(outside, Vec3::NEG_Z, face, 100.0), None);
        // A face behind the origin is not met either.
        assert_eq!(ray_face(origin, Vec3::Z, face, 100.0), None);
        // Of two faces the nearer answers, whatever the order.
        let near = [
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(1.0, 0.0, 2.0),
            Vec3::new(0.0, 1.0, 2.0),
        ];
        assert_eq!(
            nearest_face(origin, Vec3::NEG_Z, [face, near], f32::MAX),
            Some((1, 3.0))
        );
        assert_eq!(
            nearest_face(origin, Vec3::NEG_Z, [near, face], f32::MAX),
            Some((0, 3.0))
        );
        assert_eq!(nearest_face(origin, Vec3::NEG_Z, [face], 1.0), None);
        assert_eq!(FACE_DETERMINANT.to_bits(), 0x33ff_ffd7);
    }

    #[test]
    fn a_drop_on_the_figure_follows_the_sections() {
        // A drag from the view's own panel does nothing but charge the
        // cost the drag-over test stored.
        assert_eq!(
            figure_drop(DropFacts {
                from_self: true,
                has_weapon: true,
                ..Default::default()
            }),
            DropPlan::nothing()
        );
        assert!(
            figure_drop(DropFacts {
                from_self: true,
                mind_cost: true,
                ..Default::default()
            })
            .charge_cost
        );
        // Apparel: cost, both refits, the other pack charged the bulk but
        // not handed the item, even when locked.
        let plan = figure_drop(DropFacts {
            has_apparel: true,
            race_wears_apparel: true,
            shown: true,
            other_panel: true,
            mind_cost: true,
            locked: true,
            apparel_cost: 2.5,
            ..Default::default()
        });
        assert_eq!(plan.kind, FigureDrop::Apparel);
        assert_eq!(plan.cost, Some(2.5));
        assert_eq!(plan.refit, [true, true]);
        assert!(!plan.give_to_panel && plan.charge_panel);
        assert_eq!(plan.attach, [false; 2]);
        // The apparel's cost does not wait for a shown character.
        let plan = figure_drop(DropFacts {
            has_apparel: true,
            race_wears_apparel: true,
            mind_cost: true,
            apparel_cost: 2.5,
            ..Default::default()
        });
        assert_eq!(plan.cost, Some(2.5));
        assert_eq!(plan.refit, [true, false]);
        // A weapon: no refit, both hands attach, and the cost is twice the
        // desirability less what it displaces.
        let plan = figure_drop(DropFacts {
            has_weapon: true,
            shown: true,
            mind_cost: true,
            weapon_value: 5.0,
            displaced: 1.25,
            locked: true,
            other_panel: true,
            ..Default::default()
        });
        assert_eq!(plan.kind, FigureDrop::Weapon);
        assert_eq!(plan.cost, Some((5.0 - 1.25) * 2.0));
        assert_eq!(plan.attach, [true, true]);
        assert_eq!(plan.refit, [false; 2]);
        // Locked: the other pack is not handed the item.
        assert!(!plan.give_to_panel);
        // Unlocked, the other pack takes it; without a shown character the
        // weapon costs nothing.
        let plan = figure_drop(DropFacts {
            has_weapon: true,
            mind_cost: true,
            weapon_value: 5.0,
            other_panel: true,
            ..Default::default()
        });
        assert!(plan.give_to_panel && plan.cost.is_none());
        // The pointer's right half with Insight's technique 8 asks for the
        // off hand; a shield ignores it.
        let plan = figure_drop(DropFacts {
            has_weapon: true,
            insight: true,
            ..Default::default()
        });
        assert!(plan.off_wanted);
        assert!(!plan.charge_cost);
        let plan = figure_drop(DropFacts {
            has_weapon: true,
            ..Default::default()
        });
        assert!(!plan.off_wanted);
        // A shield has no cost, and the off-hand request does not apply.
        let plan = figure_drop(DropFacts {
            has_shield: true,
            shown: true,
            insight: true,
            mind_cost: true,
            ..Default::default()
        });
        assert_eq!(plan.kind, FigureDrop::Shield);
        assert_eq!(plan.cost, None);
        assert_eq!(plan.attach, [true, true]);
        assert!(!plan.off_wanted);
        // The core crystal is shown and the shown character's mark
        // cleared; with no one shown there is no mark to clear.
        let plan = figure_drop(DropFacts {
            takes_core: true,
            shown: true,
            ..Default::default()
        });
        assert_eq!(plan.kind, FigureDrop::Hand);
        assert!(plan.show && plan.clear_mark);
        let plan = figure_drop(DropFacts {
            takes_core: true,
            ..Default::default()
        });
        assert!(plan.show && !plan.clear_mark);
        assert_eq!(plan.attach, [false; 2]);
        // An item with no section does nothing.
        assert_eq!(figure_drop(DropFacts::default()).kind, FigureDrop::None);
        // A race that wears nothing ignores the apparel section.
        assert_eq!(
            figure_drop(DropFacts {
                has_apparel: true,
                has_weapon: true,
                ..Default::default()
            })
            .kind,
            FigureDrop::Weapon
        );
    }

    #[test]
    fn a_release_on_the_figure_takes_a_hand_or_an_item() {
        let facts =
            |slot, locked, payload_flags, hand, shown_is_dragged, other_panel, from_panel| {
                ReleaseFacts {
                    slot,
                    locked,
                    payload_flags,
                    mind_cost: true,
                    hand,
                    shown: true,
                    shown_is_dragged,
                    other_panel,
                    from_panel,
                    player_shown: true,
                    keep_doll: false,
                }
            };
        // A cancelled drag only charges the refunded cost again.
        let p = release_on_figure(facts(i32::MIN, false, 0, None, false, false, false));
        assert!(p.charge_cost && p.hand.is_none() && !p.take_off && !p.refit[0]);
        // A locked drag of a flagged item goes on with the answer -1 and
        // charges nothing.
        let p = release_on_figure(facts(2, true, 0x8000_0000, None, false, true, false));
        assert!(!p.charge_cost && p.take_off && p.answer == -1);
        // ... even when the drag itself was cancelled.
        let p = release_on_figure(facts(i32::MIN, true, 0x8000_0000, None, false, true, false));
        assert!(!p.charge_cost && p.answer == -1);
        // Over a slot: that slot's take-out is scheduled on both
        // characters; the other pack drops the held icon; no refit.
        let p = release_on_figure(facts(2, false, 0, Some(1), true, true, false));
        assert_eq!((p.hand, p.answer), (Some(1), 2));
        assert!(p.doll_take_out && p.shown_take_out && p.remove_held_icon && p.refresh_player);
        assert!(!p.refit[0] && !p.take_off && !p.take_off_shown);
        // A view that keeps the doll's hands schedules the shown one only.
        let p = release_on_figure(ReleaseFacts {
            keep_doll: true,
            ..facts(2, false, 0, Some(0), false, false, false)
        });
        assert!(!p.doll_take_out && p.shown_take_out);
        // The dragged icon is the shown item: it is taken off and the mark
        // set again, with no refit and no player refresh.
        let p = release_on_figure(facts(2, false, 0, None, true, false, false));
        assert!(p.take_off_shown && p.set_mark);
        assert_eq!(p.refit, [false, false]);
        assert!(!p.take_off && !p.refresh_player);
        // Anything else: the dragged item is taken off and both rebuilt.
        let p = release_on_figure(facts(2, false, 0, None, false, true, false));
        assert!(p.take_off && p.charge_panel);
        assert_eq!(p.refit, [true, true]);
        assert!(!p.remove_held_icon);
        // A drag from the same pack is not charged.
        let p = release_on_figure(facts(2, false, 0, None, false, true, true));
        assert!(!p.charge_panel);
    }

    #[test]
    fn the_drag_over_test_and_its_costs() {
        assert_eq!(drag_over_cost(true, false, 3.0, 4.0), Some(3.0));
        assert_eq!(drag_over_cost(false, true, 3.0, 4.0), Some(8.0));
        // An item with both sections is costed as a weapon.
        assert_eq!(drag_over_cost(true, true, 3.0, 4.0), Some(8.0));
        assert_eq!(drag_over_cost(false, false, 3.0, 4.0), None);
        assert_eq!(drag_over(false, None, 4), DragAnswer::Accept);
        assert_eq!(drag_over(true, None, 4), DragAnswer::Refuse);
        assert_eq!(drag_over(true, Some(1), 4), DragAnswer::Accept);
        assert_eq!(drag_over(false, Some(1), 0), DragAnswer::Refuse);
        // The drag start with its cost and payload: a frozen view off a
        // held item starts nothing and charges nothing.
        let none = drag_start_plan(true, 0, 4, false, 0, Some(2.5), false, 160);
        assert_eq!((none.cost, none.cancel, none.payload), (None, false, None));
        // A weapon of desirability 1.25 costs 2.5, stored and refunded; the
        // payload carries the kind, the icon, the item's own flags and half
        // the icon size.
        let cost = drag_over_cost(false, true, 0.0, 1.25);
        let plan = drag_start_plan(false, 0, 4, false, 0x10, cost, false, 160);
        assert_eq!((plan.cost, plan.cancel), (Some(2.5), false));
        assert_eq!(
            plan.payload,
            Some(DragPayload {
                kind: 0xFB01,
                icon: 4,
                flags: 0x10,
                half: 80
            })
        );
        // A character who cannot pay cancels; the cost is still stored.
        let plan = drag_start_plan(false, 0, 4, false, 0, Some(2.5), true, 160);
        assert_eq!(
            (plan.cost, plan.cancel, plan.payload),
            (Some(2.5), true, None)
        );
        // The charge stands when the icon then does not resolve.
        let plan = drag_start_plan(false, 0, 0, false, 0, Some(2.5), false, 160);
        assert_eq!((plan.cost, plan.payload), (Some(2.5), None));
        // A locked view's item flagged 0x80000000 travels as kind all ones
        // and keeps its flags; the half follows the icon size.
        let plan = drag_start_plan(true, 2, 4, true, 0x8000_0000, None, false, 120);
        let payload = plan.payload.expect("started");
        assert_eq!(
            (payload.kind, payload.flags, payload.half),
            (u32::MAX, 0x8000_0000, 60)
        );
    }
}
