//! The character-level writes the motile part's equip, attack and collapse
//! routines make to the body's own parts: the hands' contact mobility and a
//! collapsing gobbler's femur slack.
//!
//! The original keeps a contact mobility on every part (how far a contact
//! push moves it) and restores it from the part's mass value. Emptying the
//! hands puts each hand back at its base; the equip refresh then halves an
//! unarmed character's hands (a quarter in the heavy-claws state), so a
//! fist pushes as a heavier part; and every attack start in the heavy-claws
//! state quarters both hands, armed or not. A collapse empties an unarmed
//! character's hands (back to the base), and a gobbler's collapse gives
//! each femur's limit link 10 cm of slack. The collapse's drop then lets go
//! of every held item but a shield, so the hands record empties as the
//! original's out set does.
//!
//! Each system runs only on the tick its trigger changes (the hands, a new
//! body, an attack's start, a collapse's start), so it costs nothing on a
//! steady tick.

use bevy::prelude::*;
use exa_combat::Fighter;
use exa_combat::collapse::Collapse;
use exa_formats::weapon_record::{self, race};
use exa_physics::body::Ragdoll;
use exa_physics::grip::{
    HeldGrip, attack_hand_mobility, empty_hand_words, refreshed_hand_mobility,
};

use crate::held::Hands;
use crate::{LEFT_HAND, RIGHT_HAND, RaceWeaponry};

/// The hands' bits of a held item's class: a class without them is not a
/// weapon (the fists, a torch), as the refresh tests its class word.
pub const HANDS_BITS: u32 = 0x30;

/// Whether the character counts as armed for the refresh: its first held
/// item has hands bits. The first held item is the main hand's, or the
/// second weapon once the main one is gone (the original's held list moves
/// it to the front when the first is dropped). Empty hands are the fists,
/// whose class is 0.
pub fn armed(hands: Option<&Hands>) -> bool {
    hands
        .and_then(|h| h.main.or(h.second))
        .is_some_and(|item| u32::from(item.flags) & HANDS_BITS != 0)
}

/// Whether the race is in the heavy-claws state (the original's unarmed
/// set-up decides it from the race index and its claws word).
pub fn heavy_claws(race: Option<&RaceWeaponry>) -> bool {
    let race = race.copied().unwrap_or_default();
    weapon_record::unarmed(race.index, race.weapons.claws).heavy_claws
}

/// Writes both hands' contact mobility after the hands change (or the body
/// is new): the emptying of the hands and the refresh that follows every
/// equip, as [`refreshed_hand_mobility`]; and the fists' weapon bit
/// ([`Ragdoll::set_fist`]), on while the character is unarmed.
#[allow(clippy::type_complexity)]
pub fn refresh_hand_contacts(
    mut bodies: Query<
        (Option<&Hands>, Option<&RaceWeaponry>, &mut Ragdoll),
        (
            With<Fighter>,
            Without<HeldGrip>,
            Or<(Changed<Hands>, Added<Ragdoll>)>,
        ),
    >,
) {
    for (hands, race, mut body) in &mut bodies {
        let (armed, heavy) = (armed(hands), heavy_claws(race));
        for bone in [RIGHT_HAND, LEFT_HAND] {
            body.set_contact_share(bone, |base| refreshed_hand_mobility(base, armed, heavy));
            // The fists are the unarmed character's held items: the
            // unarmed set-up gives both hands the weapon bit, and emptying
            // the hands for an equip takes it away. [inf] A torch alone
            // counts as unarmed here, as the refresh's class test does.
            body.set_fist(bone, !armed);
        }
        if bevy::log::tracing::enabled!(bevy::log::Level::DEBUG) {
            debug!(
                "hands refreshed: armed {armed}, heavy claws {heavy}, fists {}",
                if armed { "off" } else { "on (weapon bit)" }
            );
        }
    }
}

