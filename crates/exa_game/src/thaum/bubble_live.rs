//! The force bubble in the running game: the dome entity its cast makes,
//! standing where it was cast while its power runs.
//!
//! - **Start** ([`new_bubble`]): the start plays cast motion 0x24 with the
//!   power id 0x20207; when that motion starts the caster's cast hook
//!   becomes the bubble's, which answers −1 to a cast's start (0 only to a
//!   call carrying bits 0x28). The dome grows at the cast place, 120 cm
//!   across ([`Bubble::cast`]), and the power class holds its focus.
//! - **Each tick** ([`Bubble::tick`]): the grow passing 1 stands the dome and
//!   clears the caster's hook and power id; the power's ending tells the
//!   dome to end (state 2), which breaks it, shrinks it over 700 ms and
//!   then the entity leaves the scene. The power ends when it is cast again
//!   or once the dome lies more than 20 m from its caster.
//! - **Keeping out** ([`keep_out`]): the caster's own limb spheres that reach
//!   10 cm inside the shell are pushed back in ([`Bubble::own_part_push`]);
//!   another body's parts within reach ([`Bubble::reaches`]) that the shell
//!   covers are pushed out ([`Bubble::other_part_push`], the part's mass its
//!   own). Every push strikes a ripple, and a body any of whose parts were
//!   pushed takes [`Dome::contact`]: its caster is shoved back toward the
//!   centre once beyond the radius less 30 cm, another character shoved
//!   away within the radius plus 20 cm, 40 cm either way. [inf: the shove is
//!   a knockback step, as the barrier's is, and the contact runs once a
//!   body, not once a part.]
//!
//! Not wired: the balance the contact drains and the knock count, the cast,
//! break and hit sounds, the glow and the draw.
//!
//! Evidence: the bubble's start, hook, tick and keep-out, and its power
//! class's control, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_physics::body::{BodyTicks, Ragdoll};

use super::barricade::WAKE_TICKS;
use super::cast::PowerCaster;
use super::casting::{self, CastBody};
use super::live::{EffectDraws, EffectRoll};
use super::power::force::{BUBBLE, BUBBLE_MOTION};
use crate::effects::force_dome::{Bubble, CONTACT_PUSH, Dome, DomeHit, Phase};

/// Centimetres a metre.
const CM: f32 = 100.0;
/// What the bubble's hook answers a cast's start.
pub const BUBBLE_HOOK: i32 = -1;
/// The bubble's radius, cm.
pub const BUBBLE_RADIUS: f32 = 120.0;

/// A bubble in the scene.
#[derive(Component)]
pub struct LiveBubble {
    pub bubble: Bubble,
    pub caster: Entity,
}

/// A bubble for `caster` (at `standing`, cm, its feet) around `at`: the cast
/// motion, the hook when the motion starts, and the growing dome. Whether
/// the motion started.
pub fn new_bubble(
    body: &mut CastBody,
    caster: Entity,
    at: Vec3,
    standing: Vec3,
    now: u32,
    draws: &mut EffectDraws,
) -> (LiveBubble, bool) {
    let motion = casting::start_cast(body, BUBBLE_MOTION, BUBBLE);
    if motion {
        body.cast_hook = Some(BUBBLE_HOOK);
    }
    let (bubble, _sound) = Bubble::cast(BUBBLE_RADIUS, at, now, standing, standing.y, &mut draws.0);
    (LiveBubble { bubble, caster }, motion)
}

/// The power's ending tells the dome to end.
pub fn end_bubble(live: &mut LiveBubble) {
    live.bubble.dome.phase = Phase::Ending;
}

/// One body against the bubble: its parts pushed in (`own`, the caster's)
/// or out, each push striking a ripple. Whether any part was pushed.
pub fn keep_out(bubble: &mut Bubble, own: bool, body: &mut Ragdoll) -> bool {
    let centre = bubble.dome.at;
    bubble.dome.ripples.begin_body();
    let mut pushed = false;
    for k in 0..body.parts.len() {
        let part = &body.parts[k];
        if part.inverse_mass <= 0.0 {
            continue;
        }
        let mass = 1.0 / part.inverse_mass;
        let world = part.transform();
        let spheres: Vec<(Vec3, f32)> = part
            .spheres
            .iter()
            .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
            .collect();
        if !own {
            let middle = Vec3::from(world.translation) * CM;
            let bound = spheres
                .iter()
                .map(|&(c, r)| c.distance(middle) + r)
                .fold(0.0, f32::max);
            if !bubble.reaches(middle, bound) {
                continue;
            }
        }
        for (s, (at, radius)) in spheres.into_iter().enumerate() {
            let local = at - centre;
            let push = if own {
                bubble.own_part_push(local, radius)
            } else {
                bubble.other_part_push(local, radius, mass)
            };
            let Some(push) = push else {
                continue;
            };
            body.push_sphere(k, s, push.push / CM);
            bubble.dome.ripples.strike(push.ripple_at, push.strength);
            pushed = true;
        }
    }
    pushed
}

