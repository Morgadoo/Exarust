//! The force blast and bolt in the running game: the effect entity a cast
//! makes, ticked against its caster's body and the scene's other bodies.
//!
//! - **Start** ([`start_blast`]): the blast's start plays cast motion 0x22
//!   with the setup's power id; when that motion starts, the caster's cast
//!   hook becomes the blast's, which answers −1 (it never keeps another cast
//!   from starting). A refused motion leaves the caster without a hook, and
//!   the blast still charges.
//! - **The hook** ([`hook_called`]): any call of the hook while the blast
//!   charges makes it fizzle. A later cast's start calls it (it replaces the
//!   hook and the power id), and so does the body tick when the release flag
//!   is set (pressing the blast again), which then ends the cast: the hook
//!   and power id go and the flag clears.
//! - **Each tick** (after the body tick): the blast reads its view from the
//!   caster's right hand, its facing and its aim point (the AI's target's
//!   chest, else the player's cursor), pays its drain from the caster's
//!   focus and ticks ([`Blast::tick`]); a thrown blast strikes the other
//!   bodies' limbs and leaves the scene when [`BlastEvents::removed`] says so.
//! - **Strikes** ([`strike_bodies`]): each live thrown chunk meets a limb
//!   sphere within its contact reach (`reach·scale + margin + 10`); the
//!   limb takes [`limb_push`]'s impulse at the sphere's surface point
//!   toward the chunk, the chunk moves and keeps [`contact_weight`] of its
//!   weight, and a character struck takes [`character_knock`] of its
//!   gathered impulse: the energy, and a knockback step along the knock.
//!   [inf: the original's mesh and sphere tests that find a contact are not
//!   in the notes; this sphere test, the body's mass as the sum of its
//!   parts' masses, and the knock as a knockback step are Exarust's.]
//!
//! Not wired: the alert, the sounds, the light, the stun, the hostility and
//! the recoil a struck character takes, the simulated objects' points, the
//! level's rebound and the draw. Another motion replacing the cast motion
//! calls the hook (`cast::call_motion_hooks`), which fizzles a charging
//! blast.
//!
//! Evidence: the blast's cast, creator, start and hook, the cast-motion
//! start, the body tick's release, the spend and the cast end, read from
//! `Exanima.exe`.

use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use super::barricade::WAKE_TICKS;
use super::cast::PowerCaster;
use super::casting::{self, CastBody, CastHook};
use super::force::{BLAST_MOTION, BlastSetup};
use super::live::EffectDraws;
use crate::effects::blast::{
    Blast, BlastChunk, BlastView, CHARGING, THROWN, character_knock, contact_reach, contact_weight,
    limb_push, limb_terms,
};
use crate::player::CursorGround;

/// Centimetres a metre.
const CM: f32 = 100.0;
const CHEST: usize = 9;
const RIGHT_HAND: usize = 17;
/// The blast's hook: −1 to every event.
pub const BLAST_HOOK: CastHook = CastHook::NEVER_KEEPS;
/// The extra reach a limb is tested with.
const LIMB_EXTRA: f32 = 10.0;
/// The aim point a caster with no target and no cursor looks at, cm ahead.
const AHEAD: f32 = 600.0;

/// A blast in the scene.
#[derive(Component)]
pub struct LiveBlast {
    pub blast: Blast,
    pub caster: Entity,
    /// The body the AI aims at; the player aims at the cursor.
    pub target: Option<Entity>,
    /// The caster's cast hook is this blast's.
    pub hooked: bool,
}

/// The blast's start on its caster: cast motion 0x22 with the setup's power
/// id, and the hook when the motion starts.
pub fn start_blast(body: &mut CastBody, setup: &BlastSetup) -> bool {
    let started = casting::start_cast(body, BLAST_MOTION, setup.power);
    if started {
        body.cast_hook = Some(BLAST_HOOK);
    }
    started
}

/// Whether the caster called the blast's hook since the last tick: another
/// cast's start replaced it, or the release flag is set, which also ends
/// the cast. Either way the hook is no longer the blast's.
pub fn hook_called(live: &mut LiveBlast, body: &mut CastBody) -> bool {
    if !live.hooked {
        return false;
    }
    if body.casting != live.blast.setup.power || body.cast_hook != Some(BLAST_HOOK) {
        live.hooked = false;
        return true;
    }
    if body.release {
        body.release = false;
        body.cast_hook = None;
        body.casting = 0;
        live.hooked = false;
        return true;
    }
    false
}

