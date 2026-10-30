//! The arena's opponent spawn plans: the rules that configure a generated
//! fighter before the scene creates its body.
//!
//! The scene and character generator are engine calls; these are the plain
//! values they are given: the beast's tier items and ability, a generic
//! opponent's level and outfit level, and a hire-list opponent's eight-byte
//! outfit profile with its forced company bytes and weapon variant.
//!
//! Evidence: the arena's beast and opponent spawn routines, read from
//! `Exanima.exe` 0.9.5.

/// The experience the beast's record spends on the profession grid.
pub const BEAST_ABILITY: u32 = 50_000;
/// A generated fighter's full health.
pub const SPAWN_HEALTH: f32 = 100.0;
/// The damage seed a generated fighter starts with.
pub const SPAWN_DAMAGE: f32 = -100.0;
/// The profession word a beast's record starts with (race byte 3, the
/// Ogre).
pub const BEAST_PROFESSION: u32 = 0x300;
/// The hostile role word every spawned opponent receives.
pub const HOSTILE_ROLE: u32 = 0x12;
/// The opponent team.
pub const OPPONENT_TEAM: u32 = 2;

/// A placed object the hub query can see.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedItem {
    /// Its position on the level, x and z in centimetres.
    pub position: [f32; 2],
    /// Its type bits; `0x4000` in the `0x7000` field is a weapon object.
    pub type_bits: u32,
    /// Its object-database kind; `0x40000` is the weapon kind the hub
    /// clears.
    pub template_kind: u32,
}

/// The point the physician's hub spot is cleared around, x and z.
pub const PHYSICIAN_ITEM_POINT: [f32; 2] = [-2100.0, 0.0];

/// The shopkeeper's hub spot (cm) and the yaw it faces, degrees.
pub const SHOPKEEPER_SPOT: [f32; 3] = [-320.0, 0.0, -110.0];
pub const SHOPKEEPER_YAW_DEGREES: f32 = 15.0;
/// The merchant's hub spot and yaw.
pub const MERCHANT_SPOT: [f32; 3] = [440.0, 0.0, 10.0];
pub const MERCHANT_YAW_DEGREES: f32 = -40.0;
/// The physician's hub spot and yaw.
pub const PHYSICIAN_SPOT: [f32; 3] = [-2700.0, 0.0, 1250.0];
pub const PHYSICIAN_YAW_DEGREES: f32 = 140.0;

/// The shopkeeper's scene name, sixteen bytes.
pub const SHOPKEEPER_NAME: [u8; 16] = *b"arenashpkp\0\0\0\0\0\0";
/// One degree in radians, the single the placements multiply by.
pub const DEGREE: f32 = f32::from_bits(0x3c8e_fa35);
/// The two role words a placed specialist receives, in order.
pub const SPECIALIST_ROLES: [u32; 2] = [0x11, 0x21];
/// The item handle a placed specialist's record stores (0x2500).
pub const SPECIALIST_ITEM_HANDLE: u32 = 0x2500;

/// A hub body's placement as its record receives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HubBodyPlan {
    pub position: [f32; 3],
    /// The yaw the record stores, radians.
    pub yaw: f32,
    pub health: f32,
    pub damage: f32,
    /// The third condition word, −100 for a specialist.
    pub third: f32,
    pub roles: [u32; 2],
    /// The record's condition word is copied to the second one.
    pub condition_copied: bool,
    /// The record's item handle word.
    pub item_handle: u32,
}

/// Convert a placement's degrees to the record's radians, in single
/// precision.
pub fn yaw_radians(degrees: f32) -> f32 {
    degrees * DEGREE
}

/// A specialist's body plan: the health words 100, −100, −100, the two role
/// words, the condition word copied to the second, and the item handle
/// 0x2500.
pub fn specialist_plan(position: [f32; 3], yaw_degrees: f32) -> HubBodyPlan {
    HubBodyPlan {
        position,
        yaw: yaw_radians(yaw_degrees),
        health: SPAWN_HEALTH,
        damage: SPAWN_DAMAGE,
        third: SPAWN_DAMAGE,
        roles: SPECIALIST_ROLES,
        condition_copied: true,
        item_handle: SPECIALIST_ITEM_HANDLE,
    }
}

