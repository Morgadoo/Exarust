//! The arena loadout editor: one fighter's kit for one rank, the arsenal's
//! swaps, the equipment-point limit and the save on close.
//!
//! Evidence: the loadout form's routines in the retail image, read from
//! the instructions. The form is 1024 × 768 with the character-creation
//! preview between the arsenal (left) and the equipment (right). Mode 1
//! (named creation here) adds a name box, edits rank 5 and shows the hire
//! block (HIRE, fee, upkeep, profession) in place of the rank and shelf
//! rows; mode 3 (champion) only shows the kit. An item may be equipped when its rank is at most the
//! form's rank (5 when unset) and its cost fits the rank's limit; a weapon
//! costs its points less what it displaces from the hands. The paint fills
//! the form black before the base draws.
//!
//! [`LoadoutEditor`] and [`apply_drop`] are Exarust's own list model of the
//! kit (weapons and worn items), not the original's drop handler, which
//! reconciles the three item containers entry by entry against the arena
//! stock (guess: the lists stand in for the containers).

/// The form's width in pixels.
pub const FORM_WIDTH: i32 = 1024;
/// The form's height in pixels.
pub const FORM_HEIGHT: i32 = 768;
/// The form's flags.
pub const FORM_FLAGS: u32 = 0x80_2022;
/// The form's opacity when the interface is wide (the float 0x3f565fbd).
pub const FORM_OPACITY: f32 = 0.837_398_35;
/// The equipment container's scale when the interface is wide.
pub const EQUIP_SCALE_WIDE: f32 = 0.872_611_46;
/// The arsenal container: x, y, width, and the height the opacity scales.
pub const ARSENAL_X: i32 = 4;
pub const ARSENAL_Y: i32 = 180;
pub const ARSENAL_WIDTH: i32 = 228;
pub const ARSENAL_HEIGHT: f32 = 492.0;
/// The equipment container: its width, its y and its height (narrow and
/// wide); it sits four pixels from the parent's right edge.
pub const EQUIP_WIDTH: i32 = 236;
pub const EQUIP_Y: i32 = 88;
pub const EQUIP_HEIGHT: i32 = 628;
pub const EQUIP_HEIGHT_WIDE: i32 = 548;
pub const RIGHT_MARGIN: i32 = 4;
/// The preview starts here and leaves this much on both sides plus 8.
pub const PREVIEW_X: i32 = 236;
pub const PREVIEW_MARGIN: i32 = 8;
/// The name box in creation mode: its width, y and height.
pub const NAME_BOX_WIDTH: i32 = 236;
pub const NAME_BOX_Y: i32 = 32;
pub const NAME_BOX_HEIGHT: i32 = 256;
/// The rank creation mode edits.
pub const CREATION_RANK: i32 = 5;
/// The mask that decides whether an arsenal item is swapped out.
pub const SWAP_MASK: u32 = 0x400;
/// The flag that marks a two-handed weapon's off hand.
pub const OFFHAND_FLAG: u32 = 0x8000_0000;
/// The flag of an item that may stay in the armour list.
pub const ARMOUR_KEEP: u32 = 0x100;
/// The flag the form puts on an arsenal entry the shown record has
/// equipped (a hand or a worn item).
pub const EQUIPPED_MARK: u32 = 0x200;
/// The rank an unset form rank stands for.
pub const MAX_RANK_UNSET: i32 = 5;

/// The form's mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadoutMode {
    /// The arena's loadout editor.
    Arena = 0,
    /// Character creation's kit.
    Creation = 1,
    /// A match's kit, read only.
    Match = 2,
    /// A champion's kit, shown alone.
    Champion = 3,
}

impl LoadoutMode {
    /// The mode of the form's word.
    pub fn of(word: i32) -> Self {
        match word {
            1 => LoadoutMode::Creation,
            2 => LoadoutMode::Match,
            3 => LoadoutMode::Champion,
            _ => LoadoutMode::Arena,
        }
    }
}

/// Which list an item is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// The armour list (the original's low-numbered per-rank slots).
    Armour = 1,
    /// The weapon list.
    Weapon = 2,
    /// A two-handed weapon.
    TwoHanded = 3,
}

/// An item as the form and its containers see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutItem {
    pub id: u32,
    /// The item family (the weapon, armour or other class).
    pub category: u32,
    /// The equipment points it costs.
    pub points: i32,
    /// The rank it needs.
    pub rank: i32,
    /// The item is worn rather than held (the catalogue's armour kind).
    pub worn: bool,
    pub two_handed: bool,
    /// The container flags; bit 31 marks the item as the off hand.
    pub flags: u32,
    /// The item may be dropped into this slot at all.
    pub allowed: bool,
}

impl LoadoutItem {
    /// A one-handed weapon item.
    pub fn weapon(id: u32, category: u32, points: i32, rank: i32) -> Self {
        Self {
            id,
            category,
            points,
            rank,
            worn: false,
            two_handed: false,
            flags: 0,
            allowed: true,
        }
    }

    /// A two-handed weapon item.
    pub fn two_handed(id: u32, category: u32, points: i32, rank: i32) -> Self {
        Self {
            two_handed: true,
            ..Self::weapon(id, category, points, rank)
        }
    }

    /// An armour item.
    pub fn armour(id: u32, category: u32, points: i32, rank: i32) -> Self {
        Self {
            worn: true,
            ..Self::weapon(id, category, points, rank)
        }
    }
}

/// The editor's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutEditor {
    pub mode: LoadoutMode,
    /// The rank being edited.
    pub rank: usize,
    /// The equipment points used.
    pub points: i32,
    /// The rank's limit.
    pub limit: i32,
    /// The highest rank an item may need.
    pub max_rank: i32,
    /// The weapon list.
    pub weapons: Vec<LoadoutItem>,
    /// The armour list.
    pub armour: Vec<LoadoutItem>,
    /// The arsenal.
    pub arsenal: Vec<LoadoutItem>,
    /// The form's swap mask.
    pub mask: u32,
    /// The form's own item category.
    pub category: u32,
    /// The item the form points at (the drag's item).
    pub pointed: Option<u32>,
    /// The preview shows the character.
    pub preview: bool,
}

impl Default for LoadoutEditor {
    fn default() -> Self {
        Self {
            mode: LoadoutMode::Arena,
            rank: 0,
            points: 0,
            limit: 0,
            max_rank: MAX_RANK_UNSET,
            weapons: Vec::new(),
            armour: Vec::new(),
            arsenal: Vec::new(),
            mask: SWAP_MASK,
            category: 0,
            pointed: None,
            preview: true,
        }
    }
}

/// The form's layout for one interface width: x, y, width and height of
/// each part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadoutLayout {
    pub arsenal: [i32; 4],
    pub equipment: [i32; 4],
    /// The preview's x and width (its own height less four).
    pub preview_x: i32,
    pub preview_width: i32,
    /// The name box, in creation mode only.
    pub name_box: Option<[i32; 4]>,
    pub opacity: f32,
    /// The arsenal's scale (the opacity) and the equipment's.
    pub equipment_scale: f32,
}

