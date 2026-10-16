//! What a character holds picks the moveset its clips come from, as the
//! original picks it after every equip change, in and out of combat: the
//! fists use the race's own moveset, a one-handed weapon "One Handed", a
//! shield one of the shield movesets, and so on. The same choice gives the
//! character's stance class (which arm work an attack asks of the other
//! arm) and weapon class (which attacks it can make). Rules:
//! `docs/PLAYER.md` (Moveset from held items).

use bevy::prelude::*;
use exa_anim::Animator;

/// The human race's own moveset, which every other moveset falls back to.
pub const DEFAULT_MOVESET: &str = "Default";

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

/// What a character's held items make of it: the moveset its clips are
/// looked up in (the moveset's parents in `humanoid.rsq` supply what it
/// lacks), its stance class and its weapon class ([`class`]).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct HeldMoveset {
    pub moveset: String,
    pub stance: StanceClass,
    pub weapon_class: u8,
}

impl Default for HeldMoveset {
    /// A human's empty hands.
    fn default() -> Self {
        choose(&Hands::default(), DEFAULT_MOVESET, false)
    }
}

/// The original's choice of moveset, stance class and weapon class from
/// what a character holds. `base` is the race's own moveset (the empty
/// hands' one) and `claws` the race's clawed-fists flag.
///
/// Not yet: a shield moveset with the "Torch" overlay of a torch in the
/// main hand (a moveset name here is a single moveset and its parents;
/// Exarust holds no shields yet), and the armed movesets of races other
/// than the human (looked up under the race's own moveset).
pub fn choose(hands: &Hands, base: &str, claws: bool) -> HeldMoveset {
    // The fists are held items whose flags have no hands bits.
    let main = hands.main.unwrap_or(HeldItem {
        flags: 0,
        light: false,
    });
    let hands_bits = (main.flags & 0xF0) as u8;
    let held = |moveset: &str, stance, weapon_class| HeldMoveset {
        moveset: moveset.into(),
        stance,
        weapon_class,
    };
    if let Some(style) = hands.shield {
        let moveset = match style {
            ShieldStyle::Plain => "1H & Shield",
            ShieldStyle::C1 => "Shield C1",
            ShieldStyle::C2 => "Shield C2",
        };
        return held(moveset, StanceClass::Shield, hands_bits);
    }
    if hands.off_light {
        return held("1H & Torch", StanceClass::OffHandLight, hands_bits);
    }
    if main.light {
        // "One Handed" with the "Torch" overlay.
        return held("Torch", StanceClass::Unarmed, hands_bits);
    }
    match hands_bits {
        class::ONE_HANDED if hands.second.is_some() => {
            held("2W Light", StanceClass::TwoWeapons, class::TWO_WEAPONS)
        }
        class::ONE_HANDED => held("One Handed", StanceClass::OneWeapon, class::ONE_HANDED),
        class::TWO_HANDED => {
            let moveset = if main.flags == GREATSWORD_FLAGS {
                "Greatsword"
            } else if main.flags & 1 != 0 {
                "2H Light"
            } else {
                "Two Handed"
            };
            held(moveset, StanceClass::TwoHanded, class::TWO_HANDED)
        }
        class::POLEARM => held("Polearm", StanceClass::TwoHanded, class::POLEARM),
        _ => held(
            base,
            StanceClass::Unarmed,
            if claws { class::CLAWS } else { class::FISTS },
        ),
    }
}

