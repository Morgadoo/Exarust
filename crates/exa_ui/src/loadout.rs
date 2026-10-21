//! The arena loadout editor: one fighter's kit for one rank, the arsenal's
//! swaps, the equipment-point limit and the save on close.
//!
//! Evidence: the loadout form's routines in the retail image. The form is
//! 1024 × 768 pixels with two item containers and the character-creation
//! preview. Mode 1 (creation) adds a name box and dims nothing; mode 3
//! (champion) only shows the kit. An item may be dropped when its rank fits
//! the rank's maximum and its points fit the rank's limit; dropping an
//! arsenal item of the form's own category swaps the one already there;
//! a two-handed weapon clears the off hand. The preview's encumbrance is
//! `strength² × 10`.

/// The form's width in pixels.
pub const FORM_WIDTH: i32 = 0x400;
/// The form's height in pixels.
pub const FORM_HEIGHT: i32 = 0x300;
/// The form's opacity when the interface is wide.
pub const FORM_OPACITY: f32 = 0.837_398_35;
/// The arsenal container's x, y, width and height before the opacity scale.
pub const ARSENAL_X: i32 = 4;
pub const ARSENAL_Y: i32 = 0xe4;
pub const ARSENAL_WIDTH: i32 = 0xb4;
pub const ARSENAL_HEIGHT: f32 = 492.0;
/// The equipment container's place.
pub const EQUIP_X: i32 = 0xec;
pub const EQUIP_WIDTH_MARGIN: i32 = 4;
pub const EQUIP_HEIGHT: i32 = 0x58;
pub const EQUIP_Y: i32 = 0x274;
pub const EQUIP_Y_WIDE: i32 = 0x224;
/// The preview's width.
pub const PREVIEW_WIDTH: i32 = 0xec;
/// The preview sits this far in from each side.
pub const PREVIEW_MARGIN: i32 = 8;
/// The name box in creation mode.
pub const NAME_BOX_X: i32 = 0xec;
pub const NAME_BOX_Y: i32 = 0x100;
pub const NAME_BOX_HEIGHT: i32 = 0x20;
/// The mask that decides whether an arsenal item is swapped out.
pub const SWAP_MASK: u32 = 0x400;
/// The flag that marks a two-handed weapon's off hand.
pub const OFFHAND_FLAG: u32 = 0x8000_0000;
/// The flag of an item that may stay in the armour list.
pub const ARMOUR_KEEP: u32 = 0x100;
/// The point-limit check's unset rank.
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

/// The form's layout for one opacity and interface width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadoutLayout {
    pub arsenal: [f32; 4],
    pub equipment: [f32; 4],
    pub preview: [f32; 4],
    pub name_box: [f32; 4],
    pub opacity: f32,
}

/// The layout the constructor computes. The arsenal's height and the
/// preview's size follow the opacity; the equipment container fills the
/// client width.
pub fn layout(
    mode: LoadoutMode,
    client_width: i32,
    client_height: i32,
    wide: bool,
) -> LoadoutLayout {
    let opacity = if wide { FORM_OPACITY } else { 1.0 };
    let arsenal_height = (f64::from(opacity) * f64::from(ARSENAL_HEIGHT)).floor() as f32;
    let equip_y = if wide { EQUIP_Y_WIDE } else { EQUIP_Y };
    let equip_width = client_width - EQUIP_X - EQUIP_WIDTH_MARGIN;
    let preview_height = client_height - 4;
    let name_width = if mode == LoadoutMode::Creation {
        client_width - NAME_BOX_X - EQUIP_WIDTH_MARGIN
    } else {
        equip_width
    };
    LoadoutLayout {
        arsenal: [
            ARSENAL_X as f32,
            ARSENAL_Y as f32,
            ARSENAL_WIDTH as f32,
            arsenal_height,
        ],
        equipment: [
            EQUIP_X as f32,
            equip_y as f32,
            equip_width as f32,
            EQUIP_HEIGHT as f32,
        ],
        preview: [
            PREVIEW_WIDTH as f32,
            0.0,
            (client_width - 2 * PREVIEW_WIDTH - PREVIEW_MARGIN) as f32,
            preview_height as f32,
        ],
        name_box: [
            NAME_BOX_X as f32,
            NAME_BOX_Y as f32,
            name_width as f32,
            NAME_BOX_HEIGHT as f32,
        ],
        opacity,
    }
}