/// The layout the constructor computes under a parent `parent_width`
/// wide: the arsenal's height is `492 · opacity` rounded half to even, the
/// equipment and the name box hug the parent's right edge.
pub fn layout(mode: LoadoutMode, parent_width: i32, wide: bool) -> LoadoutLayout {
    let opacity = opacity(wide);
    let arsenal_height = crate::engine_math::round_i64(ARSENAL_HEIGHT * opacity) as i32;
    let right = parent_width - EQUIP_WIDTH - RIGHT_MARGIN;
    LoadoutLayout {
        arsenal: [ARSENAL_X, ARSENAL_Y, ARSENAL_WIDTH, arsenal_height],
        equipment: [
            right,
            EQUIP_Y,
            EQUIP_WIDTH,
            if wide {
                EQUIP_HEIGHT_WIDE
            } else {
                EQUIP_HEIGHT
            },
        ],
        preview_x: PREVIEW_X,
        preview_width: FORM_WIDTH - PREVIEW_X * 2 - PREVIEW_MARGIN,
        name_box: (mode == LoadoutMode::Creation).then_some([
            parent_width - NAME_BOX_WIDTH - RIGHT_MARGIN,
            NAME_BOX_Y,
            NAME_BOX_WIDTH,
            NAME_BOX_HEIGHT,
        ]),
        opacity,
        equipment_scale: if wide { EQUIP_SCALE_WIDE } else { 1.0 },
    }
}

/// The form's opacity: 0.83739835 when the interface is wide, else one.
pub fn opacity(wide: bool) -> f32 {
    if wide { FORM_OPACITY } else { 1.0 }
}

/// The dark ink the form's backdrop quad is filled with (`0xFF000000`).
pub const BACKDROP_INK: u32 = 0xFF00_0000;

/// The four flat fills the form's parts use when they are drawn through
/// the GUI draw path: the backdrop (the paint's own black, alpha 255), the
/// arsenal and equipment containers, and the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutInks {
    pub backdrop: [u8; 4],
    pub arsenal: [u8; 4],
    pub equipment: [u8; 4],
    pub preview: [u8; 4],
}

impl LoadoutInks {
    /// The inks from the skin's form colours. The two containers take the
    /// form's background and the preview a darker form colour; the
    /// containers and the preview draw themselves through their own
    /// controls in the original, so these flat fills stand in for them
    /// (guess: the palette entry each part uses).
    pub fn from_palette(palette: &crate::skin::SkinPalette) -> Self {
        Self {
            backdrop: [0, 0, 0, u8::MAX],
            arsenal: palette.form[3],
            equipment: palette.form[3],
            preview: palette.form[5],
        }
    }
}

/// The layer one flat rectangle fills, in the draw path's solid program.
fn solid_layer(rect: [i32; 4], ink: [u8; 4]) -> crate::gui_draw::GuiLayer {
    let [x, y, w, h] = rect;
    let mut batch = crate::skin::SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(ink);
    batch.begin();
    batch.vertex(x as f32, y as f32);
    batch.vertex(x as f32, (y + h) as f32);
    batch.vertex((x + w) as f32, (y + h) as f32);
    batch.vertex((x + w) as f32, y as f32);
    batch.end_quads();
    crate::gui_draw::GuiLayer::new(batch, crate::gui_draw::LayerTexture::None)
}

/// Turns the paint's steps into the draw path's frame: the full-form
/// backdrop, then the arsenal, the equipment and the preview at the
/// layout's rectangles, in the paint's order. The name box draws with the
/// preview. A step with an empty rectangle draws nothing.
pub fn frame(
    draws: &[LoadoutDraw],
    layout: &LoadoutLayout,
    inks: &LoadoutInks,
    width: i32,
    height: i32,
) -> crate::gui_draw::GuiFrame {
    let mut frame = crate::gui_draw::GuiFrame::new();
    for draw in draws {
        let (rect, ink) = match *draw {
            LoadoutDraw::Backdrop => ([0, 0, width, height], inks.backdrop),
            LoadoutDraw::Arsenal => (layout.arsenal, inks.arsenal),
            LoadoutDraw::Equipment => (layout.equipment, inks.equipment),
            LoadoutDraw::Preview => (
                // The constructor leaves the preview's y to the character
                // creation helper; the view spans the form's height less
                // four (guess: it starts at the top).
                [layout.preview_x, 0, layout.preview_width, FORM_HEIGHT - 4],
                inks.preview,
            ),
        };
        if rect[2] > 0 && rect[3] > 0 {
            frame.push(solid_layer(rect, ink));
        }
    }
    frame
}

/// The paint's steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadoutDraw {
    /// The form's full-area backdrop quad in [`BACKDROP_INK`].
    Backdrop,
    /// The arsenal container.
    Arsenal,
    /// The equipment container.
    Equipment,
    /// The character preview.
    Preview,
}

/// The paint's order: the backdrop, the arsenal, the equipment and the
/// preview, then the base form.
pub fn draw_order() -> [LoadoutDraw; 4] {
    [
        LoadoutDraw::Backdrop,
        LoadoutDraw::Arsenal,
        LoadoutDraw::Equipment,
        LoadoutDraw::Preview,
    ]
}

/// The flags a drag out of the equipment sets: 0x200 in match mode, none
/// for a champion, 0x400 otherwise.
pub fn drag_flags(mode: LoadoutMode) -> u32 {
    match mode {
        LoadoutMode::Match => 0x200,
        LoadoutMode::Champion => 0,
        _ => 0x400,
    }
}

/// Clears the form's pointed item when the arena drops the one it points
/// at.
pub fn clear_pointed(editor: &mut LoadoutEditor, item: u32) -> bool {
    if editor.mode == LoadoutMode::Arena && editor.pointed == Some(item) {
        editor.pointed = None;
        true
    } else {
        false
    }
}

/// The constructor's record changes for a mode word, made to the 28
/// control records ([`crate::arena_forms::LOADOUT_RECORDS`]) before they are
/// laid out. Mode 1 widens Previous and Next to 112 (Next moving to x 120),
/// hides Dismiss, the rank, points and encumbrance rows and the ten shelf
/// and box buttons ([`crate::settings::HIDDEN`]), and shows the hire block
/// (HIRE, the fee and upkeep rows and the profession, state 0). Mode 3
/// disables the list, the rank and the first four shelf buttons and hides
/// Previous, Next, Dismiss and the points row. The other modes keep the
/// table as shipped.
#[must_use]
pub fn mode_records(mode: LoadoutMode) -> [crate::creation_forms::FormRecord; 28] {
    use crate::arena_forms::loadout as role;
    use crate::gui_controls::state::DISABLED;
    use crate::settings::HIDDEN;
    let mut records = crate::arena_forms::LOADOUT_RECORDS;
    match mode {
        LoadoutMode::Creation => {
            records[role::PREVIOUS].w = 0x70;
            records[role::NEXT].x = 0x78;
            records[role::NEXT].w = 0x70;
            let hidden = [
                role::DISMISS,
                role::RANK_LABEL,
                role::RANK,
                role::POINTS,
                role::ENCUMBRANCE_LABEL,
                role::ENCUMBRANCE,
            ];
            for index in hidden.into_iter().chain(role::SHELVES).chain(role::BOXES) {
                records[index].state = HIDDEN;
            }
            for index in [
                role::HIRE,
                role::FEE_LABEL,
                role::FEE,
                role::UPKEEP_LABEL,
                role::UPKEEP,
                role::PROFESSION,
            ] {
                records[index].state = 0;
            }
        }
        LoadoutMode::Champion => {
            let disabled = [role::LIST, role::RANK];
            for index in disabled
                .into_iter()
                .chain(role::SHELVES[..4].iter().copied())
            {
                records[index].state = DISABLED;
            }
            for index in [role::PREVIOUS, role::NEXT, role::DISMISS, role::POINTS] {
                records[index].state = HIDDEN;
            }
        }
        LoadoutMode::Arena | LoadoutMode::Match => {}
    }
    records
}

