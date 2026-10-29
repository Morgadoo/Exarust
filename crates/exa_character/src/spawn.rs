//! What a record holds and where its spawn stands: the generator's equip
//! pass and the spawn's motion and height rules.
//!
//! The plans are plain data. The game uses the equip plan for the hands it
//! can draw; shields and the off-hand torch stay plans until the renderer
//! has a place for them, and the spawn heights are not wired.

use exa_formats::characters::{CharacterRecord, ItemRef};
use exa_formats::objdb::Weapon;

/// Where an item's model comes from: a factory recipe or an entry of the
/// object model library the generator keeps (not the `Characters` package,
/// which the apparel loader reads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSource {
    Factory(String),
    Library(String),
}

/// How an item is shown and held, as the item-model loader plans it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemModelPlan {
    pub source: ModelSource,
    pub kind: EquipKind,
    /// The side of the generated skin textures, set only for an item with
    /// a body section: `0x400` when high-resolution skins are on and the
    /// pixel-format setting is 3 or more, else `0x200`.
    pub skin_side: Option<u32>,
    /// The word the loader writes through every node of the model: the
    /// item's id.
    pub owner: u32,
}

/// The item-model loader's plan. The model is the item's model section
/// (a name starting `@` is a factory recipe, any other a library entry);
/// an item without one has no model. A weapon section makes it a weapon,
/// else a shield section a shield; an item with neither is not held. The
/// loader also writes the item's id through the model's nodes.
pub fn item_model_plan(
    item: &exa_formats::objdb::Item,
    pixel_format: i32,
    high_res_skins: bool,
) -> Option<ItemModelPlan> {
    if item.model.is_empty() {
        return None;
    }
    let source = match item.model.strip_prefix('@') {
        Some(factory) => ModelSource::Factory(factory.to_string()),
        None => ModelSource::Library(item.model.clone()),
    };
    let kind = if item.weapon.is_some() {
        EquipKind::Weapon
    } else if item.shield.is_some() {
        EquipKind::Shield
    } else {
        return None;
    };
    let skin_side = item
        .burden
        .is_some()
        .then_some(if high_res_skins && pixel_format >= 3 {
            0x400
        } else {
            0x200
        });
    Some(ItemModelPlan {
        source,
        kind,
        skin_side,
        owner: item.id,
    })
}

/// Whether a material name is one the generator makes at run time: its
/// first character is `&`. Those are valid without a library entry.
pub fn is_runtime_material(name: &str) -> bool {
    name.starts_with('&')
}

/// The flag the `&` material pass sets on a run-time material's instance.
pub const RUNTIME_MATERIAL_FLAG: u32 = 0x8_0000;

/// The generator's `&` material pass for one material instance: a name that
/// starts `&` clears the instance's validity word and sets
/// [`RUNTIME_MATERIAL_FLAG`]. Returns whether it applied.
pub fn mark_runtime_material(name: &str, flags: &mut u32, validity: &mut u32) -> bool {
    if !is_runtime_material(name) {
        return false;
    }
    *validity = 0;
    *flags |= RUNTIME_MATERIAL_FLAG;
    true
}

/// The item-model loader's `*` material fix-up: a material instance whose
/// name starts `*` (the body section's placeholder) has its value cleared
/// and its `0x10` flag set.
pub fn clear_wildcard_material(name: &str, flags: &mut u32, value: &mut u32) -> bool {
    if !name.starts_with('*') {
        return false;
    }
    *value = 0;
    *flags |= 0x10;
    true
}

/// How the record's role is set at build time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleSet {
    /// Clear the relations and set the role.
    Force,
    /// Set the role only when the character does not hold one already.
    Try,
}

/// The role the record's profession word gives at build time: the arena and
/// creature races (`0x0500` to `0x0900`) take the hostile role 11 by force;
/// races `0x1100` and `0x1200` take roles 45 and 46 when they hold none.
/// Every other race keeps its record's roles.
pub fn creator_role(profession: u32) -> Option<(u32, RoleSet)> {
    match profession & 0xFF00 {
        0x0500 | 0x0600 | 0x0700 | 0x0800 | 0x0900 => Some((0x0B, RoleSet::Force)),
        0x1100 => Some((0x2D, RoleSet::Try)),
        0x1200 => Some((0x2E, RoleSet::Try)),
        _ => None,
    }
}

