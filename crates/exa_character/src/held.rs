//! What a character holds picks the moveset its clips come from, as the
//! original picks it after every equip change, in and out of combat: the
//! fists keep the race's own moveset, a one-handed weapon asks for "One
//! Handed", a shield for one of the shield movesets, and so on. The choice
//! is the original's config word ([`MovesetConfig`]): the weapon moveset is
//! looked up under the race's moveset chain and overlaid on it, with the
//! "Torch" overlay for a light in the main hand. The same choice gives the
//! character's stance class (which arm work an attack asks of the other
//! arm) and weapon class (which attacks it can make). Rules:
//! `docs/PLAYER.md` (Moveset from held items).

use bevy::prelude::*;
use exa_anim::{Animator, MovesetConfig};
use exa_combat::attack::Attacker;

/// The human race's own moveset, which every other moveset falls back to.
pub const DEFAULT_MOVESET: &str = exa_anim::DEFAULT_MOVESET;

/// Weapon classes: the hands bits of the held weapon's flags, or what the
/// choice makes of empty hands and pairs.
pub mod class {
    /// No hands bits: nothing held at all, not even the fists, or a shield
    /// or an off-hand light beside the fists.
    pub const NONE: u8 = 0;
    pub const ONE_HANDED: u8 = 0x10;
    pub const TWO_HANDED: u8 = 0x20;
    pub const POLEARM: u8 = 0x30;
    /// The fists, which strike as weapons.
    pub const FISTS: u8 = 0x80;
    /// The fists of a clawed race.
    pub const CLAWS: u8 = 0xC0;
    /// A one-handed weapon in each hand.
    pub const TWO_WEAPONS: u8 = 0x90;
}

/// The weapon flags (the low 16 bits of the weapon section's first word)
/// of the item that makes a two-hander a greatsword.
pub const GREATSWORD_FLAGS: u16 = 0x521;

/// An item in a hand, as the moveset choice reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldItem {
    /// The low 16 bits of the item's weapon flags (kind in bits 0–3, hands
    /// in bits 4–5).
    pub flags: u16,
    /// The item is a light (a torch).
    pub light: bool,
}

impl HeldItem {
    /// A weapon with these weapon flags (only the low 16 bits count).
    pub fn weapon(flags: u32) -> Self {
        Self {
            flags: (flags & 0xFFFF) as u16,
            light: false,
        }
    }
}

/// Which shield moveset a character with a shield fights with. Two
/// character flags pick Shield C1 or C2; what sets them is not known yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldStyle {
    Plain,
    C1,
    C2,
}

/// What a character holds in its hands. Empty hands are the fists. On a
/// body that has it, the moveset follows it ([`HeldMoveset`]); dressing
/// fills the main hand with a wielded weapon and lighting a carried torch
/// puts it in its hand.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(HeldMoveset)]
pub struct Hands {
    /// The main (right) hand's item.
    pub main: Option<HeldItem>,
    /// A second weapon, in the off hand.
    pub second: Option<HeldItem>,
    /// A shield on the off hand.
    pub shield: Option<ShieldStyle>,
    /// A light (a torch) on the off hand.
    pub off_light: bool,
}

impl Hands {
    /// The hands after their held items came loose (a collapse dropping
    /// them): every hand that does not hold a shield is empty, as the
    /// holder's came-loose hook clears every non-shield hand of the record's
    /// out set at the first drop. A shield stays on the off arm.
    pub fn let_go(self) -> Self {
        Self {
            shield: self.shield,
            ..Self::default()
        }
    }

    /// Whether both hands are empty: no item in the main hand and nothing
    /// (weapon, shield or light) in the off hand.
    pub fn is_empty(&self) -> bool {
        self.main.is_none() && self.second.is_none() && self.shield.is_none() && !self.off_light
    }
}