/// Where a specialist is placed at the hub: the merchant at its spot and
/// −40°, the physician at its spot and 140°. Any other profession is not
/// placed.
pub fn specialist_spot(profession: crate::career::Profession) -> Option<HubBodyPlan> {
    use crate::career::Profession;
    match profession {
        Profession::Merchant => Some(specialist_plan(MERCHANT_SPOT, MERCHANT_YAW_DEGREES)),
        Profession::Physician => Some(specialist_plan(PHYSICIAN_SPOT, PHYSICIAN_YAW_DEGREES)),
        Profession::Combatant | Profession::Trainer => None,
    }
}

/// The shopkeeper's body plan: −320, 0, −110 and 15°, the same words as a
/// specialist's except that its condition word is not copied.
pub fn shopkeeper_plan() -> HubBodyPlan {
    HubBodyPlan {
        condition_copied: false,
        ..specialist_plan(SHOPKEEPER_SPOT, SHOPKEEPER_YAW_DEGREES)
    }
}

/// Find the first placed item that is a weapon object of the weapon kind
/// within ten centimetres of `point` on both axes.
///
/// The original scans the level's groups in order, each group's nodes in
/// order, and returns the first node that passes. The ten-centimetre test
/// uses the absolute difference (`fabs`), so it is symmetric.
#[must_use]
pub fn find_nearby_weapon(items: &[PlacedItem], point: [f32; 2]) -> Option<usize> {
    items.iter().position(|item| {
        item.type_bits & 0x7000 == 0x4000
            && item.template_kind == 0x40000
            && (item.position[0] - point[0]).abs() < 10.0
            && (item.position[1] - point[1]).abs() < 10.0
    })
}

/// The role a spawned manager receives.
pub const MANAGER_ROLE: u32 = 0x11;
/// The player's team.
pub const PLAYER_TEAM: u32 = 1;
/// The lift a spawn spot takes above the floor.
pub const SPAWN_LIFT: f32 = 5.0;
/// How far above the profile height the floor probe starts.
pub const FLOOR_PROBE_RISE: f32 = 1000.0;
/// How far down the floor probe reaches.
pub const FLOOR_PROBE_REACH: f32 = 3000.0;

/// Settle a spawn position on the floor.
///
/// The original probes straight down from [`FLOOR_PROBE_RISE`] above the
/// given point; `probe_start` is that start point's x and y. When the probe
/// finds the floor, the position takes the probe's x and y (its z is kept)
/// and is lifted by [`SPAWN_LIFT`]; when it finds nothing, the position is
/// left as it was, without the lift.
#[must_use]
pub fn settle_on_floor(position: [f32; 3], floor_hit: Option<[f32; 2]>) -> [f32; 3] {
    match floor_hit {
        Some([x, y]) => [x, y + SPAWN_LIFT, position[2]],
        None => position,
    }
}

/// A manager body's configured values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagerPlan {
    pub health: f32,
    pub damage: f32,
    pub role: u32,
    pub team: u32,
    /// The profile position after the floor probe, see [`settle_on_floor`].
    pub position: [f32; 3],
}

/// The manager's body plan from its profile position and the floor probe's
/// hit.
#[must_use]
pub fn manager_plan(position: [f32; 3], floor_hit: Option<[f32; 2]>) -> ManagerPlan {
    ManagerPlan {
        health: SPAWN_HEALTH,
        damage: SPAWN_HEALTH,
        role: MANAGER_ROLE,
        team: PLAYER_TEAM,
        position: settle_on_floor(position, floor_hit),
    }
}

/// Whether a participant's role marks it as the manager.
pub fn is_manager(role: u32) -> bool {
    role & 0x1000_0000 != 0
}