/// What a hand holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Equipped {
    /// The model entry (or `@factory`).
    pub model: String,
    /// How the item is held.
    pub kind: EquipKind,
    /// The weapon section, for a weapon.
    pub weapon: Option<Weapon>,
    /// The shield section, for a shield.
    pub shield_record: Option<exa_formats::weapon_record::ShieldRecord>,
    /// The record's flags for this hand.
    pub flags: u32,
}

/// The three ways the equip pass can take a hand's item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipKind {
    /// A weapon in the main or off hand.
    Weapon,
    /// A shield.
    Shield,
    /// The off hand's torch (an item with the torch section).
    Torch,
}

/// A hand's item as the equip pass resolves it (the level's own or the
/// game's database).
#[derive(Clone, Debug, PartialEq)]
pub struct HandEntry {
    pub model: String,
    pub weapon: Option<Weapon>,
    pub shield: bool,
    /// The shield's own record, when the item has one.
    pub shield_record: Option<exa_formats::weapon_record::ShieldRecord>,
    /// The item has the torch section.
    pub torch: bool,
}

/// The equip pass over a record's first two hands. `resolve` gets the hand
/// index and its item reference.
///
/// The off hand's item takes the torch path when it has the torch section;
/// otherwise a weapon section makes it a weapon, else a shield section a
/// shield, and an item with neither is not equipped. Both hands are always
/// visited. The record's flags for each hand are kept so the holder can
/// apply its bits.
pub fn equip_hands(
    record: &CharacterRecord,
    mut resolve: impl FnMut(usize, ItemRef) -> Option<HandEntry>,
) -> [Option<Equipped>; 2] {
    let mut out = [None, None];
    for (hand, slot) in out.iter_mut().enumerate() {
        let r = record.hands[hand];
        let Some(item) = r.is_some().then(|| resolve(hand, r)).flatten() else {
            continue;
        };
        let kind = if hand == 1 && item.torch {
            EquipKind::Torch
        } else if item.weapon.is_some() {
            EquipKind::Weapon
        } else if item.shield {
            EquipKind::Shield
        } else {
            continue;
        };
        *slot = Some(Equipped {
            model: item.model,
            kind,
            weapon: item.weapon,
            shield_record: item.shield_record,
            flags: r.flags,
        });
    }
    out
}

/// The motion a placed character starts in: the saved one when the
/// placement has it, an attack motion (5…12) reset to the Defence stance
/// 3, else 0x13, or 0x19 when the record's profession word has `0x1000`
/// (bit 4 of its race byte).
pub fn placed_motion(saved: Option<i32>, record: &CharacterRecord) -> i32 {
    match saved {
        Some(m) if (5..13).contains(&m) => 3,
        Some(m) => m,
        None if record.profession() & 0x1000 != 0 => 0x19,
        None => 0x13,
    }
}

/// The motion the player's spawn sets: 3 in mode 1, else 0x13.
pub fn player_motion(mode: i32) -> i32 {
    if mode == 1 { 3 } else { 0x13 }
}

/// The record's hand flag that turns a weapon-section item to its flipped
/// side: the equip pass sets the item body's flip byte and builds its
/// weapon record from the side six bytes on (the same byte a torch's
/// burning state uses). Read per item, for weapon-section items only.
pub const HAND_FLIPPED: u32 = 0x0200_0000;
/// The record's hand flag that sets the character's half-sword hold (the
/// weapon handles heavier and less balanced, `exa_formats::weapon_record`):
/// every weapon-section item the equip pass takes writes it, so the last
/// one's flag wins.
pub const HAND_HALF_SWORD: u32 = 0x0100_0000;

/// How the attach pass hangs an item on the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attach {
    /// The main hand's weapon.
    Weapon,
    /// A second weapon in the off hand.
    SecondWeapon,
    /// A shield on the off arm.
    Shield,
    /// The off hand's torch (the item's torch section); its meshes are
    /// marked held.
    Torch,
}

/// One item the spawn attaches, and the record hand it came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachPlan {
    pub hand: usize,
    pub kind: Attach,
    /// The item starts on its flipped side: a weapon-section item whose
    /// hand has [`HAND_FLIPPED`] (a shield or the off-hand torch path never
    /// reads the flag).
    pub flipped: bool,
}

/// Whether the equip pass turns `item` to its flipped side: a
/// weapon-section item whose hand has [`HAND_FLIPPED`].
pub fn item_flipped(item: &Equipped) -> bool {
    item.kind == EquipKind::Weapon && item.flags & HAND_FLIPPED != 0
}