/// Quarters both hands' contact mobility when an attack starts on a
/// character in the heavy-claws state ([`attack_hand_mobility`]).
#[allow(clippy::type_complexity)]
pub fn attack_hand_contacts(
    mut bodies: Query<
        (Option<&RaceWeaponry>, &mut Ragdoll),
        (With<Fighter>, Added<exa_combat::Attacking>),
    >,
) {
    for (race, mut body) in &mut bodies {
        if !heavy_claws(race) {
            continue;
        }
        for bone in [RIGHT_HAND, LEFT_HAND] {
            body.set_contact_share(bone, |base| {
                attack_hand_mobility(base, true).unwrap_or(base)
            });
        }
    }
}

/// The collapse's writes on the tick it starts: an unarmed character's
/// hands are emptied, which puts both back at their base contact mobility
/// ([`empty_hand_words`]; an armed one's drop runs the refresh instead,
/// through the hands' change), and a gobbler's femur limits take their
/// 10 cm of slack ([`Ragdoll::slacken_femur_limits`]).
#[allow(clippy::type_complexity)]
pub fn collapse_words(
    mut bodies: Query<
        (Option<&Hands>, Option<&RaceWeaponry>, &mut Ragdoll),
        (With<Fighter>, Added<Collapse>),
    >,
) {
    for (hands, race, mut body) in &mut bodies {
        if !armed(hands) {
            for bone in [RIGHT_HAND, LEFT_HAND] {
                body.set_contact_share(bone, |base| empty_hand_words(base).1);
            }
        }
        if race.is_some_and(|r| r.index == race::GOBBLER) {
            body.slacken_femur_limits();
        }
    }
}

/// The collapse's drop of the held items, as the holder's hands record it:
/// on the tick a collapse starts, every hand but a shield's lets go
/// ([`Hands::let_go`]). It runs after [`collapse_words`], which reads the
/// hands as they were, so an armed character's emptied hands run the
/// equip refresh on the next tick through the hands' change (the drop's
/// own refresh). The item bodies fall free through
/// [`crate::grip_runtime::release_collapsed`].
pub fn drop_collapsed_hands(mut bodies: Query<&mut Hands, (With<Fighter>, Added<Collapse>)>) {
    for mut hands in &mut bodies {
        let kept = hands.let_go();
        if hands.set_if_neq(kept) {
            debug!("a collapse lets go of the hands' items (a shield stays)");
        }
    }
}

/// The body word systems, on the fixed tick before the solver.
pub struct BodyWordsPlugin;