/// The original's stance class: what the other arm does while one swings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StanceClass {
    OneWeapon = 1,
    /// A two-hander or a polearm.
    TwoHanded = 2,
    /// A weapon and a shield.
    Shield = 3,
    /// A weapon and a light in the off hand.
    OffHandLight = 4,
    /// Empty hands, or a light in the main hand.
    Unarmed = 5,
    /// A one-handed weapon in each hand.
    TwoWeapons = 6,
}

/// What a character's held items make of it: what they ask of its clip
/// table (the weapon moveset and overlays, assembled over the race's own
/// moveset chain), its stance class and its weapon class ([`class`]).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldMoveset {
    pub config: MovesetConfig,
    pub stance: StanceClass,
    pub weapon_class: u8,
}

impl Default for HeldMoveset {
    /// A human's empty hands.
    fn default() -> Self {
        choose(&Hands::default(), false)
    }
}

/// The original's choice of moveset config, stance class and weapon class
/// from what a character holds; `claws` is the race's clawed-fists flag.
/// Empty hands ask for no weapon moveset: the race's own plays.
pub fn choose(hands: &Hands, claws: bool) -> HeldMoveset {
    // The fists are held items whose flags have no hands bits.
    let main = hands.main.unwrap_or(HeldItem {
        flags: 0,
        light: false,
    });
    let hands_bits = (main.flags & 0xF0) as u8;
    let held = |config, stance, weapon_class| HeldMoveset {
        config,
        stance,
        weapon_class,
    };
    if let Some(style) = hands.shield {
        let config = match style {
            ShieldStyle::Plain => MovesetConfig::SHIELD,
            ShieldStyle::C1 => MovesetConfig::SHIELD_C1,
            ShieldStyle::C2 => MovesetConfig::SHIELD_C2,
        };
        // A light in the main hand lays the "Torch" overlay on the shield's.
        let config = if main.light {
            config.with(MovesetConfig::TORCH)
        } else {
            config
        };
        return held(config, StanceClass::Shield, hands_bits);
    }
    if hands.off_light {
        return held(
            MovesetConfig::ONE_HANDED_TORCH,
            StanceClass::OffHandLight,
            hands_bits,
        );
    }
    if main.light {
        return held(
            MovesetConfig::ONE_HANDED.with(MovesetConfig::TORCH),
            StanceClass::Unarmed,
            hands_bits,
        );
    }
    match hands_bits {
        class::ONE_HANDED if hands.second.is_some() => held(
            MovesetConfig::TWO_WEAPONS,
            StanceClass::TwoWeapons,
            class::TWO_WEAPONS,
        ),
        class::ONE_HANDED => held(
            MovesetConfig::ONE_HANDED,
            StanceClass::OneWeapon,
            class::ONE_HANDED,
        ),
        class::TWO_HANDED => {
            let config = if main.flags == GREATSWORD_FLAGS {
                MovesetConfig::GREATSWORD
            } else if main.flags & 1 != 0 {
                MovesetConfig::TWO_HANDED_LIGHT
            } else {
                MovesetConfig::TWO_HANDED
            };
            held(config, StanceClass::TwoHanded, class::TWO_HANDED)
        }
        class::POLEARM => held(
            MovesetConfig::POLEARM,
            StanceClass::TwoHanded,
            class::POLEARM,
        ),
        _ => held(
            MovesetConfig::NONE,
            StanceClass::Unarmed,
            if claws { class::CLAWS } else { class::FISTS },
        ),
    }
}