/// The attach pass over the equip pass's result ([`equip_hands`]).
///
/// The equip pass keeps three off-hand holders: a weapon in hand 1, a
/// shield from either hand (the shield goes on the off arm even from hand
/// 0, and hand 1's overwrites hand 0's) and hand 1's torch. The attach pass
/// hangs the main hand's weapon first, then one off-hand item: the shield
/// when there is one, else the torch, else the second weapon. A shield in
/// hand 0 with a weapon in hand 1 attaches the shield only.
pub fn attach_plan(hands: &[Option<Equipped>; 2]) -> Vec<AttachPlan> {
    let mut out = Vec::new();
    if let Some(main) = &hands[0]
        && main.kind == EquipKind::Weapon
    {
        out.push(AttachPlan {
            hand: 0,
            kind: Attach::Weapon,
            flipped: item_flipped(main),
        });
    }
    let shield = (0..2).rev().find(|&h| {
        hands[h]
            .as_ref()
            .is_some_and(|i| i.kind == EquipKind::Shield)
    });
    let off = match (shield, &hands[1]) {
        (Some(h), _) => Some((h, Attach::Shield)),
        (None, Some(i)) if i.kind == EquipKind::Torch => Some((1, Attach::Torch)),
        (None, Some(i)) if i.kind == EquipKind::Weapon => Some((1, Attach::SecondWeapon)),
        _ => None,
    };
    if let Some((hand, kind)) = off {
        out.push(AttachPlan {
            hand,
            kind,
            flipped: hands[hand].as_ref().is_some_and(item_flipped),
        });
    }
    out
}

/// Whether the main hand's weapon starts on its flipped side (its own
/// hand's [`HAND_FLIPPED`]); `None` without a main-hand weapon.
pub fn holds_flipped(hands: &[Option<Equipped>; 2]) -> Option<bool> {
    hands[0]
        .as_ref()
        .filter(|i| i.kind == EquipKind::Weapon)
        .map(item_flipped)
}

/// The character's half-sword hold: the [`HAND_HALF_SWORD`] flag of the
/// last weapon-section item the equip pass took (hand 1's over hand 0's).
/// `None` when no weapon was taken (the pass leaves the hold as it was).
pub fn half_sword(hands: &[Option<Equipped>; 2]) -> Option<bool> {
    hands
        .iter()
        .rev()
        .flatten()
        .find(|i| i.kind == EquipKind::Weapon)
        .map(|i| i.flags & HAND_HALF_SWORD != 0)
}

/// The corpse's motion: the Defence stance 3.
pub const CORPSE_MOTION: i32 = 3;
/// The corpse's condition word: dead.
pub const CORPSE_CONDITION: u32 = 0xA00;

/// The body flag a spawn sets on the model it built.
pub const SPAWNED: u32 = 0x1000_0000;
/// The condition-flags word a lying spawn writes.
pub const LYING_FLAGS: u32 = 0x1000;
/// The stance a spawn resets to before it sets the record's own (none).
pub const NO_STANCE: i32 = -1;

/// What a spawn does with the body's pose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseSource {
    /// No saved pose: the root stands on the placement's ground.
    Ground,
    /// The saved bone tree ([`crate::runtime::Body::restore_tree_pose`]).
    Tree,
    /// The saved flat bone frames
    /// ([`crate::runtime::Body::restore_flat_pose`]).
    Flat,
}

/// A placed character's spawn, as the placement asks for it: the motion it
/// starts in, whether it lies, which saved pose wins, and the root height a
/// ground placement gives. The original also equips both hands, attaches
/// the held items, builds the armour layers, applies the record's powers,
/// copies its saved pools, gives it its reach and registers it with the
/// scene; those are the steps, in this order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedSpawn {
    /// The motion index the body starts in ([`placed_motion`]).
    pub motion: i32,
    /// The placement's saved attack motion was reset: the caller writes the
    /// new motion back into the placement.
    pub reset_saved_motion: bool,
    /// The pair the caller writes back into the placement then (the
    /// placement's two saved words): the Defence stance and a zero phase.
    pub saved_motion_write: Option<(i32, f32)>,
    pub lying: bool,
    /// The condition-flags word (`0x1000` when lying, else 0).
    pub flags: u32,
    pub pose: PoseSource,
    /// The root's height above the ground, cm, for a ground placement.
    pub root_y: f32,
    /// Every spawn's body-flag word: [`SPAWNED`].
    pub model_flags: u32,
    /// The stance every spawn resets first.
    pub stance: i32,
}