/// The participant the round's player controls: the indexed fighter when
/// its role marks the manager, otherwise no participant (the caller then
/// spawns a manager body).
#[must_use]
pub fn player_participant(roles: &[u32], index: usize) -> Option<usize> {
    let role = *roles.get(index)?;
    is_manager(role).then_some(index)
}

/// A beast's tier items and stats.
#[derive(Clone, Debug, PartialEq)]
pub struct BeastPlan {
    /// The item in the record's main hand. Tiers three to five replace the
    /// base item with their own.
    pub body: u32,
    /// The tier's worn items, added in this order.
    pub equipment: Vec<u32>,
    /// The profession word the record starts with: race byte 3, the Ogre.
    pub profession: u32,
    /// The experience spent on the profession grid.
    pub ability: u32,
    /// The record's combat skill.
    pub health: f32,
    /// The first personality value.
    pub damage: f32,
    /// The record's voice pitch, `0.9 + 0.17 · smooth`.
    pub voice_pitch: f32,
    pub team: u32,
    pub role: u32,
}

/// The beast at `tier` (1 through 5), from the smooth look draw.
///
/// Tiers three to five swap the base hand item for the tier's own and add
/// four worn items. Every tier spends the same 50000 experience. Tiers one
/// and two, and any unknown tier, keep the base item only. The pitch is
/// computed in double precision and narrowed once.
#[must_use]
pub fn beast_plan(tier: u32, smooth: f32) -> BeastPlan {
    let (body, equipment): (u32, &[u32]) = match tier {
        3 => (0x1f6, &[0x1fc, 0x1f9, 0x1fb, 0x1fa]),
        4 => (0x207, &[0x210, 0x20e, 0x20f, 0x213]),
        5 => (0x21e, &[0x214, 0x1f7, 0x216, 0x215]),
        _ => (0x44, &[]),
    };
    BeastPlan {
        body,
        equipment: equipment.to_vec(),
        profession: BEAST_PROFESSION,
        ability: BEAST_ABILITY,
        health: SPAWN_HEALTH,
        damage: SPAWN_DAMAGE,
        voice_pitch: (f64::from(smooth) * 0.17 + 0.9) as f32,
        team: OPPONENT_TEAM,
        role: HOSTILE_ROLE,
    }
}

/// A generic opponent's configured values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenericPlan {
    /// The character level.
    pub level: i32,
    /// The outfit generator's level.
    pub outfit_level: i32,
    /// The outfit warmth, `level · 0.05` computed in double precision.
    pub warmth: f32,
    pub health: f32,
    pub damage: f32,
}

/// A generic opponent at `level`, dressed by `outfit_level`.
#[must_use]
pub fn generic_plan(level: i32, outfit_level: i32) -> GenericPlan {
    GenericPlan {
        level,
        outfit_level,
        warmth: (f64::from(level) * 0.05) as f32,
        health: SPAWN_HEALTH,
        damage: SPAWN_DAMAGE,
    }
}

/// The eight bytes of a hire-list opponent's outfit profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutfitProfile(pub [u8; 8]);

impl OutfitProfile {
    /// The profile's flags byte.
    pub const fn flags(self) -> u8 {
        self.0[0]
    }
    /// The profile's level byte.
    pub const fn level(self) -> u8 {
        self.0[1]
    }
}

/// The three appearance bytes the company table gives a forced opponent
/// outfit, indexed by company id 1 through 19.
pub const COMPANY_OUTFITS: [[u8; 3]; 19] = [
    [1, 7, 1],
    [66, 140, 1],
    [36, 1, 1],
    [203, 114, 1],
    [0, 203, 0],
    [96, 227, 0],
    [35, 7, 0],
    [155, 20, 0],
    [7, 84, 0],
    [125, 181, 0],
    [219, 253, 0],
    [37, 213, 0],
    [97, 20, 0],
    [5, 214, 0],
    [4, 92, 0],
    [243, 211, 0],
    [62, 125, 0],
    [181, 67, 0],
    [114, 140, 0],
];