/// Picks the moveset of every body whose hands changed, and switches its
/// clips to it: the clip playing goes on in the new moveset.
pub(crate) fn choose_held_movesets(
    mut bodies: Query<(&Hands, &mut HeldMoveset, Option<&mut Animator>), Changed<Hands>>,
) {
    for (hands, mut held, anim) in &mut bodies {
        let chosen = choose(hands, DEFAULT_MOVESET, false);
        if let Some(mut anim) = anim
            && anim.moveset != chosen.moveset
        {
            anim.moveset = chosen.moveset.clone();
        }
        held.set_if_neq(chosen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(flags: u32) -> Option<HeldItem> {
        Some(HeldItem::weapon(flags))
    }

    fn pick(hands: Hands) -> (String, StanceClass, u8) {
        let h = choose(&hands, DEFAULT_MOVESET, false);
        (h.moveset, h.stance, h.weapon_class)
    }

    #[test]
    fn empty_hands_fight_with_the_race_moveset() {
        assert_eq!(
            pick(Hands::default()),
            ("Default".into(), StanceClass::Unarmed, 0x80)
        );
        let golem = choose(&Hands::default(), "Golem", false);
        assert_eq!(golem.moveset, "Golem");
        let claws = choose(&Hands::default(), "UnarmClaw", true);
        assert_eq!(
            (claws.stance, claws.weapon_class),
            (StanceClass::Unarmed, 0xC0)
        );
        // Items whose hands bits are not a weapon's count as fists too.
        let odd = Hands {
            main: weapon(0x41),
            ..default()
        };
        assert_eq!(pick(odd), ("Default".into(), StanceClass::Unarmed, 0x80));
    }

    #[test]
    fn weapons_pick_their_movesets() {
        let main = |flags| Hands {
            main: weapon(flags),
            ..default()
        };
        // A plain sword (bladed, one-handed), with haft bits above 16.
        assert_eq!(
            pick(main(0x0003_0011)),
            ("One Handed".into(), StanceClass::OneWeapon, 0x10)
        );
        let pair = Hands {
            main: weapon(0x11),
            second: weapon(0x12),
            ..default()
        };
        assert_eq!(
            pick(pair),
            ("2W Light".into(), StanceClass::TwoWeapons, 0x90)
        );
        assert_eq!(
            pick(main(0x521)),
            ("Greatsword".into(), StanceClass::TwoHanded, 0x20)
        );
        assert_eq!(
            pick(main(0x21)),
            ("2H Light".into(), StanceClass::TwoHanded, 0x20)
        );
        assert_eq!(
            pick(main(0x23)),
            ("2H Light".into(), StanceClass::TwoHanded, 0x20)
        );
        assert_eq!(
            pick(main(0x22)),
            ("Two Handed".into(), StanceClass::TwoHanded, 0x20)
        );
        assert_eq!(
            pick(main(0x31)),
            ("Polearm".into(), StanceClass::TwoHanded, 0x30)
        );
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
            ("1H & Shield".into(), StanceClass::Shield, 0x10)
        );
        assert_eq!(pick(shield(ShieldStyle::C1)).0, "Shield C1");
        assert_eq!(pick(shield(ShieldStyle::C2)).0, "Shield C2");
        let off_light = Hands {
            main: sword,
            off_light: true,
            ..default()
        };
        assert_eq!(
            pick(off_light),
            ("1H & Torch".into(), StanceClass::OffHandLight, 0x10)
        );
        // Beside empty hands the off-hand light keeps the fists' bare
        // class: no hands bits.
        let lone_light = Hands {
            off_light: true,
            ..default()
        };
        assert_eq!(
            pick(lone_light),
            ("1H & Torch".into(), StanceClass::OffHandLight, 0)
        );
        // The torch (a one-handed stick) in the main hand.
        let torch = Hands {
            main: Some(HeldItem {
                flags: 0x13,
                light: true,
            }),
            ..default()
        };
        assert_eq!(pick(torch), ("Torch".into(), StanceClass::Unarmed, 0x10));
    }

    #[test]
    fn a_change_of_hands_switches_the_clips() {
        let mut app = App::new();
        app.add_systems(Update, choose_held_movesets);
        let body = app
            .world_mut()
            .spawn((Hands::default(), Animator::new("One Handed", "Cmbt - Idle")))
            .id();
        app.update();
        let anim = app.world().get::<Animator>(body).unwrap();
        assert_eq!(
            (anim.moveset.as_str(), anim.action.as_str()),
            ("Default", "Cmbt - Idle")
        );
        app.world_mut().get_mut::<Hands>(body).unwrap().main = weapon(0x11);
        app.update();
        let world = app.world();
        assert_eq!(world.get::<Animator>(body).unwrap().moveset, "One Handed");
        let held = world.get::<HeldMoveset>(body).unwrap();
        assert_eq!(
            (held.stance, held.weapon_class),
            (StanceClass::OneWeapon, 0x10)
        );
    }

    /// Every weapon item of the installed object database picks a moveset
    /// the installed motion set has, and the "Default" punches are one clip
    /// per side.
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
            let held = choose(&hands, DEFAULT_MOVESET, false);
            for action in ["Cmbt - Idle", "Defence stance", "Attack Mid", "Idle"] {
                assert!(
                    motions.clip(&held.moveset, action).is_some(),
                    "{} ({:#x}): {} has no {action}",
                    item.name,
                    w.flags,
                    held.moveset
                );
            }
            *counts.entry(held.moveset).or_default() += 1;
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
        let clip = |a: &str| motions.clip(DEFAULT_MOVESET, a).unwrap();
        assert_eq!(clip("Attack Mid"), clip("Attack Down"));
        assert_eq!(clip("Attack Mid"), clip("Attack Up"));
        assert_eq!(clip("Attack Combo 1"), clip("Attack Combo 2"));
        assert_eq!(clip("Attack Combo 1"), clip("Attack Combo 3"));
        assert_ne!(clip("Attack Mid"), clip("Attack Combo 1"));
    }
}