/// The world place (cm) of a point given in a part's space (cm).
fn part_point(body: &Ragdoll, bone: usize, local_cm: Vec3) -> Option<Vec3> {
    let part = body.parts.iter().find(|p| p.bone == bone)?;
    Some(part.transform().transform_point3(local_cm / CM) * CM)
}

/// The view a blast reads from its caster: the right hand's part (the
/// hand's node and its place), the facing and the aim point.
pub fn blast_view(caster: &Ragdoll, facing: Vec3, target: Option<Vec3>) -> Option<BlastView> {
    let hand = part_point(caster, RIGHT_HAND, Vec3::ZERO)?;
    Some(BlastView {
        target: target.unwrap_or(hand + facing * AHEAD),
        hand_node: hand,
        hand,
        facing,
    })
}

/// A body's mass: its parts' masses summed.
pub(crate) fn body_mass(body: &Ragdoll) -> f32 {
    body.parts
        .iter()
        .filter(|p| p.inverse_mass > 0.0)
        .map(|p| 1.0 / p.inverse_mass)
        .sum()
}

/// One tick of a thrown blast's chunks against `body`: each live chunk
/// pushes the limb spheres it reaches. Returns the impulse a character
/// gathered, `None` when nothing was struck.
pub fn strike_bodies(blast: &mut Blast, body: &mut Ragdoll) -> Option<Vec3> {
    let alive: Vec<bool> = blast.lives.iter().map(|l| l.ticks >= 1).collect();
    strike_chunks(&mut blast.chunks, &alive, &blast.setup, body)
}

/// The chunks marked alive against `body`'s limb spheres, as
/// [`strike_bodies`] does for a blast.
pub fn strike_chunks(
    chunks: &mut [BlastChunk],
    alive: &[bool],
    setup: &BlastSetup,
    body: &mut Ragdoll,
) -> Option<Vec3> {
    let character = body.drive.is_some();
    let mass = body_mass(body);
    let setup = *setup;
    let mut gathered = Vec3::ZERO;
    let mut struck = false;
    for (index, chunk) in chunks.iter_mut().enumerate() {
        if !alive.get(index).copied().unwrap_or(false) {
            continue;
        }
        for k in 0..body.parts.len() {
            let part = &body.parts[k];
            let inverse = part.inverse_mass;
            let (share, scale, push) = limb_terms(&setup, character, inverse, mass);
            let reach = contact_reach(chunk, scale, setup.margin, LIMB_EXTRA);
            let world = part.transform();
            let here: Vec3 = part.positions.iter().copied().sum::<Vec3>() * 0.25;
            let before: Vec3 = part.previous.iter().copied().sum::<Vec3>() * 0.25;
            let velocity = (here - before) * CM;
            let spheres: Vec<(Vec3, f32)> = part
                .spheres
                .iter()
                .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
                .collect();
            for (s, (centre, radius)) in spheres.into_iter().enumerate() {
                let d = chunk.at - centre;
                let distance = d.length();
                if distance >= reach + radius || distance <= 0.0 || distance.is_nan() {
                    continue;
                }
                let point = centre + d * (radius / distance);
                let Some(hit) = limb_push(chunk, point, velocity, inverse, share, push) else {
                    continue;
                };
                chunk.at = hit.at;
                if let Some(impulse) = hit.impulse {
                    body.push_sphere(k, s, impulse / CM);
                    if character {
                        gathered += impulse;
                    }
                }
                let (weight, wear) = contact_weight(chunk.weight, setup.piercing, character);
                chunk.weight = weight;
                if 0.0 < wear {
                    chunk.wear = wear;
                }
                struck = true;
            }
        }
    }
    struck.then_some(gathered)
}