/// A hire-list opponent's configured values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HirePlan {
    /// The character level the stats generator receives (always six here).
    pub stat_level: i32,
    /// The outfit profile's eight bytes.
    pub outfit: OutfitProfile,
    /// A weapon variant is drawn from the current event's row.
    pub draw_weapon: bool,
    /// The zero-based weapon row to draw from, when drawn.
    pub weapon_row: Option<u32>,
}

/// A hire-list opponent for `team`, optionally forced to company `outfit`.
///
/// The profile starts `[3, team, 100, 10, 10, 0, 0, 0]`. A forced outfit
/// (1 through 19) sets flags to `0x0b` and copies that company's three
/// appearance bytes. When the event's weapon field (a signed count of the
/// one-based row) is positive, flag 4 is added and the variant is drawn from
/// the zero-based row one below it; see [`pick_weapon_slot`].
#[must_use]
pub fn hire_plan(team: u8, forced_outfit: i32, event_weapon: i32) -> HirePlan {
    let mut outfit = [3, team, 100, 10, 10, 0, 0, 0];
    if let Some(index) = usize::try_from(forced_outfit)
        .ok()
        .filter(|index| (1..=19).contains(index))
    {
        outfit[0] = 0x0b;
        outfit[5..8].copy_from_slice(&COMPANY_OUTFITS[index - 1]);
    }
    if event_weapon <= 0 {
        return HirePlan {
            stat_level: 6,
            outfit: OutfitProfile(outfit),
            draw_weapon: false,
            weapon_row: None,
        };
    }
    outfit[0] |= 4;
    HirePlan {
        stat_level: 6,
        outfit: OutfitProfile(outfit),
        draw_weapon: true,
        weapon_row: Some(event_weapon as u32 - 1),
    }
}

/// The forced outfit a fresh tournament side wears: the rival company the
/// week's field names for its team.
///
/// Teams 0 and 1 (the player's) wear none; team `t` from 2 on wears the
/// field's `t − 2`th company id (1 through 19). A team past the sixteen
/// field slots wears none [guess: the original reads past the field there,
/// which no bracket reaches].
#[must_use]
pub fn rival_company(rivals: &[i32; crate::career::RIVAL_FIELD], team: i32) -> i32 {
    if team <= 1 {
        return 0;
    }
    usize::try_from(team - 2)
        .ok()
        .and_then(|slot| rivals.get(slot).copied())
        .unwrap_or(0)
}

/// [`rival_company`] for every team id a bracket can hold, indexed by team
/// id, as the fresh match's raise reads it.
#[must_use]
pub fn rival_outfits(rivals: &[i32; crate::career::RIVAL_FIELD]) -> Vec<i32> {
    (0..crate::career::RIVAL_FIELD as i32 + 2)
        .map(|team| rival_company(rivals, team))
        .collect()
}

