//! The arena loadout editor: one fighter's kit for one rank, the arsenal's
//! swaps, the equipment-point limit and the save on close.
//!
//! Evidence: the loadout form's routines in the retail image, read from
//! the instructions. The form is 1024 × 768 with the character-creation
//! preview between the arsenal (left) and the equipment (right). Mode 1
//! (creation) adds a name box and edits rank 5; mode 3 (champion) only
//! shows the kit. An item may be equipped when its rank is at most the
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

/// How many copies of an item the lists hold.
pub fn copies(editor: &LoadoutEditor, id: u32) -> usize {
    editor
        .weapons
        .iter()
        .chain(editor.armour.iter())
        .filter(|item| item.id == id)
        .count()
}

/// How many entries of one container hold an item, by its catalogue id.
pub fn container_copies(entries: &[LoadoutItem], id: u32) -> usize {
    entries.iter().filter(|item| item.id == id).count()
}

/// How many copies of an item the equipment and the arsenal hold together:
/// the drop gate counts the arsenal's and the equipment's entries for the
/// item before it compares them with the hands that hold it.
pub fn kit_copies(editor: &LoadoutEditor, id: u32) -> usize {
    copies(editor, id) + container_copies(&editor.arsenal, id)
}

/// The list model's equip check, through [`may_equip`]: an item already in
/// the lists is a drag out of the kit, held by `hands_holding` hands; its
/// rank and points come from the list item.
pub fn can_equip(editor: &LoadoutEditor, item: &LoadoutItem, hands_holding: usize) -> bool {
    let facts = EquipFacts {
        from_kit: editor
            .weapons
            .iter()
            .chain(editor.armour.iter())
            .any(|held| held.id == item.id),
        held: hands_holding as u32,
        copies: kit_copies(editor, item.id) as u32,
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
    /// Another container, matched by the item's id.
    Unknown,
}

/// The container the drop handler sorts a dragged item into. The original
/// keeps the equipment in one container (the armour bit 0x100 marks the
/// worn entries, the top bit the off hand) and the arsenal in another; a
/// drag from the arsenal is a weapon (2) or an off-hand copy (3), a drag
/// from the equipment is 1, and anything else is 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropOrigin {
    /// No container the handler recognises.
    Unknown = 0,
    /// The equipment container (a drag within the kit).
    Equipment = 1,
    /// The arsenal, as a weapon.
    Arsenal = 2,
    /// The arsenal, as an off-hand copy.
    ArsenalOffHand = 3,
}

impl DropOrigin {
    /// The word the handler's branch selector takes.
    pub fn word(self) -> u32 {
        self as u32
    }

    /// The drag comes from the arsenal.
    pub fn from_arsenal(self) -> bool {
        matches!(self, DropOrigin::Arsenal | DropOrigin::ArsenalOffHand)
    }

    /// The arsenal entry is an off-hand copy.
    pub fn off_hand(self) -> bool {
        self == DropOrigin::ArsenalOffHand
    }
}

/// What sorting a drag into a container found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassifiedDrop {
    pub origin: DropOrigin,
    /// The arsenal held the item under the form's swap mask, so the drag
    /// takes that copy instead of adding one (the handler clears the mask
    /// bit on the entry).
    pub swapped: bool,
}

impl ClassifiedDrop {
    const UNKNOWN: Self = Self {
        origin: DropOrigin::Unknown,
        swapped: false,
    };
}