/// Ticks every blast in the scene.
#[allow(clippy::type_complexity)]
pub(crate) fn tick_blasts(
    mut commands: Commands,
    mut blasts: Query<(Entity, &mut LiveBlast)>,
    mut casters: Query<(
        &mut PowerCaster,
        Option<&Locomotor>,
        Option<&GlobalTransform>,
    )>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    places: Query<&GlobalTransform>,
    cursor: Option<Res<CursorGround>>,
    mut draws: ResMut<EffectDraws>,
) {
    for (entity, mut live) in &mut blasts {
        let live = &mut *live;
        let target = live
            .target
            .and_then(|t| {
                bodies
                    .get(t)
                    .ok()
                    .and_then(|(_, body)| part_point(body, CHEST, Vec3::ZERO))
                    .or_else(|| places.get(t).ok().map(|p| p.translation() * CM))
            })
            .or_else(|| {
                live.target
                    .is_none()
                    .then(|| cursor.as_deref().and_then(|c| c.0).map(|p| p * CM))
                    .flatten()
            });
        let mut scratch = CastBody::default();
        let (body, facing) = match casters.get_mut(live.caster) {
            Ok((caster, locomotor, at)) => {
                let facing = match (locomotor, at) {
                    (Some(l), _) => Vec3::new(l.yaw.sin(), 0.0, l.yaw.cos()),
                    (None, Some(at)) => at.rotation() * Vec3::Z,
                    (None, None) => Vec3::Z,
                };
                (Some(caster), facing)
            }
            Err(_) => (None, Vec3::Z),
        };
        let view = bodies
            .get(live.caster)
            .ok()
            .and_then(|(_, body)| blast_view(body, facing, target));
        let mut caster = body;
        let cast_body = match caster.as_deref_mut() {
            Some(caster) => &mut caster.body,
            None => {
                live.blast.release();
                &mut scratch
            }
        };
        if hook_called(live, cast_body) {
            live.blast.release();
        }
        let Some(view) = view.or_else(|| {
            (live.blast.state == THROWN).then_some(BlastView {
                target: Vec3::ZERO,
                hand_node: Vec3::ZERO,
                hand: Vec3::ZERO,
                facing,
            })
        }) else {
            if live.hooked {
                cast_body.cast_hook = None;
                cast_body.casting = 0;
            }
            commands.entity(entity).despawn();
            continue;
        };
        let events = live.blast.tick(&view, cast_body, &mut draws.0);
        if live.blast.state != CHARGING {
            live.hooked = false;
        }
        if let Some(pitch) = events.released {
            debug!("blast {entity:?} thrown, release pitch {pitch:.3}");
        }
        if let Some(caster) = caster.as_deref_mut() {
            caster.sync_body();
        }
        if live.blast.state == THROWN {
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let Some(impulse) = strike_bodies(&mut live.blast, &mut body) else {
                    continue;
                };
                body.touch(WAKE_TICKS);
                let mass = body_mass(&body);
                let Some(drive) = body.drive.as_mut() else {
                    continue;
                };
                let knock = character_knock(mass, drive.energy, impulse);
                drive.energy = knock.energy;
                if let Some((direction, size)) = knock.knock {
                    let flat = Vec3::new(direction.x, 0.0, direction.z);
                    body.begin_knockback(flat * (size / CM));
                }
            }
        }
        if events.removed {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::blast::FIZZLING;
    use crate::thaum::force::{BLAST_SETUP, BOLT_SETUP};

    #[test]
    fn the_start_plays_the_motion_and_takes_the_hook() {
        let mut body = CastBody::default();
        assert!(start_blast(&mut body, &BOLT_SETUP));
        assert_eq!(body.casting, 0x202);
        assert_eq!(body.cast_motion, Some(BLAST_MOTION));
        assert_eq!(body.cast_hook, Some(BLAST_HOOK));
        // The hook answers −1, so a later cast still starts.
        assert!(casting::start_cast(&mut body, 0x23, 0x401));
        // A running cast whose hook answers 0 refuses the blast's motion.
        let mut busy = CastBody {
            cast_hook: Some(CastHook { keeps: 0x20 }),
            casting: 0x401,
            ..CastBody::default()
        };
        assert!(!start_blast(&mut busy, &BLAST_SETUP));
        assert_eq!(
            (busy.casting, busy.cast_hook),
            (0x401, Some(CastHook { keeps: 0x20 }))
        );
    }

    fn live(body: &mut CastBody) -> LiveBlast {
        let hooked = start_blast(body, &BLAST_SETUP);
        let view = BlastView {
            target: Vec3::new(0.0, 150.0, 600.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
        };
        LiveBlast {
            blast: Blast::new(
                BLAST_SETUP,
                0.1,
                12,
                &view,
                &mut crate::arena::Mt19937::new(1),
            ),
            caster: Entity::PLACEHOLDER,
            target: None,
            hooked,
        }
    }

    #[test]
    fn a_later_cast_or_the_release_calls_the_hook() {
        let mut body = CastBody::default();
        let mut blast = live(&mut body);
        assert!(!hook_called(&mut blast, &mut body));
        assert!(casting::start_cast(&mut body, 0x23, 0x401));
        assert!(hook_called(&mut blast, &mut body));
        assert!(!blast.hooked);
        assert!(!hook_called(&mut blast, &mut body));

        let mut body = CastBody::default();
        let mut blast = live(&mut body);
        body.release = true;
        assert!(hook_called(&mut blast, &mut body));
        assert_eq!(
            (body.casting, body.cast_hook, body.release),
            (0, None, false)
        );
        blast.blast.release();
        assert_eq!(blast.blast.state, FIZZLING);
    }
}
