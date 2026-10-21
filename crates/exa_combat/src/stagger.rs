//! The blow stagger on the running fighter body.
//!
//! The original's damage routine starts a body-level stagger through two
//! helpers: a wound's follow-through ([`wound_stagger`]) and the smaller
//! recoil of a parried or blocked blow ([`recoil_stagger`]). The body keeps
//! it in [`Stagger`]; each body tick its speed loses its decay and its
//! countdown runs out ([`advance_staggers`], in the fighter's attack stage
//! before the swing routine), and while it lasts the swing's clip rate is
//! multiplied by its speed (`SwingState::run_swings`).
//!
//! The helper reads the striking part's factor and a share of its speed
//! that Exarust does not keep: its weapons are bone-attached meshes, not
//! bodies with the original's part table. The held weapon's impact factor
//! (W10) stands in for the part factor and the blow's penetration for the
//! share — guesses, marked here and in the report.

use bevy::prelude::*;
use exa_physics::body::stagger::{self, Stagger};
use exa_physics::body::{BodyTicks, TickedCharacter};

use crate::damage::{Outcome, WeaponRecord};

/// One body tick of the ticked fighter's stagger: its speed loses its decay
/// and, once its countdown has passed, it leaves the body.
pub(crate) fn advance_staggers(
    mut commands: Commands,
    ticked: Res<TickedCharacter>,
    ticks: Option<Res<BodyTicks>>,
    mut bodies: Query<(Entity, &mut Stagger)>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis());
    let Ok((entity, mut stagger)) = bodies.get_mut(ticked.body) else {
        return;
    };
    if stagger.advance(now) {
        commands.entity(entity).remove::<Stagger>();
    }
}

/// The stagger the original starts after a wound's follow-through, at
/// `gain` (1 − I/2, 0.5 or the body-zone share), for a blow whose striking
/// part has the held weapon's `weapon` record, `weapon_class`, the
/// striker's swing `flags` and `clock`, and whether its part can parry.
/// The guard's parry test skips the stagger. Lengths and speeds are the
/// original's.
#[allow(clippy::too_many_arguments)]
pub fn wound_stagger(
    gain: f32,
    blow: &Outcome,
    weapon: &WeaponRecord,
    weapon_class: i32,
    swing_flags: u32,
    clock: f32,
    can_parry: bool,
    now_ms: u64,
) -> Option<Stagger> {
    let parried = stagger::parries_stagger(weapon_class, swing_flags, clock, can_parry);
    // The part factor and the speed share are Exarust's stand-ins (see the
    // module note).
    let share = blow.penetration.clamp(0.0, 1.0);
    stagger::stagger_strength(
        gain,
        weapon.impact,
        blow.speed,
        share,
        false,
        parried,
        now_ms,
    )
}

/// The smaller stagger a recoil starts: `speed` is the blow's speed times
/// the weapon's recoil, as the original's damage routine hands it. A
/// parried recoil still sets the speed and decay but starts no countdown.
pub fn recoil_stagger(
    speed: f32,
    weapon_class: i32,
    swing_flags: u32,
    clock: f32,
    can_parry: bool,
    now_ms: u64,
) -> Stagger {
    let r = stagger::recoil(speed, weapon_class, swing_flags, clock, can_parry, now_ms);
    match r.started {
        Some(start) => Stagger {
            speed: r.speed,
            decay: r.decay,
            until_ms: start.until_ms,
            flags: start.flags,
        },
        None => Stagger {
            speed: r.speed,
            decay: r.decay,
            until_ms: now_ms,
            flags: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::PartArmour;

    /// A wound's follow-through gain 0.5 on the sword's tip: the helper's
    /// strength 0.5 × W10 × speed × 0.15, its decay (1 − √penetration ×
    /// strength) × 0.05 and its countdown round(strength × 400) ticks on.
    #[test]
    fn a_wound_starts_the_stagger() {
        use crate::Roll;
        use crate::damage::Strike;
        let strike = Strike {
            speed: 20.0,
            clock: 0.5,
            along: 70.0,
            ..Strike::default()
        };
        let blow = crate::damage::resolve(
            &crate::damage::sword_a(),
            &strike,
            Some((&PartArmour::default(), Roll::default().next())),
            9,
            &crate::damage::Victim::new(0.25, 0.0),
        )
        .unwrap();
        let s = wound_stagger(
            0.5,
            &blow,
            &crate::damage::sword_a(),
            0x10,
            0,
            0.5,
            true,
            1200,
        )
        .unwrap();
        let strength =
            0.5 * f64::from(crate::damage::sword_a().impact) * f64::from(blow.speed) * 0.15;
        let share = f64::from(blow.penetration.clamp(0.0, 1.0));
        let value = (share.sqrt() * strength) as f32;
        let value = if value > 1.0 { 1.0 } else { value };
        assert!((s.speed - value).abs() < 1e-6);
        assert!((s.decay - ((1.0 - value) * 0.05)).abs() < 1e-6);
        assert_eq!(
            s.until_ms,
            1200 + (strength as f32 * 400.0).round_ties_even() as u64
        );
        assert_eq!(s.flags, stagger::STAGGER_FLAGS);
    }

    /// The body tick decays the ticked fighter's stagger a tick and takes
    /// it off once its countdown has passed.
    #[test]
    fn the_body_tick_decays_and_removes_the_stagger() {
        let mut app = App::new();
        app.insert_resource(BodyTicks(10))
            .add_systems(Update, advance_staggers);
        let body = app
            .world_mut()
            .spawn(Stagger {
                speed: 0.15,
                decay: 0.0425,
                until_ms: 160,
                flags: stagger::STAGGER_FLAGS,
            })
            .id();
        app.world_mut().insert_resource(TickedCharacter {
            body,
            character: None,
        });
        app.update();
        let s = app.world().get::<Stagger>(body).unwrap();
        assert!((s.speed - 0.1075).abs() < 1e-6);
        app.insert_resource(BodyTicks(20));
        app.update();
        assert!(app.world().get::<Stagger>(body).is_none());
    }

    /// A recoil's speed and decay are 0.5 and it ends
    /// round(speed × 0.1 × 400) + 200 ms on; a class word 0x90 in its
    /// strike window is parried and starts none.
    #[test]
    fn a_recoil_starts_the_smaller_stagger() {
        let s = recoil_stagger(0.5 * 0.1, 0x10, 0, 0.5, true, 1000);
        assert_eq!((s.speed, s.decay), (0.5, 0.5));
        assert_eq!(s.until_ms, 1000 + 2 + 200);
        assert_eq!(s.flags, 0x26);
        let parried = recoil_stagger(0.5 * 0.1, 0x90, 0x1_0000, 0.5, true, 1000);
        assert_eq!((parried.speed, parried.decay), (0.5, 0.5));
        assert_eq!((parried.flags, parried.until_ms), (0, 1000));
    }
}