/// A placed character's spawn rules. A saved pose wins over the root
/// placement: the body is placed by its saved bones and its root height is
/// left at the body's standing height, not the placement's. The lying flag
/// (the placement's bit 1) sets the condition flags and, without a saved
/// pose, places the root at the standing height less five centimetres.
pub fn placed_spawn(
    record: &CharacterRecord,
    saved_motion: Option<i32>,
    lying: bool,
    pose: PoseSource,
    placement_y: f32,
    standing: f32,
) -> PlacedSpawn {
    let motion = placed_motion(saved_motion, record);
    let reset = matches!(saved_motion, Some(m) if (5..13).contains(&m));
    PlacedSpawn {
        motion,
        reset_saved_motion: reset,
        saved_motion_write: reset.then_some((3, 0.0)),
        lying,
        flags: if lying { LYING_FLAGS } else { 0 },
        pose,
        root_y: placed_root_height(placement_y, standing, lying),
        model_flags: SPAWNED,
        stance: NO_STANCE,
    }
}

/// The player's spawn: 3 in mode 1 (and the mode's stance byte set), else
/// 0x13; mode 2 starts it in the lying flags; the race's own equip flag
/// decides whether the record's hands are used. Without a frame the root
/// stands on the ground at the standing height less 5 cm, the body's
/// "on ground" frame being a zero translation and one scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSpawn {
    pub motion: i32,
    pub mode_one: bool,
    pub lying: bool,
    pub flags: u32,
    pub model_flags: u32,
    /// The root height in cm: the frame's height plus the standing pelvis
    /// less 6 cm, or the standing pelvis less 5 cm without a frame.
    pub root_y: f32,
    /// Whether the record's hands are equipped (the race's equip flag).
    pub equip: bool,
    pub stance: i32,
}

/// The player's spawn rules ([`player_spawn`]): `frame_y` is the optional
/// placement frame's height.
pub fn player_spawn(
    mode: i32,
    frame_y: Option<f32>,
    standing: f32,
    race_equips: bool,
) -> PlayerSpawn {
    PlayerSpawn {
        motion: player_motion(mode),
        mode_one: mode == 1,
        lying: mode == 2,
        flags: if mode == 2 { LYING_FLAGS } else { 0 },
        model_flags: SPAWNED,
        root_y: player_root_height(standing, frame_y),
        equip: race_equips,
        stance: NO_STANCE,
    }
}

/// The corpse's spawn: the Defence stance, the dead condition word, the
/// lying byte and "down" float cleared, and a saved pose when the placement
/// has one. It does not move the root: the placement's position is already
/// the body's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorpseSpawn {
    pub motion: i32,
    pub condition: u32,
    pub pose: PoseSource,
    pub model_flags: u32,
    /// The spawn clears the body's limp byte (always true: a corpse spawn
    /// writes it to zero).
    pub limp: bool,
    /// The "down" float it writes: zero.
    pub down: f32,
}

/// The corpse's spawn rules ([`corpse_spawn`]).
pub fn corpse_spawn(pose: PoseSource) -> CorpseSpawn {
    CorpseSpawn {
        motion: CORPSE_MOTION,
        condition: CORPSE_CONDITION,
        pose,
        model_flags: SPAWNED,
        limp: true,
        down: 0.0,
    }
}

/// The root height a placed spawn without a saved pose places a body at:
/// the placement's height plus the standing pelvis, less 8 cm, or, for a
/// lying placement (its flag bit 1), the standing pelvis less 5 cm. A
/// saved pose places the body itself and takes its root height from it.
pub fn placed_root_height(placement_y: f32, standing: f32, lying: bool) -> f32 {
    if lying {
        standing - 5.0
    } else {
        (placement_y + standing) - 8.0
    }
}