/// Sorts a drag into its container, as the handler does before it touches
/// any list.
///
/// A drag the caller places in the equipment is an equipment drag. A drag
/// from the arsenal is an arsenal weapon or off-hand copy, and an entry
/// under the form's swap mask is given up (its mask bit cleared). A drag of
/// unknown origin is looked for in the arsenal first, then in the
/// equipment; an item in neither is [`DropOrigin::Unknown`].
pub fn classify_source(
    editor: &mut LoadoutEditor,
    item: &LoadoutItem,
    source: DropSource,
) -> ClassifiedDrop {
    let origin_of = |flags: u32| {
        if flags & OFFHAND_FLAG == 0 {
            DropOrigin::Arsenal
        } else {
            DropOrigin::ArsenalOffHand
        }
    };
    match source {
        DropSource::Weapons | DropSource::Armour => ClassifiedDrop {
            origin: DropOrigin::Equipment,
            swapped: false,
        },
        DropSource::Arsenal => match editor.arsenal.iter().position(|held| held.id == item.id) {
            Some(index) => {
                let flags = editor.arsenal[index].flags;
                let swapped = flags & editor.mask != 0;
                if swapped {
                    editor.arsenal[index].flags &= !editor.mask;
                }
                ClassifiedDrop {
                    origin: origin_of(flags),
                    swapped,
                }
            }
            // The arsenal container does not hold the item: the handler
            // still treats it as an arsenal drag (guess: the drag record
            // carries the container it started in).
            None => ClassifiedDrop {
                origin: origin_of(item.flags),
                swapped: false,
            },
        },
        DropSource::Unknown => {
            if let Some(index) = editor.arsenal.iter().position(|held| held.id == item.id) {
                let flags = editor.arsenal[index].flags;
                let swapped = flags & editor.mask != 0;
                if swapped {
                    editor.arsenal[index].flags &= !editor.mask;
                }
                ClassifiedDrop {
                    origin: origin_of(flags),
                    swapped,
                }
            } else if editor
                .weapons
                .iter()
                .chain(editor.armour.iter())
                .any(|held| held.id == item.id)
            {
                ClassifiedDrop {
                    origin: DropOrigin::Equipment,
                    swapped: false,
                }
            } else {
                ClassifiedDrop::UNKNOWN
            }
        }
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
    /// The container the handler sorted the drag into.
    pub origin: DropOrigin,
    /// The item's slot is the off hand.
    pub off_hand: bool,
    /// How many copies of the item the equipment still holds: the original
    /// hands this count to the preview's loadout record.
    pub armed: u32,
    /// The preview must be refreshed.
    pub refresh: bool,
    /// The form clears its pointer to the item.
    pub clear_pointer: bool,
}

/// Applies a drop the way the handler walks it: sort the drag into its
/// container, give up an arsenal copy under the swap mask, take the
/// displaced entry out of the equipment, then put the item in its slot.
///
/// The three container branches differ in what they displace: an arsenal
/// off-hand copy takes the first worn entry with the same id that is not
/// armour; an arsenal weapon takes the arsenal's off-hand copy of it; a
/// drag within the equipment moves the item out of the slot it was in.
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
            origin: DropOrigin::Unknown,
            off_hand: false,
            armed: 0,
            refresh: false,
            clear_pointer: true,
        };
    }
    let classified = classify_source(editor, item, source);
    let origin = classified.origin;
    if classified.swapped {
        // The arsenal entry gave up its copy: the equipment is left alone.
        editor.points = points_used(&editor.weapons, &editor.armour);
        return DropResult {
            accepted: false,
            swapped: true,
            source,
            origin,
            off_hand: origin.off_hand(),
            armed: copies(editor, item.id) as u32,
            refresh: true,
            clear_pointer: true,
        };
    }
    if !can_equip(editor, item, 0) {
        return DropResult {
            accepted: false,
            swapped: false,
            source,
            origin,
            off_hand: false,
            armed: 0,
            refresh: false,
            clear_pointer: true,
        };
    }
    match origin {
        DropOrigin::ArsenalOffHand => {
            // The off-hand copy replaces the first worn entry with the same
            // id that is not armour.
            if let Some(index) = editor
                .armour
                .iter()
                .position(|held| held.id == item.id && held.flags & ARMOUR_KEEP == 0)
            {
                editor.armour.remove(index);
            }
        }
        DropOrigin::Arsenal => {
            // An arsenal weapon takes the arsenal's off-hand copy out of
            // the equipment.
            if let Some(index) = editor
                .armour
                .iter()
                .position(|held| held.id == item.id && held.flags & OFFHAND_FLAG != 0)
            {
                editor.armour.remove(index);
            }
        }
        DropOrigin::Equipment | DropOrigin::Unknown => {
            // The drag moves the item: its own entry leaves the equipment.
            editor.weapons.retain(|held| held.id != item.id);
            editor.armour.retain(|held| held.id != item.id);
        }
    }
    let off_hand = item.flags & OFFHAND_FLAG != 0;
    let slot = if item.two_handed {
        Slot::TwoHanded
    } else if item.worn {
        Slot::Armour
    } else {
        Slot::Weapon
    };
    match slot {
        Slot::Armour => {
            editor.armour.retain(|held| held.id != item.id);
            editor.armour.push(*item);
        }
        Slot::Weapon => {
            editor.weapons.retain(|held| held.id != item.id);
            editor.weapons.push(*item);
        }
        Slot::TwoHanded => {
            // The two-handed weapon takes both hands: the off hand goes.
            if item.two_handed {
                editor.weapons.clear();
                editor.armour.retain(|held| held.flags & ARMOUR_KEEP != 0);
            }
            editor.weapons.retain(|held| held.id != item.id);
            editor.weapons.push(*item);
        }
    }
    editor.points = points_used(&editor.weapons, &editor.armour);
    DropResult {
        accepted: true,
        swapped: false,
        source,
        origin,
        off_hand,
        armed: copies(editor, item.id) as u32,
        refresh: true,
        clear_pointer: false,
    }
}