/// The form's opacity: 0.83739835 when the interface is wide, else one.
pub fn opacity(wide: bool) -> f32 {
    if wide { FORM_OPACITY } else { 1.0 }
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

/// Whether an item may be equipped: the copies already held must not exceed
/// the free hands, its rank must fit the maximum and its points the limit.
/// In champion mode every item is accepted; in match mode only the armour
/// container's own items.
pub fn can_equip(editor: &LoadoutEditor, item: &LoadoutItem, hands_holding: usize) -> bool {
    if editor.mode == LoadoutMode::Champion {
        return true;
    }
    if editor.mode != LoadoutMode::Match && !item.allowed {
        return false;
    }
    let in_lists = editor
        .weapons
        .iter()
        .chain(editor.armour.iter())
        .any(|held| held.id == item.id);
    if in_lists && copies(editor, item.id) <= hands_holding {
        return false;
    }
    let rank = if editor.max_rank < 0 {
        MAX_RANK_UNSET
    } else {
        editor.max_rank
    };
    item.rank <= rank && editor.points + item.points <= editor.limit
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

/// Applies a drop: finds the source, removes the item from the list it came
/// from, clears the off hand for a two-handed weapon and takes the arsenal's
/// matching item when the drop swaps.
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
    if !accepted || (editor.mode != LoadoutMode::Champion && !can_equip(editor, item, 0)) {
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
        DropSource::Arsenal | DropSource::Unknown => {
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

/// What closing the form saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutSave {
    /// The mode that writes a loadout.
    pub writes: bool,
    /// The rank whose loadout is written.
    pub rank: usize,
    /// The character view needs a refresh.
    pub refresh_view: bool,
    /// The arena's character cards are refreshed.
    pub refresh_arena: bool,
}

/// The close's save: creation and match modes write the name only; a
/// champion leaves; the arena writes the rank's loadout and refreshes the
/// character and the arena.
pub fn save_on_close(editor: &LoadoutEditor) -> LoadoutSave {
    match editor.mode {
        LoadoutMode::Creation => LoadoutSave {
            writes: false,
            rank: editor.rank,
            refresh_view: true,
            refresh_arena: false,
        },
        LoadoutMode::Champion => LoadoutSave {
            writes: false,
            rank: editor.rank,
            refresh_view: false,
            refresh_arena: true,
        },
        LoadoutMode::Arena | LoadoutMode::Match => LoadoutSave {
            writes: true,
            rank: editor.rank,
            refresh_view: true,
            refresh_arena: editor.mode == LoadoutMode::Arena,
        },
    }
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
    fn the_layout_scales_the_arsenal_and_preview() {
        let wide = layout(LoadoutMode::Arena, 1024, 768, true);
        assert_eq!(wide.arsenal[0..3], [4.0, 228.0, 180.0]);
        assert_eq!(wide.arsenal[3], 411.0);
        assert_eq!(wide.equipment[0..2], [236.0, 548.0]);
        assert_eq!(wide.equipment[2], 784.0);
        assert_eq!(wide.equipment[3], 88.0);
        assert_eq!(wide.preview, [236.0, 0.0, 544.0, 764.0]);
        assert!((wide.opacity - 0.837_398_35).abs() < 1e-7);
        let narrow = layout(LoadoutMode::Arena, 800, 600, false);
        assert_eq!(narrow.arsenal[3], 492.0);
        assert_eq!(narrow.equipment[1], 628.0);
        assert_eq!(narrow.preview[3], 596.0);
        assert_eq!(narrow.opacity, 1.0);
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
    fn the_close_saves_the_arena_rank_only() {
        let mut editor = editor();
        editor.rank = 2;
        let save = save_on_close(&editor);
        assert!(save.writes);
        assert_eq!(save.rank, 2);
        assert!(save.refresh_view);
        assert!(save.refresh_arena);
        editor.mode = LoadoutMode::Match;
        let save = save_on_close(&editor);
        assert!(save.writes);
        assert!(!save.refresh_arena);
        editor.mode = LoadoutMode::Creation;
        assert!(!save_on_close(&editor).writes);
        editor.mode = LoadoutMode::Champion;
        assert!(!save_on_close(&editor).writes);
        assert_eq!(encumbrance(5.0), 250.0);
    }
}