/// The player's spawn height: the standing pelvis less 5 cm, or, with a
/// placement frame, the frame's height plus the pelvis less 6 cm.
pub fn player_root_height(standing: f32, frame_y: Option<f32>) -> f32 {
    match frame_y {
        Some(y) => (y + standing) - 6.0,
        None => standing - 5.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(flags: u32) -> Weapon {
        Weapon {
            flags,
            seeds: [0; 2],
            quality: 0,
            damage: 0,
            dirt: 0,
            materials: [(0, 0, 0, 0); 4],
            selection: [0; 10],
            stats: exa_formats::weapon_stats::StatBytes::default(),
        }
    }

    fn entry(name: &str, weapon: Option<Weapon>, shield: bool) -> HandEntry {
        HandEntry {
            model: name.into(),
            weapon,
            shield,
            shield_record: None,
            torch: false,
        }
    }

    fn item(id: u32, model: &str) -> exa_formats::objdb::Item {
        exa_formats::objdb::Item {
            id,
            name: "Sword".into(),
            parent: 0,
            model: model.into(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; exa_formats::objdb::SLOTS],
            slot_words: [0; exa_formats::objdb::SLOTS],
            occupancy: [0; exa_formats::objdb::OCCUPANCY_BONES],
            weapon: Some(weapon(0x11)),
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
            volume: None,
            rank_word: 0,
            armour_type: 0,
            container: None,
            random_table: None,
            has_body: false,
            has_weapon: false,
            has_shield: false,
        }
    }

    #[test]
    fn an_item_model_is_a_factory_or_a_library_entry() {
        let it = item(1, "@sword_a");
        let plan = item_model_plan(&it, 2, true).unwrap();
        assert_eq!(plan.source, ModelSource::Factory("sword_a".into()));
        assert_eq!(plan.kind, EquipKind::Weapon);
        assert_eq!(plan.owner, 1);
        // No body section: the skin side is left alone.
        assert_eq!(plan.skin_side, None);
        let mut it = item(2, "mace_a");
        it.burden = Some(exa_formats::objdb::Burden::default());
        assert_eq!(
            item_model_plan(&it, 5, true).unwrap().source,
            ModelSource::Library("mace_a".into())
        );
        assert_eq!(
            item_model_plan(&it, 5, true).unwrap().skin_side,
            Some(0x400)
        );
        assert_eq!(
            item_model_plan(&it, 5, false).unwrap().skin_side,
            Some(0x200)
        );
        assert_eq!(
            item_model_plan(&it, 2, true).unwrap().skin_side,
            Some(0x200)
        );
        // Neither weapon nor shield: no held model; nor without a model.
        it.weapon = None;
        assert!(item_model_plan(&it, 2, true).is_none());
        assert!(item_model_plan(&item(3, ""), 2, true).is_none());
    }

    #[test]
    fn run_time_materials_start_with_an_ampersand() {
        assert!(is_runtime_material("&doublet_ba1"));
        assert!(is_runtime_material("&short"));
        assert!(!is_runtime_material("harhps1"));
        let (mut flags, mut validity) = (0u32, 9u32);
        assert!(mark_runtime_material("&short", &mut flags, &mut validity));
        assert_eq!((flags, validity), (RUNTIME_MATERIAL_FLAG, 0));
        assert!(!mark_runtime_material(
            "skinhuman",
            &mut flags,
            &mut validity
        ));
        assert_eq!((flags, validity), (RUNTIME_MATERIAL_FLAG, 0));
        let (mut flags, mut value) = (0u32, 5u32);
        assert!(clear_wildcard_material("*body", &mut flags, &mut value));
        assert_eq!((flags, value), (0x10, 0));
        assert!(!clear_wildcard_material(
            "skinhuman",
            &mut flags,
            &mut value
        ));
    }

    /// The creator races' roles: a hostile role by force, or a hireling's
    /// role when the character holds none.
    #[test]
    fn the_creator_races_take_their_roles() {
        for race in [5u32, 6, 7, 8, 9] {
            assert_eq!(
                creator_role(race << 8),
                Some((0x0B, RoleSet::Force)),
                "{race}"
            );
        }
        // The female bit does not change the race byte's role.
        assert_eq!(creator_role((5 << 8) | 1), Some((0x0B, RoleSet::Force)));
        assert_eq!(creator_role(0x1100), Some((0x2D, RoleSet::Try)));
        assert_eq!(creator_role(0x1200), Some((0x2E, RoleSet::Try)));
        assert_eq!(creator_role(0), None);
        assert_eq!(creator_role(4 << 8), None);
        assert_eq!(creator_role(0x0501), Some((0x0B, RoleSet::Force)));
    }

    #[test]
    fn a_weapon_goes_to_the_main_hand_and_a_shield_to_the_off() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0);
        record.hands[1] = ItemRef::new(2, 0);
        let hands = equip_hands(&record, |_, r| {
            Some(match r.id {
                1 => entry("sword", Some(weapon(0x11)), false),
                _ => entry("shield", None, true),
            })
        });
        assert_eq!(hands[0].as_ref().unwrap().kind, EquipKind::Weapon);
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Shield);
        // An item with both sections is a weapon.
        let both = equip_hands(&record, |_, _| {
            Some(entry("buckler", Some(weapon(0x11)), true))
        });
        assert_eq!(both[1].as_ref().unwrap().kind, EquipKind::Weapon);
    }

    #[test]
    fn the_off_hands_torch_takes_its_own_path() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0);
        record.hands[1] = ItemRef::new(2, 0x100);
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 {
                HandEntry {
                    torch: true,
                    ..entry("torch", None, false)
                }
            } else {
                HandEntry {
                    torch: true,
                    ..entry("torch", Some(weapon(0x13)), false)
                }
            })
        });
        // The main hand's torch section is ignored; with no weapon or
        // shield it is not equipped.
        assert!(hands[0].is_none());
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Torch);
        assert_eq!(hands[1].as_ref().unwrap().flags, 0x100);
        // An item with neither section is skipped, and the off hand is
        // still visited.
        record.hands[1] = ItemRef::new(3, 0);
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 {
                entry("cup", None, false)
            } else {
                entry("shield", None, true)
            })
        });
        assert!(hands[0].is_none());
        assert_eq!(hands[1].as_ref().unwrap().kind, EquipKind::Shield);
    }

    #[test]
    fn an_empty_hand_leaves_no_entry() {
        let record = CharacterRecord::default();
        assert_eq!(equip_hands(&record, |_, _| None), [None, None]);
    }

    /// The attach pass: the main hand's weapon, then one off-hand item (a
    /// shield first, else the torch, else a second weapon); each weapon's
    /// own `0x2000000` flips it, the last weapon's `0x1000000` sets the
    /// half-sword hold.
    #[test]
    fn the_attach_plan_takes_the_equipped_hands() {
        let mut record = CharacterRecord::default();
        record.hands[0] = ItemRef::new(1, 0x0300_0000);
        record.hands[1] = ItemRef::new(2, 0x0200_0000);
        let sword = || entry("sword", Some(weapon(0x11)), false);
        let torch = || HandEntry {
            torch: true,
            ..entry("torch", None, false)
        };
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 { sword() } else { torch() })
        });
        assert_eq!(
            attach_plan(&hands),
            [
                AttachPlan {
                    hand: 0,
                    kind: Attach::Weapon,
                    flipped: true,
                },
                // The torch path does not read the flip flag.
                AttachPlan {
                    hand: 1,
                    kind: Attach::Torch,
                    flipped: false,
                },
            ]
        );
        assert_eq!(holds_flipped(&hands), Some(true));
        // Only the sword is a weapon: its 0x1000000 sets the hold.
        assert_eq!(half_sword(&hands), Some(true));
        // A second weapon: its own kind and its own flip; the last weapon
        // (hand 1, without 0x1000000) decides the hold.
        let hands = equip_hands(&record, |_, _| Some(sword()));
        assert_eq!(
            attach_plan(&hands)[1],
            AttachPlan {
                hand: 1,
                kind: Attach::SecondWeapon,
                flipped: true,
            }
        );
        assert_eq!(holds_flipped(&hands), Some(true));
        assert_eq!(half_sword(&hands), Some(false));
        record.hands[0] = ItemRef::new(1, 0x0100_0000);
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 { sword() } else { torch() })
        });
        assert_eq!(holds_flipped(&hands), Some(false));
        assert_eq!(half_sword(&hands), Some(true));
        // A shield in hand 0 goes on the off arm and wins over hand 1's
        // weapon; nothing hangs in the main hand, and no weapon is flipped.
        let shield = || entry("shield", None, true);
        let hands = equip_hands(&record, |hand, _| {
            Some(if hand == 0 { shield() } else { sword() })
        });
        assert_eq!(
            attach_plan(&hands),
            [AttachPlan {
                hand: 0,
                kind: Attach::Shield,
                flipped: false,
            }]
        );
        assert_eq!(holds_flipped(&hands), None);
        assert!(attach_plan(&[None, None]).is_empty());
        assert_eq!(holds_flipped(&[None, None]), None);
        assert_eq!(half_sword(&[None, None]), None);
    }

    #[test]
    fn motions_are_the_spawns() {
        let mut record = CharacterRecord::default();
        assert_eq!(placed_motion(None, &record), 0x13);
        // Bit 0x1000 of the profession word is the race byte's bit 4.
        record.race = 0x10;
        assert_eq!(placed_motion(None, &record), 0x19);
        assert_eq!(placed_motion(Some(7), &record), 3);
        assert_eq!(placed_motion(Some(4), &record), 4);
        assert_eq!(placed_motion(Some(13), &record), 13);
        assert_eq!(placed_motion(Some(1), &record), 1);
        assert_eq!((player_motion(1), player_motion(0)), (3, 0x13));
        assert_eq!(CORPSE_MOTION, 3);
        assert_eq!(CORPSE_CONDITION, 0xA00);
    }

    #[test]
    fn the_spawn_heights_are_the_originals() {
        assert_eq!(placed_root_height(0.0, 101.5, false), 93.5);
        assert_eq!(placed_root_height(2.0, 101.5, false), 95.5);
        assert_eq!(placed_root_height(2.0, 101.5, true), 96.5);
        assert_eq!(player_root_height(101.5, None), 96.5);
        assert_eq!(player_root_height(101.5, Some(2.0)), 97.5);
    }

    /// A placed spawn: the motion from the record and the saved value, the
    /// attack reset written back, the lying flags, and the ground height.
    #[test]
    fn a_placed_spawn_takes_its_motion_pose_and_height() {
        let record = CharacterRecord::default();
        let spawn = placed_spawn(&record, None, false, PoseSource::Ground, 2.0, 101.5);
        assert_eq!(spawn.motion, 0x13);
        assert!(!spawn.reset_saved_motion);
        assert_eq!(spawn.saved_motion_write, None);
        assert!(!spawn.lying);
        assert_eq!(spawn.flags, 0);
        assert_eq!(spawn.root_y, 95.5);
        assert_eq!(spawn.model_flags, SPAWNED);
        assert_eq!(spawn.stance, NO_STANCE);
        let attack = placed_spawn(&record, Some(7), false, PoseSource::Tree, 2.0, 101.5);
        assert_eq!(attack.motion, 3);
        assert!(attack.reset_saved_motion);
        // The reset writes the Defence stance and a zero phase back.
        assert_eq!(attack.saved_motion_write, Some((3, 0.0)));
        assert_eq!(attack.pose, PoseSource::Tree);
        let lying = placed_spawn(&record, Some(3), true, PoseSource::Flat, 2.0, 101.5);
        assert_eq!((lying.motion, lying.flags), (3, 0x1000));
        assert_eq!(lying.root_y, 96.5);
        // A profession bit `0x1000` starts it in 0x19.
        let golem = CharacterRecord {
            race: 0x10,
            ..Default::default()
        };
        assert_eq!(
            placed_spawn(&golem, None, false, PoseSource::Ground, 0.0, 100.0).motion,
            0x19
        );
    }

    /// The player's spawn: mode 1 is the Defence stance with the mode's
    /// stance byte, mode 2 the lying flags, the frame's height or the
    /// ground.
    #[test]
    fn the_players_spawn_follows_the_mode_and_the_frame() {
        let a = player_spawn(0, None, 101.5, true);
        assert_eq!((a.motion, a.lying, a.flags), (0x13, false, 0));
        assert_eq!((a.root_y, a.equip), (96.5, true));
        let b = player_spawn(1, None, 101.5, false);
        assert_eq!((b.motion, b.mode_one, b.equip), (3, true, false));
        let c = player_spawn(2, Some(2.0), 101.5, false);
        assert_eq!((c.motion, c.lying, c.flags), (0x13, true, 0x1000));
        assert_eq!(c.root_y, 97.5);
        assert_eq!(c.stance, NO_STANCE);
        assert_eq!(c.model_flags, SPAWNED);
    }

    #[test]
    fn a_corpse_spawn_is_dead_and_limp() {
        let c = corpse_spawn(PoseSource::Flat);
        assert_eq!((c.motion, c.condition), (3, 0xA00));
        assert_eq!(c.pose, PoseSource::Flat);
        assert!(c.limp);
        assert_eq!(c.down, 0.0);
        assert_eq!(c.model_flags, SPAWNED);
    }
}