/// Marks the arsenal entries the shown record has equipped: first the mark
/// comes off every entry, then it goes on each entry whose item is in one
/// of the two hands or the worn list. The entries are the selected box's
/// (the original walks the record, not the rank loadouts).
pub fn mark_equipped(entries: &mut [LoadoutItem], hands: [u32; 2], worn: &[u32]) {
    for entry in entries.iter_mut() {
        entry.flags &= !EQUIPPED_MARK;
        let held = hands.contains(&entry.id);
        if held || worn.contains(&entry.id) {
            entry.flags |= EQUIPPED_MARK;
        }
    }
}

/// Whether a drop is allowed: onto the equipment container a champion
/// takes nothing and an item whose drag flags share the form's mask is
/// refused; any other target allows it.
pub fn drop_allowed(editor: &LoadoutEditor, item: &LoadoutItem, onto_equipment: bool) -> bool {
    if !onto_equipment {
        return true;
    }
    if editor.mode == LoadoutMode::Champion {
        return false;
    }
    item.flags & editor.mask == 0
}

/// The weapon in the first hand, as the cost rule reads it: its family
/// (the weapon record's flags & 0xf0) and its points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldWeapon {
    pub family: u32,
    pub points: i32,
}

/// The cost of equipping a weapon of `family` worth `points`, given what
/// the hands hold: a one-handed weapon (family 0x10) that cannot go in the
/// off hand displaces the first hand's weapon; one that can displaces the
/// second hand's shield when the first holds a one-handed weapon, else the
/// first hand's weapon. A two-handed or other weapon (0x20, 0x30)
/// displaces both. Any other family costs nothing.
pub fn item_cost(
    family: u32,
    points: i32,
    off_hand: bool,
    first: Option<HeldWeapon>,
    second_shield: Option<i32>,
) -> i32 {
    match family & 0xf0 {
        0x10 => {
            let freed = match first {
                Some(held) if !off_hand || held.family & 0xf0 != 0x10 => -held.points,
                Some(_) => second_shield.map_or(0, |shield| -shield),
                None => 0,
            };
            freed + points
        }
        0x20 | 0x30 => {
            let freed = first.map_or(0, |held| -held.points) - second_shield.unwrap_or(0);
            freed + points
        }
        _ => 0,
    }
}

/// An item's kind as the equip rule reads its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipKind {
    /// A weapon: its rank and its cost ([`item_cost`]).
    Weapon { rank: u8, cost: i32 },
    /// A shield or armour piece: its rank (bits 5…7) and points (bits
    /// 0…4) of its packed word.
    Armour { rank: u8, points: i32 },
    /// A worn piece: the same packing in its own record.
    Worn { rank: u8, points: i32 },
    /// No record the rule knows.
    None,
}

/// What the equip rule knows about a dragged item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipFacts {
    /// The drag comes from the arsenal or the equipment.
    pub from_kit: bool,
    /// How many of the character's two hands hold this item.
    pub held: u32,
    /// How many copies the arsenal and the equipment hold.
    pub copies: u32,
    /// [`drop_allowed`] for a drag from elsewhere.
    pub allowed: bool,
    pub kind: EquipKind,
}

/// The original's equip rule: a drag out of the kit needs a copy the hands
/// do not hold; any other drag must be allowed (a champion's always is,
/// and a champion takes any item). Then the item's rank must be at most
/// the form's (5 when it is negative) and its cost plus the points used
/// within the limit.
pub fn may_equip(mode: LoadoutMode, rank: i32, used: i32, limit: i32, facts: EquipFacts) -> bool {
    let rank = if rank < 0 { MAX_RANK_UNSET } else { rank };
    if facts.from_kit {
        if facts.copies <= facts.held {
            return false;
        }
    } else if mode != LoadoutMode::Champion && !facts.allowed {
        return false;
    }
    if mode == LoadoutMode::Champion {
        return true;
    }
    let fits = |item_rank: u8, cost: i32| {
        i64::from(item_rank) <= i64::from(rank)
            && i64::from(used) + i64::from(cost) <= i64::from(limit)
    };
    match facts.kind {
        EquipKind::Weapon { rank, cost } => fits(rank, cost),
        EquipKind::Armour { rank, points } | EquipKind::Worn { rank, points } => fits(rank, points),
        EquipKind::None => false,
    }
}

/// The match-mode close: a mode-2 release with a key other than 8 takes the
/// last item off the weapon list.
pub fn release_for_key(editor: &mut LoadoutEditor, key: i32) -> bool {
    if editor.mode != LoadoutMode::Match || key == 8 {
        return false;
    }
    if editor.weapons.pop().is_some() {
        editor.points = points_used(&editor.weapons, &editor.armour);
        true
    } else {
        false
    }
}

/// The equipment points the lists use. A two-handed weapon counts once; the
/// off hand's copy is not added.
pub fn points_used(weapons: &[LoadoutItem], armour: &[LoadoutItem]) -> i32 {
    let mut points = 0;
    let mut two_handed = false;
    for item in weapons {
        if item.two_handed {
            if !two_handed {
                points += item.points;
                two_handed = true;
            }
        } else if item.flags & OFFHAND_FLAG == 0 || !two_handed {
            points += item.points;
        }
    }
    points + armour.iter().map(|item| item.points).sum::<i32>()
}

/// How many entries of one container hold an item, by its catalogue id.
pub fn container_copies(entries: &[LoadoutItem], id: u32) -> usize {
    entries.iter().filter(|item| item.id == id).count()
}

/// How many copies of an item the kit holds: the weapon container's
/// entries and the worn container's, counted one container at a time
/// ([`container_copies`]). The company arsenal is not counted.
pub fn copies(editor: &LoadoutEditor, id: u32) -> usize {
    container_copies(&editor.weapons, id) + container_copies(&editor.armour, id)
}

/// The list model's equip check, through [`may_equip`]: an item already in
/// the lists is a drag out of the kit, held by `hands_holding` hands; its
/// rank and points come from the list item. The gate counts the copies the
/// weapon and worn containers hold ([`copies`]), not the arsenal's.
pub fn can_equip(editor: &LoadoutEditor, item: &LoadoutItem, hands_holding: usize) -> bool {
    let facts = EquipFacts {
        from_kit: editor
            .weapons
            .iter()
            .chain(editor.armour.iter())
            .any(|held| held.id == item.id),
        held: hands_holding as u32,
        copies: copies(editor, item.id) as u32,
        allowed: item.allowed,
        kind: EquipKind::Weapon {
            rank: item.rank.clamp(0, 255) as u8,
            cost: item.points,
        },
    };
    may_equip(
        editor.mode,
        editor.max_rank,
        editor.points,
        editor.limit,
        facts,
    )
}

/// Where a drop came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropSource {
    /// The weapon list.
    Weapons,
    /// The armour list.
    Armour,
    /// The arsenal.
    Arsenal,
    /// The character preview.
    Preview,
    /// Another container, matched by the item's id.
    Unknown,
}

/// The container the arsenal's drop handler finds a returned item in. The
/// weapon container marks an off-hand copy with the top bit; the worn
/// container marks the entries held in a hand with [`ARMOUR_KEEP`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropOrigin {
    /// Neither kit container holds it.
    Unknown = 0,
    /// The worn container.
    Armour = 1,
    /// The weapon container.
    Weapons = 2,
    /// The weapon container, as an off-hand copy.
    WeaponsOffHand = 3,
}

impl DropOrigin {
    /// The word the handler's branch selector takes.
    pub fn word(self) -> u32 {
        self as u32
    }

    fn of_flags(flags: u32) -> Self {
        if flags & OFFHAND_FLAG == 0 {
            DropOrigin::Weapons
        } else {
            DropOrigin::WeaponsOffHand
        }
    }
}

