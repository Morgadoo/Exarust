//! Who a body was last used by, and for how long: a kinesis pick stamps
//! the body it grabs, and a throw stamps its body every tick it coasts
//! after the shell drops, each with the caster for 2 s
//! ([`super::power::force::hold_until`]). While the stamp holds, a limb
//! the body hurts ([`exa_combat::impact::LimbImpact`]) adds to the struck
//! character's concern toward the user: the script's `AddConcern` with an
//! amount of 0.9 and a gain of three times the damage, when the user is
//! one of its contacts.
//!
//! [inf] The original stamps every node of the body; Exarust stamps the
//! body. The original passes the struck brain's own aggression trait;
//! Exarust's perception has none either and passes 0.

use bevy::prelude::*;
use exa_ai::combatant::Combatant;
use exa_combat::impact::LimbImpact;
use exa_physics::body::BodyTicks;

/// The body's user until the game clock passes `until_ms`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserStamp {
    pub user: Entity,
    pub until_ms: u32,
}

impl UserStamp {
    /// The stamp still holds at `now` (an unsigned compare).
    pub fn holds(&self, now: u32) -> bool {
        self.until_ms > now
    }
}

/// The concern's amount and its gain per unit of damage.
pub const BLAME_AMOUNT: f32 = 0.9;
pub const BLAME_GAIN: f32 = 3.0;

/// The struck character's concern toward the user of a stamped body that
/// hurt it.
pub(super) fn blame_impacts(
    mut impacts: MessageReader<LimbImpact>,
    ticks: Option<Res<BodyTicks>>,
    stamps: Query<&UserStamp>,
    mut brains: Query<&mut Combatant>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for hit in impacts.read() {
        let Ok(stamp) = stamps.get(hit.other) else {
            continue;
        };
        if !stamp.holds(now) {
            continue;
        }
        if let Ok(mut brain) = brains.get_mut(hit.target)
            && let Some(concern) =
                brain.add_concern(stamp.user, 0.0, BLAME_AMOUNT, hit.amount * BLAME_GAIN)
        {
            info!(
                "{:?} blames {:?} for {:?}'s impact: concern {concern:.3}",
                hit.target, stamp.user, hit.other
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use exa_ai::combatant::Contact;

    use super::*;

    /// A stamped body that hurts a character grows its concern toward the
    /// user while the stamp holds; past it, or with no stamp, nothing.
    #[test]
    fn a_stamped_body_blames_its_user() {
        let mut app = App::new();
        app.add_message::<LimbImpact>()
            .insert_resource(BodyTicks(100));
        let now = app.world().resource::<BodyTicks>().millis() as u32;
        let user = app.world_mut().spawn_empty().id();
        let mut brain = Combatant::default();
        brain.contacts.push(Contact::new(user));
        let target = app.world_mut().spawn(brain).id();
        let crate_body = app
            .world_mut()
            .spawn(UserStamp {
                user,
                until_ms: now + 1,
            })
            .id();
        let stale = app
            .world_mut()
            .spawn(UserStamp {
                user,
                until_ms: now,
            })
            .id();
        let hit = |other| LimbImpact {
            target,
            bone: 1,
            other,
            amount: 0.05,
        };
        let concern = |app: &App| app.world().get::<Combatant>(target).unwrap().contacts[0].concern;
        app.world_mut().write_message(hit(stale));
        app.world_mut().run_system_once(blame_impacts).unwrap();
        assert_eq!(concern(&app), 0.0);
        app.world_mut().write_message(hit(crate_body));
        app.world_mut().run_system_once(blame_impacts).unwrap();
        let mut want = Contact::new(user);
        exa_ai::combatant::add_concern(&mut want, 0.0, BLAME_AMOUNT, 0.05 * BLAME_GAIN);
        assert_eq!(concern(&app), want.concern);
        assert!(want.concern > 0.0);
    }
}