impl Plugin for BodyWordsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                refresh_hand_contacts,
                attack_hand_contacts,
                collapse_words,
                drop_collapsed_hands,
            )
                .chain()
                .after(exa_physics::body::TickBehaviour)
                .before(exa_physics::grip_solve::GripSolveSet::Sync),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::held::HeldItem;
    use bevy::math::Affine3A;
    use exa_physics::body::PartSetup;

    fn setup(bone: usize, parent: Option<usize>, at: Vec3) -> PartSetup {
        PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.05),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }
    }

    fn arms() -> Ragdoll {
        Ragdoll::new(&[
            setup(1, None, Vec3::ZERO),
            setup(RIGHT_HAND, Some(0), Vec3::X * 0.3),
            setup(LEFT_HAND, Some(0), -Vec3::X * 0.3),
        ])
    }

    fn mobility(app: &App, e: Entity, part: usize) -> f32 {
        app.world().get::<Ragdoll>(e).unwrap().parts[part]
            .pairing
            .mobility
    }

    /// A human's empty hands push at half their base (2.5 × 2/3 × 0.5 =
    /// 0.8333); a sword in hand brings them back to the base; the pelvis is
    /// never touched.
    #[test]
    fn unarmed_hands_take_half_their_base_and_a_weapon_gives_it_back() {
        let mut app = App::new();
        app.add_systems(Update, refresh_hand_contacts);
        let body = arms();
        let pelvis = body.parts[0].pairing.mobility;
        let base = body.contact_base(1).unwrap();
        let e = app
            .world_mut()
            .spawn((Fighter::default(), Hands::default(), body))
            .id();
        app.update();
        assert_eq!(mobility(&app, e, 1), base * 0.5);
        assert_eq!(mobility(&app, e, 2), base * 0.5);
        assert!((mobility(&app, e, 1) - 0.833_333_4).abs() < 1e-6);
        assert_eq!(mobility(&app, e, 0), pelvis);
        app.world_mut().get_mut::<Hands>(e).unwrap().main = Some(HeldItem {
            flags: 0x10,
            light: false,
        });
        app.update();
        assert_eq!(mobility(&app, e, 1), base);
        assert_eq!(mobility(&app, e, 2), base);
        // A torch has no hands bits: the hands count as unarmed.
        app.world_mut().get_mut::<Hands>(e).unwrap().main = Some(HeldItem {
            flags: 0,
            light: true,
        });
        app.update();
        assert_eq!(mobility(&app, e, 1), base * 0.5);
    }

    /// Empty hands are fists: both hands carry the weapon bit (0x1) and the
    /// pelvis does not; a sword takes it off both, dropping it puts it
    /// back, and no other bit of the hands' class words changes.
    #[test]
    fn unarmed_hands_carry_the_weapon_bit() {
        use exa_physics::part::PartState;
        let mut app = App::new();
        app.add_systems(Update, refresh_hand_contacts);
        let body = arms();
        let before: Vec<u32> = body.parts.iter().map(|p| p.state.bits()).collect();
        let e = app
            .world_mut()
            .spawn((Fighter::default(), Hands::default(), body))
            .id();
        let states = |app: &App| -> Vec<u32> {
            app.world()
                .get::<Ragdoll>(e)
                .unwrap()
                .parts
                .iter()
                .map(|p| p.state.bits())
                .collect()
        };
        app.update();
        let fists = states(&app);
        assert_eq!(fists[0], before[0]);
        assert_eq!(fists[1], before[1] | PartState::STRIKING);
        assert_eq!(fists[2], before[2] | PartState::STRIKING);
        app.world_mut().get_mut::<Hands>(e).unwrap().main = Some(HeldItem {
            flags: 0x10,
            light: false,
        });
        app.update();
        assert_eq!(states(&app), before);
        app.world_mut().get_mut::<Hands>(e).unwrap().main = None;
        app.update();
        assert_eq!(states(&app), fists);
    }

    /// A Sir (race index 4, heavy claws) has its unarmed hands at a
    /// quarter; armed, an attack's start quarters them again.
    #[test]
    fn heavy_claws_quarter_the_hands_unarmed_and_on_every_attack() {
        let mut app = App::new();
        app.add_systems(
            Update,
            (refresh_hand_contacts, attack_hand_contacts).chain(),
        );
        let body = arms();
        let base = body.contact_base(1).unwrap();
        let sir = RaceWeaponry {
            index: 4,
            ..RaceWeaponry::default()
        };
        assert!(heavy_claws(Some(&sir)));
        assert!(!heavy_claws(None));
        let e = app
            .world_mut()
            .spawn((Fighter::default(), Hands::default(), sir, body))
            .id();
        app.update();
        assert_eq!(mobility(&app, e, 1), base * 0.25);
        app.world_mut().get_mut::<Hands>(e).unwrap().main = Some(HeldItem {
            flags: 0x20,
            light: false,
        });
        app.update();
        assert_eq!(mobility(&app, e, 2), base);
        app.world_mut()
            .entity_mut(e)
            .insert(exa_combat::Attacking::new(
                exa_combat::attack::AttackMotion::Main(0),
                0,
            ));
        app.update();
        assert_eq!(mobility(&app, e, 1), base * 0.25);
        assert_eq!(mobility(&app, e, 2), base * 0.25);
    }

    /// Only a gobbler's collapse slackens its femur limits.
    #[test]
    fn only_a_collapse_wordsens_the_femurs() {
        let mut app = App::new();
        app.add_systems(Update, collapse_words);
        let hip = |slack| {
            let mut femur = setup(2, Some(0), Vec3::new(0.1, -0.1, 0.0));
            femur.links = vec![(
                Vec3::new(0.0, -0.12, 0.0),
                Vec3::new(0.02, -0.1, 0.07),
                slack,
            )];
            let mut shin = setup(3, Some(1), Vec3::new(0.1, -0.5, 0.0));
            shin.foot = Some(Vec3::new(0.0, -0.45, 0.0));
            Ragdoll::new(&[setup(1, None, Vec3::ZERO), femur, shin])
        };
        let gobbler = app
            .world_mut()
            .spawn((
                RaceWeaponry {
                    index: race::GOBBLER,
                    ..RaceWeaponry::default()
                },
                hip(0.15),
                Collapse::default(),
                Fighter::default(),
            ))
            .id();
        let human = app
            .world_mut()
            .spawn((
                RaceWeaponry::default(),
                hip(0.15),
                Collapse::default(),
                Fighter::default(),
            ))
            .id();
        app.update();
        let slack = |app: &App, e: Entity| {
            let b = app.world().get::<Ragdoll>(e).unwrap();
            b.links[b.parts[1].leg_limit.unwrap()].slack
        };
        assert_eq!(slack(&app, gobbler), 0.1);
        assert_eq!(slack(&app, human), 0.15);
    }

    /// A collapse puts an unarmed character's hands back at their base and
    /// leaves an armed one's; a second weapon left alone still arms.
    #[test]
    fn a_collapse_empties_unarmed_hands() {
        let mut app = App::new();
        app.add_systems(Update, (refresh_hand_contacts, collapse_words).chain());
        let body = arms();
        let base = body.contact_base(1).unwrap();
        let e = app
            .world_mut()
            .spawn((Fighter::default(), Hands::default(), body))
            .id();
        app.update();
        assert_eq!(mobility(&app, e, 1), base * 0.5);
        app.world_mut().entity_mut(e).insert(Collapse::default());
        app.update();
        assert_eq!(mobility(&app, e, 1), base);
        assert_eq!(mobility(&app, e, 2), base);
        let sword = HeldItem {
            flags: 0x10,
            light: false,
        };
        assert!(armed(Some(&Hands {
            second: Some(sword),
            ..Hands::default()
        })));
        assert!(!armed(None));
    }

    /// An armed collapse lets go of the weapon but keeps a shield, and the
    /// emptied hands run the refresh on the next tick: the fists come back
    /// halved, with their weapon bit.
    #[test]
    fn an_armed_collapse_drops_its_weapon_and_keeps_a_shield() {
        let mut app = App::new();
        app.add_systems(
            Update,
            (refresh_hand_contacts, collapse_words, drop_collapsed_hands).chain(),
        );
        let body = arms();
        let base = body.contact_base(1).unwrap();
        let e = app
            .world_mut()
            .spawn((
                Fighter::default(),
                Hands {
                    main: Some(HeldItem::weapon(0x11)),
                    shield: Some(crate::held::ShieldStyle::Plain),
                    ..Hands::default()
                },
                body,
            ))
            .id();
        app.update();
        assert_eq!(mobility(&app, e, 1), base);
        app.world_mut().entity_mut(e).insert(Collapse::default());
        app.update();
        let hands = *app.world().get::<Hands>(e).unwrap();
        assert_eq!(hands.main, None);
        assert_eq!(hands.shield, Some(crate::held::ShieldStyle::Plain));
        assert!(!hands.is_empty());
        app.update();
        assert_eq!(mobility(&app, e, 1), base * 0.5);
    }
}