/// Sorts an item returned to the arsenal into the kit container it left. A
/// drag from the weapon list takes the off-hand mark from the drag's own
/// flags; a drag from the worn list is [`DropOrigin::Armour`]. Any other
/// drag looks for the item in the weapon list, first entry first, and
/// takes that entry out (its mark decides the origin); failing that it
/// looks for a worn entry with the item that no hand holds, last entry
/// first.
pub fn sort_drop(editor: &mut LoadoutEditor, item: &LoadoutItem, source: DropSource) -> DropOrigin {
    match source {
        DropSource::Weapons => DropOrigin::of_flags(item.flags),
        DropSource::Armour => DropOrigin::Armour,
        _ => {
            if let Some(index) = editor.weapons.iter().position(|held| held.id == item.id) {
                let origin = DropOrigin::of_flags(editor.weapons[index].flags);
                editor.weapons.remove(index);
                origin
            } else if editor
                .armour
                .iter()
                .rev()
                .any(|held| held.id == item.id && held.flags & ARMOUR_KEEP == 0)
            {
                DropOrigin::Armour
            } else {
                DropOrigin::Unknown
            }
        }
    }
}

/// What returning an item to the arsenal did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StockReturn {
    /// The kit container the item left.
    pub origin: DropOrigin,
    /// An arsenal entry lent out under the form's mask took the item back
    /// (its mask bit was cleared).
    pub released: bool,
    /// The arsenal's count of the item is set to zero (an item of another
    /// category the arsenal had not lent).
    pub zero_stock: bool,
    /// The drag keeps its item, so the arsenal adds it as a new entry.
    pub keep_item: bool,
    /// The worn container gave up the hand-held copy, so the hand slots
    /// holding the item are emptied.
    pub freed_hand: bool,
    /// How many copies of the item the kit still holds: the count written
    /// to the rank's loadout record.
    pub count: u32,
}

/// The arsenal's drop handler: an item dragged back onto the arsenal.
///
/// A champion's form takes nothing and the drag drops its item. Otherwise,
/// for an item of the form's own category, the first arsenal entry with
/// the item under the form's mask takes it back (the bit is cleared).
/// When none does, `lent` is the arsenal's own answer to whether it has
/// lent the item under the mask; an unlent item of another category has
/// its arsenal count zeroed. Either way the drag keeps its item only for
/// an unlent item of the form's category.
///
/// Then the item is sorted into the kit container it left
/// ([`sort_drop`]) and the copies the kit still holds are counted, per
/// container:
/// - an off-hand copy removes the first worn entry with the item that a
///   hand holds and counts the other worn entries;
/// - in match mode the weapon list gives up its first off-hand entry when
///   the drag comes from the preview (the count stays as it is);
/// - otherwise a worn copy counts the worn entries, less one (the one a
///   drag from the preview removes: the first that no hand holds), plus the
///   weapon entries that are not off-hand copies;
/// - and a weapon counts the weapon entries that are not off-hand copies,
///   less one (the first off-hand entry a drag from the preview removes
///   instead), plus every worn entry.
pub fn return_to_stock(
    editor: &mut LoadoutEditor,
    item: &LoadoutItem,
    source: DropSource,
    lent: bool,
) -> StockReturn {
    if editor.mode == LoadoutMode::Champion {
        return StockReturn {
            origin: DropOrigin::Unknown,
            released: false,
            zero_stock: false,
            keep_item: false,
            freed_hand: false,
            count: 0,
        };
    }
    let own_category = item.category == editor.category;
    let mut released = false;
    if own_category {
        let mask = editor.mask;
        if let Some(entry) = editor
            .arsenal
            .iter_mut()
            .find(|entry| entry.id == item.id && entry.flags & mask != 0)
        {
            entry.flags &= !mask;
            released = true;
        }
    }
    let zero_stock = !released && !lent && !own_category;
    let keep_item = !released && !lent && own_category;
    let origin = sort_drop(editor, item, source);
    let from_preview = source == DropSource::Preview;
    let id = item.id;
    let mut count = 0u32;
    let mut freed_hand = false;
    if origin == DropOrigin::WeaponsOffHand {
        editor.armour.retain(|held| {
            if held.id != id {
                return true;
            }
            if held.flags & ARMOUR_KEEP != 0 && !freed_hand {
                freed_hand = true;
                false
            } else {
                count += 1;
                true
            }
        });
    }
    if editor.mode == LoadoutMode::Match {
        if from_preview
            && let Some(index) = editor
                .weapons
                .iter()
                .position(|held| held.id == id && held.flags & OFFHAND_FLAG != 0)
        {
            editor.weapons.remove(index);
        }
    } else if origin == DropOrigin::Armour {
        let mut removed = false;
        editor.armour.retain(|held| {
            if held.id != id {
                return true;
            }
            if held.flags & ARMOUR_KEEP == 0 && !removed && from_preview {
                removed = true;
                false
            } else {
                count += 1;
                true
            }
        });
        if !removed && count > 0 {
            count -= 1;
        }
        count += editor
            .weapons
            .iter()
            .filter(|held| held.id == id && held.flags & OFFHAND_FLAG == 0)
            .count() as u32;
    } else if origin == DropOrigin::Weapons {
        let mut removed = false;
        editor.weapons.retain(|held| {
            if held.id != id {
                return true;
            }
            if held.flags & OFFHAND_FLAG == 0 {
                count += 1;
                true
            } else if from_preview && !removed {
                removed = true;
                false
            } else {
                true
            }
        });
        if !removed && count > 0 {
            count -= 1;
        }
        count += container_copies(&editor.armour, id) as u32;
    }
    editor.points = points_used(&editor.weapons, &editor.armour);
    StockReturn {
        origin,
        released,
        zero_stock,
        keep_item,
        freed_hand,
        count,
    }
}

/// What a drop did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropResult {
    /// The item stays equipped.
    pub accepted: bool,
    /// A matching item was taken out of the arsenal.
    pub swapped: bool,
    /// The list the item came from.
    pub source: DropSource,
    /// The item's slot is the off hand.
    pub off_hand: bool,
    /// The preview must be refreshed.
    pub refresh: bool,
    /// The form clears its pointer to the item.
    pub clear_pointer: bool,
}