/// The response of a body the dome pushed.
fn contact(dome: &mut Dome, hit: &DomeHit, now: u32, roll: &mut EffectRoll, body: &mut Ragdoll) {
    let Some(contact) = dome.contact(hit, now, &mut roll.0) else {
        return;
    };
    if let Some(shove) = contact.shove {
        body.begin_knockback(shove * (CONTACT_PUSH / CM));
    }
}

/// Ticks every bubble in the scene.
pub(crate) fn tick_bubbles(
    mut commands: Commands,
    mut bubbles: Query<(Entity, &mut LiveBubble)>,
    mut casters: Query<&mut PowerCaster>,
    mut bodies: Query<(Entity, &mut Ragdoll, Option<&GlobalTransform>)>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (entity, mut live) in &mut bubbles {
        let live = &mut *live;
        let (_break_sound, stood) = live.bubble.tick(now, &mut roll.0);
        if stood && let Ok(mut caster) = casters.get_mut(live.caster) {
            caster.body.cast_hook = None;
            caster.body.casting = 0;
            caster.sync_body();
        }
        if live.bubble.dome.phase == Phase::Gone {
            commands.entity(entity).despawn();
            continue;
        }
        let centre = live.bubble.dome.at;
        for (other, mut body, at) in &mut bodies {
            let own = other == live.caster;
            if !keep_out(&mut live.bubble, own, &mut body) {
                continue;
            }
            body.touch(WAKE_TICKS);
            let Some(at) = at.map(|a| a.translation() * CM) else {
                continue;
            };
            let hit = DomeHit {
                offset: at - centre,
                scale: 1.0,
                caster: own,
                limp: body.drive.is_none(),
                struck_flag: false,
            };
            contact(&mut live.bubble.dome, &hit, now, &mut roll, &mut body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;

    #[test]
    fn the_start_plays_the_motion_and_takes_the_hook() {
        let mut body = CastBody::default();
        let mut draws = EffectDraws(Mt19937::new(1));
        let at = Vec3::new(0.0, 100.0, 0.0);
        let (live, motion) = new_bubble(
            &mut body,
            Entity::PLACEHOLDER,
            at,
            Vec3::ZERO,
            0,
            &mut draws,
        );
        assert!(motion);
        assert_eq!(body.casting, BUBBLE);
        assert_eq!(body.cast_motion, Some(BUBBLE_MOTION));
        assert_eq!(body.cast_hook, Some(BUBBLE_HOOK));
        assert_eq!(live.bubble.dome.radius, BUBBLE_RADIUS);
        assert_eq!(live.bubble.dome.at, at);
        assert_eq!(live.bubble.dome.phase, Phase::Growing);
        // The hook answers −1, so a later cast still starts.
        assert!(casting::start_cast(&mut body, 0x22, 0x201));
    }

    #[test]
    fn the_ending_breaks_and_shrinks_the_dome() {
        let mut body = CastBody::default();
        let mut draws = EffectDraws(Mt19937::new(1));
        let (mut live, _) = new_bubble(
            &mut body,
            Entity::PLACEHOLDER,
            Vec3::ZERO,
            Vec3::ZERO,
            0,
            &mut draws,
        );
        let mut roll = crate::thaum::engine::Roll(1);
        let (_, stood) = live.bubble.tick(1_300, &mut roll);
        assert!(stood);
        end_bubble(&mut live);
        let (sound, _) = live.bubble.tick(1_312, &mut roll);
        assert!(sound.is_some());
        assert_eq!(live.bubble.dome.phase, Phase::Shrinking);
        live.bubble.tick(2_100, &mut roll);
        assert_eq!(live.bubble.dome.phase, Phase::Gone);
    }
}