/// The match-mode release: a key other than 8 takes the last arsenal entry
/// away. Every other mode, and key 8, leaves the arsenal alone.
pub fn drop_cancelled(editor: &mut LoadoutEditor, key: i32) -> bool {
    if editor.mode != LoadoutMode::Match || key == 8 {
        return false;
    }
    editor.arsenal.pop().is_some()
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

/// One of the form's control records, decoded: class and id, the
/// x and y words (a negative y is the layout pass's anchor code), width and
/// height, the caption, and the style and state words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormControl {
    pub class: u16,
    pub id: u16,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub text: Option<&'static str>,
    pub style: u32,
    pub state: u32,
}

/// The arsenal form's 27 control records, in the constructor's order:
/// the class and id, the x and y words (negative anchors the layout pass
/// resolves), the width and height, the caption, the style word and the
/// state word.
pub const CONTROLS: [FormControl; 27] = [
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 200,
        y: 1,
        w: 160,
        h: 0,
        text: Some("ARSENAL"),
        style: 0x3b00,
        state: 0x0,
    },
    FormControl {
        class: 0x12,
        id: 0x0,
        x: 8,
        y: 5,
        w: 220,
        h: 0,
        text: None,
        style: 0x8403320,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: 4,
        y: -32762,
        w: 76,
        h: 24,
        text: Some("Previous"),
        style: 0x403320,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: -32759,
        y: -32768,
        w: 76,
        h: 24,
        text: Some("Next"),
        style: 0x403320,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: -32760,
        y: -32768,
        w: 64,
        h: 24,
        text: Some("Dismiss"),
        style: 0x403320,
        state: 0x0,
    },
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 8,
        y: -32763,
        w: 40,
        h: 0,
        text: Some("Rank"),
        style: 0x0,
        state: 0x0,
    },
    FormControl {
        class: 0x4,
        id: 0x102,
        x: 56,
        y: -32768,
        w: 112,
        h: 0,
        text: Some("Inept"),
        style: 0x405320,
        state: 0x0,
    },
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 52,
        y: -32760,
        w: 228,
        h: 28,
        text: Some("Points: 0 / 5"),
        style: 0x0,
        state: 0x0,
    },
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 8,
        y: -32760,
        w: 12,
        h: 28,
        text: Some("Encumbrance:"),
        style: 0x0,
        state: 0x0,
    },
    FormControl {
        class: 0xac01,
        id: 0x0,
        x: 100,
        y: -32768,
        w: 12,
        h: 28,
        text: None,
        style: 0x0,
        state: 0x0,
    },
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 4,
        y: 105,
        w: 228,
        h: 0,
        text: None,
        style: 0x3300,
        state: 0x0,
    },
    FormControl {
        class: 0x3,
        id: 0x0,
        x: 12,
        y: 4,
        w: 44,
        h: 44,
        text: None,
        style: 0x1420080,
        state: 0x0,
    },
    FormControl {
        class: 0x3,
        id: 0x0,
        x: -32758,
        y: -32768,
        w: 44,
        h: 44,
        text: None,
        style: 0x1420080,
        state: 0x0,
    },
    FormControl {
        class: 0x3,
        id: 0x0,
        x: -32758,
        y: -32768,
        w: 44,
        h: 44,
        text: None,
        style: 0x1420080,
        state: 0x0,
    },
    FormControl {
        class: 0x3,
        id: 0x0,
        x: -32758,
        y: -32768,
        w: 44,
        h: 44,
        text: None,
        style: 0x1420080,
        state: 0x0,
    },
    FormControl {
        class: 0x3,
        id: 0x0,
        x: -32758,
        y: -32768,
        w: 44,
        h: 44,
        text: None,
        style: 0x1420080,
        state: 0x60,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: 16,
        y: -32764,
        w: 36,
        h: 0,
        text: Some("5"),
        style: 0x1423300,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: -32760,
        y: -32768,
        w: 36,
        h: 0,
        text: Some("4"),
        style: 0x1423300,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: -32760,
        y: -32768,
        w: 36,
        h: 0,
        text: Some("3"),
        style: 0x1423300,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: -32760,
        y: -32768,
        w: 36,
        h: 0,
        text: Some("2"),
        style: 0x1423300,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: -32760,
        y: -32768,
        w: 36,
        h: 0,
        text: Some("1"),
        style: 0x1423300,
        state: 0x60,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: 36,
        y: 0,
        w: 164,
        h: 28,
        text: Some("CLOSE"),
        style: 0x20483300,
        state: 0x0,
    },
    FormControl {
        class: 0x1,
        id: 0x0,
        x: 40,
        y: 0,
        w: 164,
        h: 28,
        text: Some("HIRE"),
        style: 0x204a3300,
        state: 0x110,
    },
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 108,
        y: 13,
        w: 96,
        h: 0,
        text: Some("Hire Fee:"),
        style: 0xa2300,
        state: 0x110,
    },
    FormControl {
        class: 0xac03,
        id: 0x0,
        x: 72,
        y: 13,
        w: 48,
        h: 0,
        text: None,
        style: 0xa0000,
        state: 0x110,
    },
    FormControl {
        class: 0x5,
        id: 0x0,
        x: 108,
        y: 8,
        w: 96,
        h: 0,
        text: Some("Upkeep:"),
        style: 0xa2300,
        state: 0x110,
    },
    FormControl {
        class: 0xac03,
        id: 0x0,
        x: 72,
        y: 8,
        w: 48,
        h: 0,
        text: None,
        style: 0xa0000,
        state: 0x110,
    },
];