/// The list model's equip: finds the source, removes the item from the
/// list it came from, clears the off hand for a two-handed weapon and
/// takes the arsenal's matching item when the drop swaps. This stands in
/// for the containers' own drag-and-drop (outside the form); the form's
/// rule it applies is the equip gate ([`can_equip`]). An item found in
/// neither list goes to the armour list when it is worn.
pub fn apply_drop(
    editor: &mut LoadoutEditor,
    item: &LoadoutItem,
    source: DropSource,
) -> DropResult {
    if editor.mode == LoadoutMode::Champion {
        return DropResult {
            accepted: false,
            swapped: false,
            source,
            off_hand: false,
            refresh: false,
            clear_pointer: true,
        };
    }
    let mut accepted = true;
    let mut swapped = false;
    // The arsenal of the form's own category swaps the item already there.
    if source == DropSource::Arsenal
        && item.category == editor.category
        && let Some(index) = editor
            .arsenal
            .iter()
            .position(|held| held.id == item.id && held.flags & editor.mask != 0)
    {
        editor.arsenal[index].flags &= !editor.mask;
        accepted = false;
        swapped = true;
    }
    if !accepted || !can_equip(editor, item, 0) {
        return DropResult {
            accepted: false,
            swapped,
            source,
            off_hand: false,
            refresh: false,
            clear_pointer: true,
        };
    }
    // Find the item in a list if the caller did not say which one it was.
    let slot: Slot;
    let mut resolved = source;
    match source {
        DropSource::Weapons => slot = Slot::Weapon,
        DropSource::Armour => slot = Slot::Armour,
        DropSource::Arsenal | DropSource::Preview | DropSource::Unknown => {
            if let Some(index) = editor.weapons.iter().position(|held| held.id == item.id) {
                slot = if editor.weapons[index].flags & OFFHAND_FLAG != 0 {
                    Slot::TwoHanded
                } else {
                    Slot::Weapon
                };
                editor.weapons.remove(index);
                resolved = DropSource::Weapons;
            } else if let Some(index) = editor.armour.iter().position(|held| held.id == item.id) {
                slot = Slot::Armour;
                editor.armour.remove(index);
                resolved = DropSource::Armour;
            } else {
                slot = if item.two_handed {
                    Slot::TwoHanded
                } else if item.worn {
                    Slot::Armour
                } else {
                    Slot::Weapon
                };
            }
        }
    }
    let off_hand = item.flags & OFFHAND_FLAG != 0;
    match slot {
        Slot::Armour => {
            // A second copy of an armour item is replaced, not stacked.
            editor.armour.retain(|held| held.id != item.id);
            editor.armour.push(*item);
        }
        Slot::Weapon => {
            editor.weapons.retain(|held| held.id != item.id);
            editor.weapons.push(*item);
        }
        Slot::TwoHanded => {
            // The two-handed weapon takes both hands: the off hand goes.
            editor.weapons.clear();
            editor.armour.retain(|held| held.flags & ARMOUR_KEEP != 0);
            editor.weapons.push(*item);
        }
    }
    editor.points = points_used(&editor.weapons, &editor.armour);
    DropResult {
        accepted: true,
        swapped,
        source: resolved,
        off_hand,
        refresh: true,
        clear_pointer: false,
    }
}

/// Removes an item from a list: returns whether it was there.
pub fn unequip(editor: &mut LoadoutEditor, id: u32) -> bool {
    let before = editor.weapons.len() + editor.armour.len();
    editor.weapons.retain(|item| item.id != id);
    editor.armour.retain(|item| item.id != id);
    editor.points = points_used(&editor.weapons, &editor.armour);
    editor.weapons.len() + editor.armour.len() != before
}

/// The preview's encumbrance: `strength² × 10`.
pub fn encumbrance(strength: f32) -> f32 {
    strength * strength * 10.0
}

/// What closing the form does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutSave {
    /// The equipment is written to the rank's kit (the arena and match
    /// modes).
    pub saves_kit: bool,
    /// The form's own loadout is stored on the character (a champion).
    pub stores_record: bool,
    /// The rank whose kit is written.
    pub rank: usize,
    /// The player's and the companions' character views are refreshed (the
    /// arena and match modes).
    pub refresh_views: bool,
    /// The arena's cards are refreshed (every mode but the champion's).
    pub refresh_arena: bool,
}

/// The close: the arena and match modes write the equipment to the rank's
/// kit and refresh the character views; a champion stores the form's
/// loadout on the character; creation writes nothing; every mode but the
/// champion's refreshes the arena.
pub fn save_on_close(editor: &LoadoutEditor) -> LoadoutSave {
    let kit = matches!(editor.mode, LoadoutMode::Arena | LoadoutMode::Match);
    LoadoutSave {
        saves_kit: kit,
        stores_record: editor.mode == LoadoutMode::Champion,
        rank: editor.rank,
        refresh_views: kit,
        refresh_arena: editor.mode != LoadoutMode::Champion,
    }
}

/// The form's controls as the control layout pass reads them, placed in a
/// client (see [`crate::gui_controls::layout_records`]): the constructor's
/// 28 records ([`crate::arena_forms::LOADOUT_RECORDS`]).
pub fn controls(
    unit: i32,
    client: crate::gui_controls::LayoutClient,
) -> Vec<crate::gui_controls::Control> {
    controls_for(LoadoutMode::Arena, unit, client)
}