/// The weapon slot a hire-list opponent draws from an event row's three
/// slots.
///
/// The original counts the slots that are set — the compare is unsigned, so
/// only a positive value counts, and the second and third are counted
/// without checking that the earlier ones were — steps the roll once and
/// takes the slot at `(count · roll) >> 32` as the row stands, empty slots
/// included. With no slot set the first slot (zero) is returned.
#[must_use]
pub fn pick_weapon_slot(slots: [u32; 3], roll: u32) -> u32 {
    let count = u64::from((slots[0] as i32) > 0)
        + u64::from((slots[1] as i32) > 0)
        + u64::from((slots[2] as i32) > 0);
    slots[((count * u64::from(roll)) >> 32) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tournament_team_wears_its_rival_company() {
        let rivals = [15, 17, 4, 1, 13, 9, 5, 14, 8, 6, 11, 2, 18, 3, 19, 10];
        assert_eq!(rival_company(&rivals, 0), 0);
        assert_eq!(rival_company(&rivals, 1), 0);
        assert_eq!(rival_company(&rivals, 2), 15);
        assert_eq!(rival_company(&rivals, 3), 17);
        assert_eq!(rival_company(&rivals, 17), 10);
        assert_eq!(rival_company(&rivals, 18), 0);
        let table = rival_outfits(&rivals);
        assert_eq!(table.len(), 18);
        assert_eq!(&table[..4], &[0, 0, 15, 17]);
        // The rival's company bytes reach the hire profile.
        let plan = hire_plan(3, rival_company(&rivals, 5), 0);
        assert_eq!(plan.outfit.flags(), 0x0b);
        assert_eq!(&plan.outfit.0[5..8], &COMPANY_OUTFITS[0]);
    }

    #[test]
    fn a_beast_takes_its_tier_items_and_pitch() {
        let base = beast_plan(2, 0.0);
        assert_eq!(base.body, 0x44);
        assert!(base.equipment.is_empty());
        assert_eq!(base.profession, 0x300);
        assert_eq!(base.voice_pitch, 0.9);
        assert_eq!(base.ability, 50_000);
        let third = beast_plan(3, 0.5);
        assert_eq!(third.body, 0x1f6);
        assert_eq!(third.equipment, [0x1fc, 0x1f9, 0x1fb, 0x1fa]);
        assert!((third.voice_pitch - 0.985).abs() < 1e-6);
        let fourth = beast_plan(4, 0.0);
        assert_eq!(fourth.body, 0x207);
        assert_eq!(fourth.equipment, [0x210, 0x20e, 0x20f, 0x213]);
        let fifth = beast_plan(5, 0.0);
        assert_eq!(fifth.body, 0x21e);
        assert_eq!(fifth.equipment, [0x214, 0x1f7, 0x216, 0x215]);
        // An unknown tier keeps the base body only.
        assert_eq!(beast_plan(9, 0.0).body, 0x44);
        assert!(beast_plan(1, 0.0).equipment.is_empty());
        assert_eq!(base.team, 2);
        assert_eq!(base.role, 0x12);
    }

    #[test]
    fn a_generic_spawn_uses_the_level_warmth() {
        let plan = generic_plan(3, 1);
        assert_eq!(plan.level, 3);
        assert_eq!(plan.outfit_level, 1);
        assert_eq!(plan.warmth, (3.0f64 * 0.05) as f32);
        assert_eq!(plan.health, 100.0);
        assert_eq!(plan.damage, -100.0);
    }

    #[test]
    fn a_forced_hire_takes_the_company_appearance() {
        let plain = hire_plan(4, 0, 0);
        assert_eq!(plain.outfit.0, [3, 4, 100, 10, 10, 0, 0, 0]);
        assert_eq!(plain.stat_level, 6);
        let forced = hire_plan(4, 2, 0);
        assert_eq!(forced.outfit.0, [0x0b, 4, 100, 10, 10, 66, 140, 1]);
        // Out-of-range companies are ignored.
        assert_eq!(hire_plan(1, 20, 0).outfit.flags(), 3);
        assert_eq!(hire_plan(1, -1, 0).outfit.flags(), 3);
        // An event weapon row adds flag 4 and asks for a draw.
        let armed = hire_plan(2, 3, 5);
        assert_eq!(armed.outfit.flags(), 0x0f);
        assert!(armed.draw_weapon);
        assert_eq!(armed.weapon_row, Some(4));
        assert!(!hire_plan(2, 3, 0).draw_weapon);
        assert!(!hire_plan(2, 3, -4).draw_weapon);
        assert!(hire_plan(2, 3, 0x20).draw_weapon);
    }

    #[test]
    fn a_weapon_slot_is_drawn_among_the_set_ones() {
        assert_eq!(pick_weapon_slot([7, 8, 9], 0), 7);
        assert_eq!(pick_weapon_slot([7, 8, 9], 0x5555_5556), 8);
        assert_eq!(pick_weapon_slot([7, 8, 9], u32::MAX), 9);
        assert_eq!(pick_weapon_slot([7, 8, 0], u32::MAX), 8);
        assert_eq!(pick_weapon_slot([0, 0, 0], u32::MAX), 0);
        // The draw indexes the row as it stands: an interior zero is a slot
        // the count can reach, and a negative slot is not counted at all.
        assert_eq!(pick_weapon_slot([7, 0, 9], u32::MAX), 0);
        assert_eq!(pick_weapon_slot([7, 0, 9], 0), 7);
        assert_eq!(pick_weapon_slot([7, 0x8000_0000, 9], 0), 7);
        assert_eq!(pick_weapon_slot([7, 0x8000_0000, 9], u32::MAX), 0x8000_0000);
    }

    #[test]
    fn the_hub_places_its_shopkeeper_and_specialists() {
        let keeper = shopkeeper_plan();
        assert_eq!(keeper.position, [-320.0, 0.0, -110.0]);
        assert_eq!(keeper.yaw.to_bits(), (15.0f32 * DEGREE).to_bits());
        assert_eq!(keeper.health, 100.0);
        assert_eq!(keeper.damage, -100.0);
        assert_eq!(keeper.third, -100.0);
        assert_eq!(keeper.roles, [0x11, 0x21]);
        assert!(!keeper.condition_copied);
        assert_eq!(keeper.item_handle, 0x2500);
        let merchant = specialist_spot(crate::career::Profession::Merchant).unwrap();
        assert_eq!(merchant.position, [440.0, 0.0, 10.0]);
        assert_eq!(merchant.yaw.to_bits(), (-40.0f32 * DEGREE).to_bits());
        assert!(merchant.condition_copied);
        let physician = specialist_spot(crate::career::Profession::Physician).unwrap();
        assert_eq!(physician.position, [-2700.0, 0.0, 1250.0]);
        assert_eq!(physician.yaw.to_bits(), (140.0f32 * DEGREE).to_bits());
        assert!(specialist_spot(crate::career::Profession::Combatant).is_none());
        assert!(specialist_spot(crate::career::Profession::Trainer).is_none());
        assert_eq!(yaw_radians(180.0).to_bits(), 3.1415927f32.to_bits());
    }

    #[test]
    fn the_hub_weapon_query_takes_the_first_node_within_ten() {
        let items = [
            PlacedItem {
                position: [-2100.0, 0.0],
                type_bits: 0x4000,
                template_kind: 0x40000,
            },
            PlacedItem {
                position: [-2091.0, 9.99],
                type_bits: 0x4000,
                template_kind: 0x40001,
            },
        ];
        assert_eq!(find_nearby_weapon(&items, PHYSICIAN_ITEM_POINT), Some(0));
        let none = [PlacedItem {
            position: [-2089.0, 0.0],
            type_bits: 0x4000,
            template_kind: 0x40000,
        }];
        assert_eq!(find_nearby_weapon(&none, PHYSICIAN_ITEM_POINT), None);
        let wrong = [PlacedItem {
            position: [-2100.0, 0.0],
            type_bits: 0x5000,
            template_kind: 0x40000,
        }];
        assert_eq!(find_nearby_weapon(&wrong, PHYSICIAN_ITEM_POINT), None);
    }

    #[test]
    fn a_manager_plan_lifts_the_profile_and_numbers_team_one() {
        let plan = manager_plan([1.0, 2.0, 3.0], Some([1.5, 2.5]));
        assert_eq!(plan.position, [1.5, 7.5, 3.0]);
        // Without a floor hit the profile position is kept.
        assert_eq!(
            manager_plan([1.0, 2.0, 3.0], None).position,
            [1.0, 2.0, 3.0]
        );
        assert_eq!(plan.health, 100.0);
        assert_eq!(plan.damage, 100.0);
        assert_eq!(plan.role, 0x11);
        assert_eq!(plan.team, 1);
        assert!(is_manager(0x1000_0000));
        assert!(!is_manager(0x11));
        assert_eq!(player_participant(&[2, 0x1000_0000], 1), Some(1));
        assert_eq!(player_participant(&[2, 3], 1), None);
        assert_eq!(player_participant(&[], 0), None);
    }
}