/// The form's controls as the control layout pass reads them, placed in a
/// client (see [`crate::gui_controls::layout_records`]).
pub fn controls(
    unit: i32,
    client: crate::gui_controls::LayoutClient,
) -> Vec<crate::gui_controls::Control> {
    let mut records: Vec<crate::gui_controls::Control> = CONTROLS
        .iter()
        .map(|record| {
            let mut control = crate::gui_controls::Control::new(
                record.class,
                record.x,
                record.y,
                record.w,
                record.h,
            );
            control.text = record
                .text
                .map(|text| text.as_bytes().to_vec())
                .unwrap_or_default();
            control.style = record.style;
            control.state = record.state;
            control
        })
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
        assert_eq!(CONTROLS.len(), 27);
        assert_eq!(CONTROLS[0].text, Some("ARSENAL"));
        assert_eq!(CONTROLS[2].text, Some("Previous"));
        assert_eq!(CONTROLS[4].text, Some("Dismiss"));
        assert_eq!(CONTROLS[7].text, Some("Points: 0 / 5"));
        assert_eq!(CONTROLS[8].text, Some("Encumbrance:"));
        assert_eq!(CONTROLS[21].text, Some("CLOSE"));
        assert_eq!(CONTROLS[21].style, 0x2048_3300);
        assert_eq!(CONTROLS[22].state, 0x110);
        let client = crate::gui_controls::LayoutClient {
            inner_right: 1020,
            border: 2,
            bottom: 766,
        };
        let controls = controls(4, client);
        assert_eq!(controls.len(), 27);
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
    fn the_kit_and_the_arsenal_count_the_same_item_together() {
        let mut editor = editor();
        editor.weapons = vec![item(1, 2, 0)];
        editor.armour = vec![item(1, 2, 0)];
        editor.arsenal = vec![item(1, 2, 0), item(2, 2, 0)];
        assert_eq!(copies(&editor, 1), 2);
        assert_eq!(container_copies(&editor.arsenal, 1), 1);
        assert_eq!(kit_copies(&editor, 1), 3);
        assert_eq!(kit_copies(&editor, 2), 1);
        assert_eq!(kit_copies(&editor, 3), 0);
        // Two hands holding the item leave one arsenal copy to add.
        assert!(can_equip(&editor, &item(1, 2, 0), 2));
        // A fourth copy needs a fourth hand the fighter does not have.
        editor.arsenal.push(item(1, 2, 0));
        assert_eq!(kit_copies(&editor, 1), 4);
        assert!(!can_equip(&editor, &item(1, 2, 0), 4));
    }

    #[test]
    fn a_drag_is_sorted_into_its_container() {
        let mut editor = editor();
        editor.arsenal = vec![
            LoadoutItem {
                flags: SWAP_MASK,
                ..item(1, 2, 0)
            },
            item(2, 2, 0),
        ];
        // An arsenal entry under the mask gives its copy up.
        let sorted = classify_source(&mut editor, &item(1, 2, 0), DropSource::Arsenal);
        assert_eq!(sorted.origin, DropOrigin::Arsenal);
        assert!(sorted.swapped);
        assert_eq!(editor.arsenal[0].flags, 0);
        // An off-hand arsenal entry is the off-hand copy.
        editor.arsenal[0].flags = OFFHAND_FLAG | SWAP_MASK;
        let sorted = classify_source(&mut editor, &item(1, 2, 0), DropSource::Arsenal);
        assert_eq!(sorted.origin, DropOrigin::ArsenalOffHand);
        assert!(sorted.swapped);
        assert_eq!(sorted.origin.word(), 3);
        // A drag placed in the equipment is an equipment drag.
        let sorted = classify_source(&mut editor, &item(1, 2, 0), DropSource::Armour);
        assert_eq!(sorted.origin, DropOrigin::Equipment);
        assert_eq!(sorted.origin.word(), 1);
        // Unknown origin is found in the equipment when the arsenal has no
        // entry for it.
        editor.weapons = vec![item(3, 2, 0)];
        let sorted = classify_source(&mut editor, &item(3, 2, 0), DropSource::Unknown);
        assert_eq!(sorted.origin, DropOrigin::Equipment);
        let sorted = classify_source(&mut editor, &item(9, 2, 0), DropSource::Unknown);
        assert_eq!(sorted.origin, DropOrigin::Unknown);
        assert_eq!(sorted.origin.word(), 0);
        assert!(!sorted.origin.from_arsenal());
        assert!(DropOrigin::ArsenalOffHand.from_arsenal());
        assert!(DropOrigin::ArsenalOffHand.off_hand());
        assert!(!DropOrigin::Arsenal.off_hand());
    }

    #[test]
    fn an_off_hand_copy_takes_the_worn_entry_with_its_id() {
        let mut editor = editor();
        editor.weapons = vec![item(1, 2, 0)];
        editor.armour = vec![
            LoadoutItem {
                flags: ARMOUR_KEEP,
                ..item(2, 3, 0)
            },
            item(3, 4, 0),
        ];
        let mut copy = item(3, 4, 0);
        copy.flags = OFFHAND_FLAG;
        let result = apply_drop(&mut editor, &copy, DropSource::Arsenal);
        assert!(result.accepted);
        assert_eq!(result.origin, DropOrigin::ArsenalOffHand);
        assert!(result.off_hand);
        // The worn entry with that id leaves; the armour entry stays.
        assert_eq!(editor.armour.len(), 1);
        assert_eq!(editor.armour[0].id, 2);
        assert_eq!(result.armed, 1);
        assert_eq!(editor.points, 9);
    }

    #[test]
    fn an_equipment_drag_moves_the_item_between_the_lists() {
        let mut editor = editor();
        editor.weapons = vec![item(1, 2, 0)];
        editor.armour = vec![LoadoutItem {
            flags: ARMOUR_KEEP,
            ..item(2, 3, 0)
        }];
        let mut moved = LoadoutItem::armour(1, 0x40, 2, 0);
        moved.flags = ARMOUR_KEEP;
        let result = apply_drop(&mut editor, &moved, DropSource::Weapons);
        assert!(result.accepted);
        assert_eq!(result.origin, DropOrigin::Equipment);
        assert!(editor.weapons.is_empty());
        assert_eq!(editor.armour.len(), 2);
        assert_eq!(editor.points, 5);
    }

    #[test]
    fn a_cancelled_match_drop_takes_the_last_arsenal_entry() {
        let mut editor = editor();
        editor.arsenal = vec![item(1, 2, 0), item(2, 3, 0)];
        editor.mode = LoadoutMode::Match;
        assert!(!drop_cancelled(&mut editor, 8));
        assert_eq!(editor.arsenal.len(), 2);
        assert!(drop_cancelled(&mut editor, 1));
        assert_eq!(editor.arsenal.len(), 1);
        editor.mode = LoadoutMode::Arena;
        assert!(!drop_cancelled(&mut editor, 1));
        assert_eq!(editor.arsenal.len(), 1);
    }
}