/// Picks the moveset config of every body whose hands changed, and switches
/// its clips to it: the body keeps its race's own moveset as the base, and
/// the clip playing goes on in the new table. The stance class is copied
/// onto the attacker, which the swing reads when it sets the arms.
#[allow(clippy::type_complexity)]
pub(crate) fn choose_held_movesets(
    mut bodies: Query<
        (
            &Hands,
            &mut HeldMoveset,
            Option<&mut Animator>,
            Option<&mut Attacker>,
        ),
        Changed<Hands>,
    >,
) {
    for (hands, mut held, anim, attacker) in &mut bodies {
        let chosen = choose(hands, false);
        if let Some(mut anim) = anim
            && anim.config != chosen.config
        {
            anim.config = chosen.config;
        }
        held.set_if_neq(chosen);
        if let Some(mut attacker) = attacker {
            attacker.stance_class = chosen.stance as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use MovesetConfig as C;

    fn weapon(flags: u32) -> Option<HeldItem> {
        Some(HeldItem::weapon(flags))
    }

    fn pick(hands: Hands) -> (MovesetConfig, StanceClass, u8) {
        let h = choose(&hands, false);
        (h.config, h.stance, h.weapon_class)
    }

    #[test]
    fn empty_hands_keep_the_race_moveset() {
        assert_eq!(
            pick(Hands::default()),
            (C::NONE, StanceClass::Unarmed, 0x80)
        );
        let claws = choose(&Hands::default(), true);
        assert_eq!(
            (claws.config, claws.stance, claws.weapon_class),
            (C::NONE, StanceClass::Unarmed, 0xC0)
        );
        // Items whose hands bits are not a weapon's count as fists too.
        let odd = Hands {
            main: weapon(0x41),
            ..default()
        };
        assert_eq!(pick(odd), (C::NONE, StanceClass::Unarmed, 0x80));
    }

    #[test]
    fn weapons_ask_for_their_movesets() {
        let main = |flags| Hands {
            main: weapon(flags),
            ..default()
        };
        // A plain sword (bladed, one-handed), with haft bits above 16.
        assert_eq!(
            pick(main(0x0003_0011)),
            (C(0x1), StanceClass::OneWeapon, 0x10)
        );
        let pair = Hands {
            main: weapon(0x11),
            second: weapon(0x12),
            ..default()
        };
        assert_eq!(pick(pair), (C(0x4), StanceClass::TwoWeapons, 0x90));
        assert_eq!(pick(main(0x521)), (C(0x30), StanceClass::TwoHanded, 0x20));
        assert_eq!(pick(main(0x21)), (C(0x20), StanceClass::TwoHanded, 0x20));
        assert_eq!(pick(main(0x23)), (C(0x20), StanceClass::TwoHanded, 0x20));
        assert_eq!(pick(main(0x22)), (C(0x10), StanceClass::TwoHanded, 0x20));
        assert_eq!(pick(main(0x31)), (C(0x40), StanceClass::TwoHanded, 0x30));
        let names = |flags| choose(&main(flags), false).config.weapon_moveset();
        assert_eq!(names(0x11), Some("One Handed"));
        assert_eq!(names(0x521), Some("Greatsword"));
        assert_eq!(names(0x21), Some("2H Light"));
        assert_eq!(names(0x22), Some("Two Handed"));
        assert_eq!(names(0x31), Some("Polearm"));
    }

    #[test]
    fn shields_and_lights_come_first() {
        let sword = weapon(0x11);
        let shield = |style| Hands {
            main: sword,
            shield: Some(style),
            ..default()
        };
        assert_eq!(
            pick(shield(ShieldStyle::Plain)),
            (C(0x100), StanceClass::Shield, 0x10)
        );
        assert_eq!(pick(shield(ShieldStyle::C1)).0, C(0x200));
        assert_eq!(pick(shield(ShieldStyle::C2)).0, C(0x300));
        let off_light = Hands {
            main: sword,
            off_light: true,
            ..default()
        };
        assert_eq!(pick(off_light), (C(0x3), StanceClass::OffHandLight, 0x10));
        // Beside empty hands the off-hand light keeps the fists' bare
        // class: no hands bits.
        let lone_light = Hands {
            off_light: true,
            ..default()
        };
        assert_eq!(pick(lone_light), (C(0x3), StanceClass::OffHandLight, 0));
        // The torch (a one-handed stick) in the main hand: "One Handed"
        // with the "Torch" overlay.
        let torch = HeldItem {
            flags: 0x13,
            light: true,
        };
        let in_hand = Hands {
            main: Some(torch),
            ..default()
        };
        assert_eq!(pick(in_hand), (C(0x1001_0001), StanceClass::Unarmed, 0x10));
        // Beside a shield the torch lays its overlay on the shield's.
        let torch_and_shield = Hands {
            main: Some(torch),
            shield: Some(ShieldStyle::C1),
            ..default()
        };
        assert_eq!(
            pick(torch_and_shield),
            (C(0x1001_0200), StanceClass::Shield, 0x10)
        );
    }

    #[test]
    fn a_change_of_hands_switches_the_clips() {
        let mut app = App::new();
        app.add_systems(Update, choose_held_movesets);
        let mut ogre = Animator::new("Ogre", "Cmbt - Idle");
        ogre.config = C::POLEARM;
        let body = app
            .world_mut()
            .spawn((Hands::default(), ogre, Attacker::default()))
            .id();
        app.update();
        let anim = app.world().get::<Animator>(body).unwrap();
        assert_eq!(
            app.world().get::<Attacker>(body).unwrap().stance_class,
            StanceClass::Unarmed as u8
        );
        // The race's own moveset stays the base.
        assert_eq!(
            (anim.moveset.as_str(), anim.config, anim.action.as_str()),
            ("Ogre", C::NONE, "Cmbt - Idle")
        );
        app.world_mut().get_mut::<Hands>(body).unwrap().main = weapon(0x11);
        app.update();
        let world = app.world();
        let anim = world.get::<Animator>(body).unwrap();
        assert_eq!(
            (anim.moveset.as_str(), anim.config),
            ("Ogre", C::ONE_HANDED)
        );
        let held = world.get::<HeldMoveset>(body).unwrap();
        assert_eq!(
            (held.stance, held.weapon_class),
            (StanceClass::OneWeapon, 0x10)
        );
        assert_eq!(
            world.get::<Attacker>(body).unwrap().stance_class,
            StanceClass::OneWeapon as u8
        );
    }

    /// Every weapon item of the installed object database asks for a
    /// moveset the installed motion set has, and the "Default" punches are
    /// one clip per side.
    #[test]
    fn shipped_weapons_pick_shipped_movesets() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let items = super::super::objdb_items(&game);
        let rsq = game
            .pack(exa_assets::PackKind::Resource)
            .and_then(|p| p.get("humanoid.rsq"))
            .unwrap();
        let motions = exa_formats::rsq::Sequences::parse(rsq).unwrap().motions;
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for item in &items {
            let Some(w) = item.weapon else { continue };
            let hands = Hands {
                main: weapon(w.flags),
                ..default()
            };
            let held = choose(&hands, false);
            let name = held.config.weapon_moveset().expect("a weapon moveset");
            for action in ["Cmbt - Idle", "Defence stance", "Attack Mid", "Idle"] {
                assert!(
                    motions.clip(DEFAULT_MOVESET, held.config, action).is_some(),
                    "{} ({:#x}): {name} has no {action}",
                    item.name,
                    w.flags,
                );
            }
            *counts.entry(name.into()).or_default() += 1;
        }
        // No weapon item lacks hands bits; none is a pair or a shield.
        let expected = [
            ("2H Light", 18),
            ("Greatsword", 3),
            ("One Handed", 136),
            ("Polearm", 32),
            ("Two Handed", 19),
        ];
        assert_eq!(
            counts
                .iter()
                .map(|(m, n)| (m.as_str(), *n))
                .collect::<Vec<_>>(),
            expected
        );
        let clip = |a: &str| motions.clip(DEFAULT_MOVESET, C::NONE, a).unwrap();
        assert_eq!(clip("Attack Mid"), clip("Attack Down"));
        assert_eq!(clip("Attack Mid"), clip("Attack Up"));
        assert_eq!(clip("Attack Combo 1"), clip("Attack Combo 2"));
        assert_eq!(clip("Attack Combo 1"), clip("Attack Combo 3"));
        assert_ne!(clip("Attack Mid"), clip("Attack Combo 1"));
    }
}
