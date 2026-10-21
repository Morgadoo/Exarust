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

/// The beast's ability word.
pub const BEAST_ABILITY: u32 = 50_000;
/// A generated fighter's full health.
pub const SPAWN_HEALTH: f32 = 100.0;
/// The damage seed a generated fighter starts with.
pub const SPAWN_DAMAGE: f32 = -100.0;
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

/// A manager body's configured values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagerPlan {
    pub health: f32,
    pub damage: f32,
    pub role: u32,
    pub team: u32,
    /// The profile position with the spawn lift added to its y.
    pub position: [f32; 3],
}

/// The manager's body plan from its profile position.
#[must_use]
pub fn manager_plan(position: [f32; 3]) -> ManagerPlan {
    ManagerPlan {
        health: SPAWN_HEALTH,
        damage: SPAWN_HEALTH,
        role: MANAGER_ROLE,
        team: PLAYER_TEAM,
        position: [position[0], position[1] + SPAWN_LIFT, position[2]],
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
    /// The body item and its tier equipment.
    pub items: Vec<u32>,
    pub ability: u32,
    pub health: f32,
    pub damage: f32,
    /// The beast's scale, `0.9 + 0.17 · smooth`.
    pub scale: f32,
    pub team: u32,
    pub role: u32,
}

/// The beast at `tier` (1 through 5), from the smooth look draw.
///
/// Tiers above one add the tier's four equipment items. Every tier gives
/// the same 50000 ability and experience. An unknown tier keeps the base
/// body only.
#[must_use]
pub fn beast_plan(tier: u32, smooth: f32) -> BeastPlan {
    let mut items = vec![0x44];
    match tier {
        3 => items.extend_from_slice(&[0x1f6, 0x1fc, 0x1f9, 0x1fb, 0x1fa]),
        4 => items.extend_from_slice(&[0x207, 0x210, 0x20e, 0x20f, 0x213]),
        5 => items.extend_from_slice(&[0x21e, 0x214, 0x1f7, 0x216, 0x215]),
        _ => {}
    }
    BeastPlan {
        items,
        ability: BEAST_ABILITY,
        health: SPAWN_HEALTH,
        damage: SPAWN_DAMAGE,
        scale: smooth * 0.17 + 0.9,
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
    /// The outfit warmth, `level · 0.05`.
    pub warmth: f32,
    pub health: f32,
    pub damage: f32,
    /// The profession word the body starts with.
    pub profession: u32,
    /// The experience passed to the skill generator.
    pub experience: u32,
}

/// A generic opponent at `level`, dressed by `outfit_level`.
#[must_use]
pub fn generic_plan(level: i32, outfit_level: i32) -> GenericPlan {
    GenericPlan {
        level,
        outfit_level,
        warmth: level as f32 * 0.05,
        health: SPAWN_HEALTH,
        damage: SPAWN_DAMAGE,
        profession: 0x300,
        experience: 0,
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
    /// The event's weapon row index, when drawn.
    pub weapon_row: Option<u32>,
}

/// A hire-list opponent for `team`, optionally forced to company `outfit`.
///
/// The profile starts `[3, team, 100, 10, 10, 0, 0, 0]`. A forced outfit
/// (1 through 19) sets flags to `0x0b` and copies that company's three
/// appearance bytes. A tournament event with a weapon row adds flag 4 and
/// asks for a variant draw.
#[must_use]
pub fn hire_plan(team: u8, forced_outfit: i32, weapon_row: Option<u32>) -> HirePlan {
    let mut outfit = [3, team, 100, 10, 10, 0, 0, 0];
    if let Some(index) = usize::try_from(forced_outfit)
        .ok()
        .filter(|index| (1..=19).contains(index))
    {
        outfit[0] = 0x0b;
        outfit[5..8].copy_from_slice(&COMPANY_OUTFITS[index - 1]);
    }
    let draw_weapon = weapon_row.is_some_and(|row| row > 0);
    if !draw_weapon {
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
        weapon_row,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beast_takes_its_tier_items_and_scale() {
        let base = beast_plan(2, 0.0);
        assert_eq!(base.items, [0x44]);
        assert_eq!(base.scale, 0.9);
        assert_eq!(base.ability, 50_000);
        let third = beast_plan(3, 0.5);
        assert_eq!(third.items, [0x44, 0x1f6, 0x1fc, 0x1f9, 0x1fb, 0x1fa]);
        assert!((third.scale - 0.985).abs() < 1e-6);
        let fourth = beast_plan(4, 0.0);
        assert_eq!(fourth.items[1..], [0x207, 0x210, 0x20e, 0x20f, 0x213]);
        let fifth = beast_plan(5, 0.0);
        assert_eq!(fifth.items[1..], [0x21e, 0x214, 0x1f7, 0x216, 0x215]);
        // An unknown tier keeps the base body only.
        assert_eq!(beast_plan(9, 0.0).items, [0x44]);
        assert_eq!(base.team, 2);
        assert_eq!(base.role, 0x12);
    }

    #[test]
    fn a_generic_spawn_uses_the_level_warmth() {
        let plan = generic_plan(3, 1);
        assert_eq!(plan.level, 3);
        assert_eq!(plan.outfit_level, 1);
        assert_eq!(plan.warmth, 0.15);
        assert_eq!(plan.health, 100.0);
        assert_eq!(plan.damage, -100.0);
    }

    #[test]
    fn a_forced_hire_takes_the_company_appearance() {
        let plain = hire_plan(4, 0, None);
        assert_eq!(plain.outfit.0, [3, 4, 100, 10, 10, 0, 0, 0]);
        assert_eq!(plain.stat_level, 6);
        let forced = hire_plan(4, 2, None);
        assert_eq!(forced.outfit.0, [0x0b, 4, 100, 10, 10, 66, 140, 1]);
        // Out-of-range companies are ignored.
        assert_eq!(hire_plan(1, 20, None).outfit.flags(), 3);
        assert_eq!(hire_plan(1, -1, None).outfit.flags(), 3);
        // An event weapon row adds flag 4 and asks for a draw.
        let armed = hire_plan(2, 3, Some(5));
        assert_eq!(armed.outfit.flags(), 0x0f);
        assert!(armed.draw_weapon);
        assert_eq!(armed.weapon_row, Some(5));
        assert!(!hire_plan(2, 3, Some(0)).draw_weapon);
        assert!(hire_plan(2, 3, Some(0x20)).draw_weapon);
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
        let plan = manager_plan([1.0, 2.0, 3.0]);
        assert_eq!(plan.position, [1.0, 7.0, 3.0]);
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