/// [`controls`] with a mode's record changes ([`mode_records`]) made first.
pub fn controls_for(
    mode: LoadoutMode,
    unit: i32,
    client: crate::gui_controls::LayoutClient,
) -> Vec<crate::gui_controls::Control> {
    let mut records: Vec<crate::gui_controls::Control> = mode_records(mode)
        .iter()
        .map(crate::record_form::record_control)
        .collect();
    crate::gui_controls::layout_records(&mut records, client, unit);
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u32, points: i32, rank: i32) -> LoadoutItem {
        LoadoutItem::weapon(id, 0x20, points, rank)
    }

    const LOADOUT_RECORDS_STATE: [u32; 28] = {
        let mut states = [0; 28];
        let mut i = 0;
        while i < 28 {
            states[i] = crate::arena_forms::LOADOUT_RECORDS[i].state;
            i += 1;
        }
        states
    };

    fn editor() -> LoadoutEditor {
        LoadoutEditor {
            mode: LoadoutMode::Arena,
            rank: 0,
            points: 0,
            limit: 10,
            max_rank: 2,
            ..LoadoutEditor::default()
        }
    }

    #[test]
    fn the_form_controls_are_the_shipped_table() {
        let records = &crate::arena_forms::LOADOUT_RECORDS;
        assert_eq!(records[0].text, Some("ARSENAL"));
        assert_eq!(records[2].text, Some("Previous"));
        assert_eq!(records[4].text, Some("Dismiss"));
        assert_eq!(records[7].text, Some("Points: 0 / 5"));
        assert_eq!(records[8].text, Some("Encumbrance:"));
        assert_eq!(records[21].text, Some("CLOSE"));
        assert_eq!(records[21].style, 0x2048_3300);
        assert_eq!(records[22].state, 0x110);
        let client = crate::gui_controls::LayoutClient {
            inner_right: 1020,
            border: 2,
            bottom: 766,
        };
        let controls = controls(4, client);
        assert_eq!(controls.len(), 28);
        let previous = &controls[2];
        assert_eq!(
            (previous.x, previous.y, previous.w, previous.h),
            (4, 52, 76, 24)
        );
        assert_eq!((controls[3].x, controls[3].y), (84, 52));
        assert_eq!((controls[4].x, controls[4].y), (168, 52));
        // The title label is centred in the client's width.
        assert_eq!((controls[0].x, controls[0].y), (429, 2));
        // Points reads 228 wide below the rank row.
        assert_eq!(
            (controls[7].x, controls[7].y, controls[7].w, controls[7].h),
            (52, 118, 228, 28)
        );
    }

    #[test]
    fn the_layout_reads_the_constructors_fields() {
        assert_eq!(FORM_OPACITY.to_bits(), 0x3f56_5fbd);
        assert_eq!(EQUIP_SCALE_WIDE.to_bits(), 0x3f5f_6377);
        let wide = layout(LoadoutMode::Arena, 1024, true);
        // 492 · 0.83739835 = 411.99999 as a float, rounded to 412.
        assert_eq!(wide.arsenal, [4, 180, 228, 412]);
        assert_eq!(wide.equipment, [784, 88, 236, 548]);
        assert_eq!((wide.preview_x, wide.preview_width), (236, 544));
        assert_eq!(wide.name_box, None);
        assert_eq!(wide.equipment_scale, EQUIP_SCALE_WIDE);
        let narrow = layout(LoadoutMode::Creation, 800, false);
        assert_eq!(narrow.arsenal[3], 492);
        assert_eq!(narrow.equipment, [560, 88, 236, 628]);
        assert_eq!(narrow.name_box, Some([560, 32, 236, 256]));
        assert_eq!((narrow.opacity, narrow.equipment_scale), (1.0, 1.0));
    }

    #[test]
    fn a_weapon_costs_what_it_displaces_from_the_hands() {
        let sword = Some(HeldWeapon {
            family: 0x10,
            points: 4,
        });
        let axe = Some(HeldWeapon {
            family: 0x20,
            points: 6,
        });
        // A one-handed weapon that cannot go in the off hand replaces the
        // first hand.
        assert_eq!(item_cost(0x10, 5, false, sword, Some(3)), 1);
        // One that can: beside a one-handed weapon it replaces the shield.
        assert_eq!(item_cost(0x10, 5, true, sword, Some(3)), 2);
        assert_eq!(item_cost(0x10, 5, true, sword, None), 5);
        // Beside a two-handed weapon it replaces that.
        assert_eq!(item_cost(0x10, 5, true, axe, Some(3)), -1);
        assert_eq!(item_cost(0x11, 5, false, None, None), 5);
        // A two-handed weapon frees both hands.
        assert_eq!(item_cost(0x20, 9, false, sword, Some(3)), 2);
        assert_eq!(item_cost(0x30, 9, false, None, None), 9);
        assert_eq!(item_cost(0x40, 9, false, sword, Some(3)), 0);
    }

    #[test]
    fn the_equip_rule_checks_copies_rank_and_limit() {
        let facts = EquipFacts {
            from_kit: true,
            held: 1,
            copies: 2,
            allowed: false,
            kind: EquipKind::Weapon { rank: 2, cost: 3 },
        };
        assert!(may_equip(LoadoutMode::Arena, 2, 7, 10, facts));
        assert!(!may_equip(LoadoutMode::Arena, 2, 8, 10, facts));
        assert!(!may_equip(LoadoutMode::Arena, 1, 0, 10, facts));
        // A negative form rank stands for 5.
        let high = EquipFacts {
            kind: EquipKind::Armour { rank: 5, points: 1 },
            ..facts
        };
        assert!(may_equip(LoadoutMode::Arena, -1, 0, 10, high));
        // Every copy held: refused, even for a champion.
        let held = EquipFacts { held: 2, ..facts };
        assert!(!may_equip(LoadoutMode::Champion, 5, 0, 10, held));
        // From elsewhere the drop must be allowed, except for a champion,
        // who takes anything.
        let other = EquipFacts {
            from_kit: false,
            kind: EquipKind::None,
            ..facts
        };
        assert!(!may_equip(LoadoutMode::Arena, 5, 0, 10, other));
        assert!(may_equip(LoadoutMode::Champion, 0, 99, 0, other));
        let allowed = EquipFacts {
            allowed: true,
            kind: EquipKind::Worn { rank: 0, points: 2 },
            ..other
        };
        assert!(may_equip(LoadoutMode::Match, 0, 8, 10, allowed));
        assert!(!may_equip(LoadoutMode::Match, 0, 9, 10, allowed));
    }

    #[test]
    fn points_count_a_two_handed_weapon_once() {
        let sword = LoadoutItem::two_handed(1, 0x20, 6, 0);
        let dagger = item(2, 2, 0);
        assert_eq!(points_used(&[sword, dagger], &[]), 8);
        // A copy in the off hand does not double the two-handed weapon.
        let mut off = sword;
        off.flags |= OFFHAND_FLAG;
        assert_eq!(points_used(&[sword, off], &[]), 6);
        let mail = LoadoutItem::armour(3, 0x40, 4, 0);
        assert_eq!(points_used(&[sword], &[mail]), 10);
    }

    #[test]
    fn an_item_needs_its_rank_and_points_and_a_free_hand() {
        let mut editor = editor();
        assert!(can_equip(&editor, &item(1, 5, 2), 2));
        assert!(!can_equip(&editor, &item(1, 5, 3), 2));
        editor.points = 7;
        assert!(!can_equip(&editor, &item(1, 5, 2), 2));
        editor.points = 5;
        assert!(can_equip(&editor, &item(1, 5, 2), 2));
        let mut disallowed = item(1, 0, 2);
        disallowed.allowed = false;
        assert!(!can_equip(&editor, &disallowed, 2));
        // Champion mode ignores every check.
        editor.mode = LoadoutMode::Champion;
        assert!(can_equip(&editor, &item(1, 99, 9), 0));
    }

    #[test]
    fn a_copy_is_refused_when_every_hand_holds_it() {
        let mut editor = editor();
        editor.weapons = vec![item(1, 2, 0)];
        // One copy in the weapons list and no hand holding it: the drop is
        // allowed (it is the same item being moved).
        assert!(can_equip(&editor, &item(1, 2, 0), 0));
        editor.weapons.push(item(1, 2, 0));
        editor.armour.push(item(1, 2, 0));
        assert!(!can_equip(&editor, &item(1, 2, 0), 3));
    }

    #[test]
    fn a_drop_swaps_the_arsenal_item_and_sets_the_points() {
        let mut editor = editor();
        editor.category = 0x20;
        editor.arsenal = vec![LoadoutItem {
            flags: SWAP_MASK,
            ..item(1, 3, 0)
        }];
        let result = apply_drop(&mut editor, &item(1, 3, 0), DropSource::Arsenal);
        assert!(!result.accepted);
        assert!(result.swapped);
        assert_eq!(editor.arsenal[0].flags, 0);
        assert!(editor.weapons.is_empty());
        // A different category equips.
        let result = apply_drop(&mut editor, &item(2, 3, 0), DropSource::Arsenal);
        assert!(result.accepted);
        assert_eq!(editor.weapons.len(), 1);
        assert_eq!(editor.points, 3);
    }

    #[test]
    fn a_two_handed_weapon_clears_the_off_hand() {
        let mut editor = editor();
        editor.weapons = vec![item(1, 2, 0), {
            let mut off = item(2, 2, 0);
            off.flags |= OFFHAND_FLAG;
            off
        }];
        let result = apply_drop(
            &mut editor,
            &LoadoutItem::two_handed(3, 0x20, 4, 0),
            DropSource::Unknown,
        );
        assert!(result.accepted);
        assert_eq!(editor.weapons.len(), 1);
        assert_eq!(editor.weapons[0].id, 3);
        assert!(editor.weapons[0].two_handed);
        assert_eq!(editor.points, 4);
    }

    #[test]
    fn an_armour_drop_replaces_the_same_item() {
        let mut editor = editor();
        editor.armour = vec![
            LoadoutItem::armour(1, 0x40, 2, 0),
            LoadoutItem::armour(2, 0x40, 3, 0),
        ];
        let result = apply_drop(
            &mut editor,
            &LoadoutItem::armour(1, 0x40, 5, 0),
            DropSource::Armour,
        );
        assert!(result.accepted);
        assert_eq!(editor.armour.len(), 2);
        assert_eq!(editor.points, 8);
        assert_eq!(editor.armour[1].points, 5);
        // Taking it out loses it.
        editor.points = points_used(&editor.weapons, &editor.armour);
        assert!(unequip(&mut editor, 1));
        assert_eq!(editor.points, 3);
        assert!(!unequip(&mut editor, 1));
    }

    #[test]
    fn champion_mode_takes_nothing() {
        let mut editor = editor();
        editor.mode = LoadoutMode::Champion;
        let result = apply_drop(&mut editor, &item(1, 2, 0), DropSource::Arsenal);
        assert!(!result.accepted);
        assert!(result.clear_pointer);
        assert!(!result.refresh);
        assert!(editor.weapons.is_empty());
    }

    #[test]
    fn the_paint_and_drag_flags_are_pinned() {
        assert_eq!(
            draw_order(),
            [
                LoadoutDraw::Backdrop,
                LoadoutDraw::Arsenal,
                LoadoutDraw::Equipment,
                LoadoutDraw::Preview
            ]
        );
        assert_eq!(BACKDROP_INK, 0xFF00_0000);
        assert_eq!(drag_flags(LoadoutMode::Match), 0x200);
        assert_eq!(drag_flags(LoadoutMode::Arena), 0x400);
        assert_eq!(drag_flags(LoadoutMode::Creation), 0x400);
        assert_eq!(drag_flags(LoadoutMode::Champion), 0);
        let mut editor = editor();
        let mut carved = item(1, 2, 0);
        carved.flags = SWAP_MASK;
        assert!(!drop_allowed(&editor, &carved, true));
        assert!(drop_allowed(&editor, &carved, false));
        assert!(drop_allowed(&editor, &item(2, 2, 0), true));
        editor.mode = LoadoutMode::Champion;
        assert!(!drop_allowed(&editor, &item(2, 2, 0), true));
        assert!(drop_allowed(&editor, &item(2, 2, 0), false));
    }

    #[test]
    fn the_pointed_item_is_cleared_only_in_the_arena() {
        let mut editor = editor();
        editor.pointed = Some(5);
        assert!(!clear_pointed(&mut editor, 4));
        assert!(clear_pointed(&mut editor, 5));
        assert_eq!(editor.pointed, None);
        editor.pointed = Some(5);
        editor.mode = LoadoutMode::Match;
        assert!(!clear_pointed(&mut editor, 5));
    }

    #[test]
    fn the_mode_words_change_the_constructors_records() {
        use crate::arena_forms::loadout as role;
        use crate::settings::HIDDEN;
        let hire = mode_records(LoadoutMode::Creation);
        assert_eq!((hire[role::PREVIOUS].w, hire[role::NEXT].x), (0x70, 0x78));
        assert_eq!(hire[role::NEXT].w, 0x70);
        // Dismiss to encumbrance (4..=9) and the shelves and boxes (11..=20)
        // are hidden; the panel (10) and CLOSE (21) keep their words.
        for index in (4..=9).chain(11..=20) {
            assert_eq!(hire[index].state, HIDDEN, "record {index}");
        }
        assert_eq!(hire[role::PANEL].state, LOADOUT_RECORDS_STATE[role::PANEL]);
        assert_eq!(hire[role::CLOSE].state, LOADOUT_RECORDS_STATE[role::CLOSE]);
        // The hire block, hidden in the shipped table, is shown.
        for index in 22..=27 {
            assert_eq!(LOADOUT_RECORDS_STATE[index], HIDDEN);
            assert_eq!(hire[index].state, 0, "record {index}");
        }
        let champion = mode_records(LoadoutMode::Champion);
        for index in [1, 6, 11, 12, 13, 14] {
            assert_eq!(champion[index].state, 0x10, "record {index}");
        }
        for index in [2, 3, 4, 7] {
            assert_eq!(champion[index].state, HIDDEN, "record {index}");
        }
        assert_eq!(champion[15].state, LOADOUT_RECORDS_STATE[15]);
        assert_eq!(
            mode_records(LoadoutMode::Arena),
            crate::arena_forms::LOADOUT_RECORDS
        );
    }

    #[test]
    fn the_arsenal_marks_the_records_hands_and_worn_items() {
        let mut entries = vec![item(1, 2, 0), item(2, 3, 0), item(3, 4, 0)];
        // The first entry keeps a mark that is not the equipped one.
        entries[0].flags = 0x400;
        mark_equipped(&mut entries, [2, 0], &[3]);
        assert_eq!(entries[0].flags & EQUIPPED_MARK, 0);
        assert_eq!(entries[0].flags, 0x400);
        assert_eq!(entries[1].flags & EQUIPPED_MARK, EQUIPPED_MARK);
        assert_eq!(entries[2].flags & EQUIPPED_MARK, EQUIPPED_MARK);
        // The next refresh takes the mark off the entries no longer held.
        mark_equipped(&mut entries, [0, 0], &[]);
        assert!(entries.iter().all(|entry| entry.flags & EQUIPPED_MARK == 0));
        assert_eq!(entries[0].flags, 0x400);
    }

    #[test]
    fn a_match_release_takes_the_last_weapon() {
        let mut editor = editor();
        editor.mode = LoadoutMode::Match;
        editor.weapons = vec![item(1, 2, 0), item(2, 3, 0)];
        assert!(release_for_key(&mut editor, 1));
        assert_eq!(editor.weapons.len(), 1);
        assert_eq!(editor.points, 2);
        assert!(!release_for_key(&mut editor, 8));
        editor.mode = LoadoutMode::Arena;
        assert!(!release_for_key(&mut editor, 1));
    }

    #[test]
    fn the_close_saves_the_kit_in_the_arena_and_match_modes() {
        let mut editor = editor();
        editor.rank = 2;
        let save = save_on_close(&editor);
        assert!(save.saves_kit && save.refresh_views && save.refresh_arena);
        assert!(!save.stores_record);
        assert_eq!(save.rank, 2);
        editor.mode = LoadoutMode::Match;
        assert_eq!(save_on_close(&editor), save);
        editor.mode = LoadoutMode::Creation;
        let save = save_on_close(&editor);
        assert!(!save.saves_kit && !save.refresh_views && save.refresh_arena);
        editor.mode = LoadoutMode::Champion;
        let save = save_on_close(&editor);
        assert!(save.stores_record && !save.saves_kit && !save.refresh_arena);
        assert_eq!(encumbrance(5.0), 250.0);
    }

    #[test]
    fn the_gate_counts_the_two_kit_containers_not_the_arsenal() {
        let mut editor = editor();
        editor.weapons = vec![item(1, 2, 0)];
        editor.armour = vec![item(1, 2, 0)];
        editor.arsenal = vec![item(1, 2, 0), item(2, 2, 0)];
        assert_eq!(container_copies(&editor.weapons, 1), 1);
        assert_eq!(container_copies(&editor.arsenal, 1), 1);
        assert_eq!(copies(&editor, 1), 2);
        assert_eq!(copies(&editor, 2), 0);
        // One hand holding it leaves the second copy free.
        assert!(can_equip(&editor, &item(1, 2, 0), 1));
        // Both copies held: the arsenal's copy does not count.
        assert!(!can_equip(&editor, &item(1, 2, 0), 2));
    }

    #[test]
    fn a_returned_item_is_sorted_into_the_container_it_left() {
        let mut editor = editor();
        let mut off = item(1, 2, 0);
        off.flags = OFFHAND_FLAG;
        // From the weapon list, the drag's own mark decides.
        assert_eq!(
            sort_drop(&mut editor, &off, DropSource::Weapons),
            DropOrigin::WeaponsOffHand
        );
        assert_eq!(
            sort_drop(&mut editor, &item(1, 2, 0), DropSource::Weapons).word(),
            2
        );
        assert_eq!(sort_drop(&mut editor, &off, DropSource::Armour).word(), 1);
        // Elsewhere: the weapon list's first entry with the id leaves and
        // its mark decides.
        editor.weapons = vec![item(3, 2, 0), off, item(1, 2, 0)];
        assert_eq!(
            sort_drop(&mut editor, &item(1, 2, 0), DropSource::Unknown),
            DropOrigin::WeaponsOffHand
        );
        assert_eq!(editor.weapons.len(), 2);
        assert_eq!(editor.weapons[1].flags, 0);
        // A worn entry counts only when no hand holds it.
        editor.armour = vec![LoadoutItem {
            flags: ARMOUR_KEEP,
            ..item(4, 2, 0)
        }];
        assert_eq!(
            sort_drop(&mut editor, &item(4, 2, 0), DropSource::Preview),
            DropOrigin::Unknown
        );
        editor.armour.push(item(4, 2, 0));
        assert_eq!(
            sort_drop(&mut editor, &item(4, 2, 0), DropSource::Preview),
            DropOrigin::Armour
        );
        assert_eq!(editor.armour.len(), 2);
    }

    #[test]
    fn an_off_hand_copy_frees_the_held_worn_entry() {
        let mut editor = editor();
        let held = LoadoutItem {
            flags: ARMOUR_KEEP,
            ..item(5, 3, 0)
        };
        editor.armour = vec![item(5, 3, 0), held, held, item(6, 1, 0)];
        let mut off = item(5, 3, 0);
        off.flags = OFFHAND_FLAG;
        let done = return_to_stock(&mut editor, &off, DropSource::Weapons, false);
        assert_eq!(done.origin, DropOrigin::WeaponsOffHand);
        assert!(done.freed_hand);
        // The first held entry leaves; the unheld one and the second held
        // one are counted.
        assert_eq!(done.count, 2);
        assert_eq!(editor.armour.len(), 3);
        assert_eq!(editor.points, 3 + 3 + 1);
    }

    #[test]
    fn a_returned_copy_counts_what_the_kit_still_holds() {
        // A worn copy from the worn list: three worn entries less one, plus
        // the weapon entry that is not an off-hand copy.
        let mut editor = editor();
        let mut off = item(7, 2, 0);
        off.flags = OFFHAND_FLAG;
        editor.armour = vec![item(7, 2, 0), item(7, 2, 0), item(7, 2, 0)];
        editor.weapons = vec![item(7, 2, 0), off];
        let done = return_to_stock(&mut editor, &item(7, 2, 0), DropSource::Armour, false);
        assert_eq!(done.origin, DropOrigin::Armour);
        assert_eq!(done.count, 3 - 1 + 1);
        assert_eq!(editor.armour.len(), 3);
        // From the preview the first unheld worn entry leaves instead of
        // the one taken off the count.
        let done = return_to_stock(&mut editor, &item(7, 2, 0), DropSource::Preview, false);
        // The weapon list's first entry with the id left in the sort.
        assert_eq!(done.origin, DropOrigin::Weapons);
        assert!(editor.weapons.is_empty());
        // Its off-hand copy was the preview's to remove; no plain entry is
        // left, and the three worn entries count.
        assert_eq!(done.count, 3);
        // A weapon from the weapon list: two plain entries less one, plus
        // one worn entry; the off-hand entry stays (not from the preview).
        let mut editor = editor_with(
            vec![item(8, 2, 0), off_of(8), item(8, 2, 0)],
            vec![item(8, 2, 0)],
        );
        let done = return_to_stock(&mut editor, &item(8, 2, 0), DropSource::Weapons, false);
        assert_eq!(done.count, 2 - 1 + 1);
        assert_eq!(editor.weapons.len(), 3);
    }

    fn off_of(id: u32) -> LoadoutItem {
        LoadoutItem {
            flags: OFFHAND_FLAG,
            ..item(id, 2, 0)
        }
    }

    fn editor_with(weapons: Vec<LoadoutItem>, armour: Vec<LoadoutItem>) -> LoadoutEditor {
        LoadoutEditor {
            weapons,
            armour,
            ..editor()
        }
    }

    #[test]
    fn the_arsenal_takes_back_a_lent_item_of_its_category() {
        let mut editor = editor();
        editor.category = 0x20;
        editor.arsenal = vec![LoadoutItem {
            flags: SWAP_MASK | 1,
            ..item(9, 2, 0)
        }];
        let done = return_to_stock(&mut editor, &item(9, 2, 0), DropSource::Unknown, false);
        assert!(done.released && !done.keep_item && !done.zero_stock);
        assert_eq!(editor.arsenal[0].flags, 1);
        assert_eq!(done.origin, DropOrigin::Unknown);
        assert_eq!(done.count, 0);
        // Not lent under the mask: the drag keeps its item.
        let done = return_to_stock(&mut editor, &item(9, 2, 0), DropSource::Unknown, false);
        assert!(!done.released && done.keep_item);
        // The arsenal says it lent it elsewhere: the item is dropped.
        let done = return_to_stock(&mut editor, &item(9, 2, 0), DropSource::Unknown, true);
        assert!(!done.keep_item && !done.zero_stock);
        // Another category is never searched; unlent, its count is zeroed.
        editor.arsenal[0].flags = SWAP_MASK;
        let other = LoadoutItem::weapon(9, 0x10, 2, 0);
        let done = return_to_stock(&mut editor, &other, DropSource::Unknown, false);
        assert!(!done.released && done.zero_stock && !done.keep_item);
        assert_eq!(editor.arsenal[0].flags, SWAP_MASK);
        // A champion's form only drops the item.
        editor.mode = LoadoutMode::Champion;
        let done = return_to_stock(&mut editor, &item(9, 2, 0), DropSource::Unknown, false);
        assert!(!done.released && !done.keep_item);
        assert_eq!(editor.arsenal[0].flags, SWAP_MASK);
    }

    #[test]
    fn a_match_return_drops_the_previews_off_hand_copy() {
        let mut editor = editor_with(vec![item(2, 2, 0), off_of(2), off_of(2)], vec![]);
        editor.mode = LoadoutMode::Match;
        let done = return_to_stock(&mut editor, &item(2, 2, 0), DropSource::Preview, false);
        // The sort took the first entry; the match branch the first
        // off-hand copy; the count is not taken in match mode.
        assert_eq!(done.origin, DropOrigin::Weapons);
        assert_eq!(editor.weapons, vec![off_of(2)]);
        assert_eq!(done.count, 0);
    }

    #[test]
    fn the_frame_draws_the_backdrop_and_the_layouts_parts() {
        let layout = layout(LoadoutMode::Arena, 1024, false);
        let inks = LoadoutInks {
            backdrop: [0, 0, 0, 255],
            arsenal: [1, 2, 3, 4],
            equipment: [5, 6, 7, 8],
            preview: [9, 10, 11, 12],
        };
        let drawn = frame(&draw_order(), &layout, &inks, 1024, 768);
        assert_eq!(drawn.layers.len(), 4);
        assert_eq!(drawn.layers[0].batch.primitives.len(), 1);
        // The arsenal is 4,180 228×492 (the interface is not wide); the
        // equipment hugs the right edge.
        assert_eq!(layout.arsenal, [4, 180, 228, 492]);
        assert_eq!(layout.equipment, [1024 - 236 - 4, 88, 236, 628]);
        let drawn = frame(
            &[LoadoutDraw::Arsenal, LoadoutDraw::Preview],
            &layout,
            &inks,
            1024,
            768,
        );
        assert_eq!(drawn.layers.len(), 2);
        // The palette's fills are the form's own colours.
        let palette = crate::line_skin::line_palette();
        let from_palette = LoadoutInks::from_palette(&palette);
        assert_eq!(from_palette.backdrop, [0, 0, 0, 255]);
        assert_eq!(from_palette.arsenal, palette.form[3]);
        assert_eq!(from_palette.preview, palette.form[5]);
        // Empty parts draw nothing.
        let drawn = frame(&[LoadoutDraw::Backdrop], &layout, &inks, 0, 768);
        assert_eq!(drawn.layers.len(), 0);
    }
}
