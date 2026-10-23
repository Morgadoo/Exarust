//! The force wave in the running game: the effect entity a wave cast makes,
//! its ring of chunks charging round the caster and then thrown outward.
//!
//! - **Start** ([`start_wave`]): the wave's start plays cast motion 0x24
//!   with the power id 0x204; when that motion starts the caster's cast
//!   hook becomes the blast's ([`BLAST_HOOK`]), which answers −1 and
//!   fizzles the wave when it is called while it charges. The drain a tick
//!   is the power's cost times the tick in seconds; the ring stands round
//!   the caster's body root and aims along its facing ([`Wave::new`]).
//! - **The hook**: as the blast's ([`hook_called`]): a later cast's start,
//!   the player's buttons or the body tick's release calls it.
//! - **Each tick** ([`Wave::tick_paying`]): a charging wave pays its drain
//!   once its chunks are placed; a shortfall calls the hook (it fizzles)
//!   and ends the cast. The charge passing 1 throws the ring and clears the
//!   caster's hook and power id. Thrown, its live chunks strike the other
//!   bodies' limbs through the blast's own contact ([`strike_chunks`]), the
//!   wave's setup giving the pushes, and a struck character takes the
//!   blast's knock. The entity leaves the scene when the wave is spent or
//!   fizzled.
//!
//! Not wired, as for the blast: the alert at the throw (reach 1200 cm), the
//! sounds, the light, the stun and the draw; and the contact's wear, which
//! the wave's chunks do not keep.
//!
//! Evidence: the wave's start and tick, which share the blast's hook and
//! contact handlers, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use super::barricade::WAKE_TICKS;
use super::blast_live::{BLAST_HOOK, LimbSphere, body_mass, strike_chunks};
use super::cast::PowerCaster;
use super::casting::{self, CastBody, spend_tick};
use super::force::{BlastSetup, WAVE_SETUP};
use super::live::EffectDraws;
use crate::effects::blast::{BlastChunk, character_knock};
use crate::effects::force_wave::{Wave, WaveChunk, WaveEvent, WaveState};

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The wave's power id and its cast motion.
pub const WAVE: u32 = 0x204;
pub const WAVE_MOTION: u32 = 0x24;

/// A wave in the scene.
#[derive(Component)]
pub struct LiveWave {
    pub wave: Wave,
    pub setup: BlastSetup,
    pub caster: Entity,
    /// The focus it pays a tick while charging.
    pub drain: f32,
    /// The caster's cast hook is this wave's.
    pub hooked: bool,
    /// The cast generation whose hook the wave took.
    pub generation: u64,
}

/// The wave's start on its caster: cast motion 0x24 with the power id, and
/// the blast's hook when the motion starts.
pub fn start_wave(body: &mut CastBody) -> bool {
    let started = casting::start_cast(body, WAVE_MOTION, WAVE);
    if started {
        body.cast_hook = Some(BLAST_HOOK);
    }
    started
}

/// A wave for `caster`, its ring round `root` (cm) aiming along `facing`.
pub fn new_wave(
    body: &mut CastBody,
    caster: Entity,
    cost: f32,
    root: Vec3,
    facing: Vec3,
    tick_ms: u32,
    draws: &mut EffectDraws,
) -> LiveWave {
    let hooked = start_wave(body);
    let generation = body.cast_generation;
    let seconds = (f64::from(tick_ms) * 0.001) as f32;
    LiveWave {
        wave: Wave::new(root, facing, tick_ms, &mut draws.0),
        setup: WAVE_SETUP,
        caster,
        drain: cost * seconds,
        hooked,
        generation,
    }
}

/// Whether the caster called the wave's hook since the last tick: it is no
/// longer the wave's (a call answers −1 and ends the cast, or a later
/// cast's start replaced it). A charging wave then fizzles.
pub fn hook_called(live: &mut LiveWave, body: &mut CastBody) -> bool {
    if !live.hooked {
        return false;
    }
    let called = body.cast_generation != live.generation
        || body.casting != WAVE
        || body.cast_hook != Some(BLAST_HOOK);
    if called {
        live.hooked = false;
        fizzle(&mut live.wave);
    }
    called
}

/// The blast's hook on a wave: a charging one fizzles.
fn fizzle(wave: &mut Wave) {
    if wave.state == WaveState::Charging {
        wave.state = WaveState::Fizzling;
    }
}

fn as_blast_chunk(chunk: &WaveChunk) -> BlastChunk {
    BlastChunk {
        at: chunk.at,
        previous: chunk.previous,
        velocity: chunk.velocity,
        offset: chunk.offset,
        direction: chunk.direction,
        weight: chunk.weight,
        head: chunk.head,
        trail: chunk.trail,
        reach: chunk.reach,
        radius: chunk.radius,
        previous_radius: chunk.previous_radius,
        head_radius: chunk.head_radius,
        wear: 0.0,
    }
}

/// One tick of a thrown wave's live chunks against `body`. Returns the
/// impulse a character gathered, `None` when nothing was struck. `chunks`,
/// `alive` and `limbs` are the caller's scratch buffers.
pub(crate) fn strike_wave(
    live: &mut LiveWave,
    body: &mut Ragdoll,
    chunks: &mut Vec<BlastChunk>,
    alive: &mut Vec<bool>,
    limbs: &mut Vec<LimbSphere>,
) -> Option<Vec3> {
    chunks.clear();
    chunks.extend(live.wave.chunks.iter().map(as_blast_chunk));
    alive.clear();
    alive.extend(live.wave.chunks.iter().map(|c| c.life >= 1));
    let struck = strike_chunks(chunks, alive, &live.setup, body, limbs);
    for (wave, blast) in live.wave.chunks.iter_mut().zip(chunks.iter()) {
        wave.at = blast.at;
        wave.weight = blast.weight;
    }
    struck
}

fn root_of(body: &Ragdoll) -> Option<Vec3> {
    body.parts
        .first()
        .map(|part| Vec3::from(part.transform().translation) * CM)
}

/// One tick of a wave on its caster's body: the hook, then the wave's tick
/// paying its drain. The tick's event.
pub fn tick_wave(
    live: &mut LiveWave,
    body: &mut CastBody,
    root: Vec3,
    draws: &mut EffectDraws,
) -> WaveEvent {
    hook_called(live, body);
    let drain = live.drain;
    let hooked = live.hooked;
    let mut short = false;
    let event = live.wave.tick_paying(root, &mut draws.0, || {
        short = spend_tick(body, drain);
        short && hooked
    });
    if short {
        live.hooked = false;
        casting::end_cast(body);
    }
    // The throw clears the hook and the power id itself, not through the
    // cast end: the cast motion plays on.
    if let WaveEvent::Release { .. } = event {
        live.hooked = false;
        body.cast_hook = None;
        body.casting = 0;
    }
    event
}

/// Ticks every wave in the scene.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn tick_waves(
    mut commands: Commands,
    mut waves: Query<(Entity, &mut LiveWave)>,
    mut casters: Query<(&mut PowerCaster, Option<&Locomotor>)>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    mut draws: ResMut<EffectDraws>,
    mut chunks: Local<Vec<BlastChunk>>,
    mut alive: Local<Vec<bool>>,
    mut limbs: Local<Vec<LimbSphere>>,
) {
    for (entity, mut live) in &mut waves {
        let live = &mut *live;
        let root = bodies
            .get(live.caster)
            .ok()
            .and_then(|(_, body)| root_of(body));
        let mut scratch = CastBody::default();
        let mut caster = casters.get_mut(live.caster).ok();
        let body = match caster.as_mut() {
            Some((caster, _)) => &mut caster.body,
            None => {
                live.hooked = false;
                fizzle(&mut live.wave);
                &mut scratch
            }
        };
        let root = root.unwrap_or_else(|| {
            live.wave
                .chunks
                .first()
                .map_or(Vec3::ZERO, |c| c.at - c.offset)
        });
        let event = tick_wave(live, body, root, &mut draws);
        if let Some((caster, _)) = caster.as_mut() {
            caster.sync_body();
        }
        if live.wave.state == WaveState::Thrown {
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let Some(impulse) =
                    strike_wave(live, &mut body, &mut chunks, &mut alive, &mut limbs)
                else {
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
        if event == WaveEvent::Removed {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;

    fn live(body: &mut CastBody) -> LiveWave {
        let mut draws = EffectDraws(Mt19937::new(1));
        new_wave(
            body,
            Entity::PLACEHOLDER,
            0.2,
            Vec3::ZERO,
            Vec3::Z,
            12,
            &mut draws,
        )
    }

    #[test]
    fn the_start_plays_the_motion_and_takes_the_blasts_hook() {
        let mut body = CastBody::default();
        let wave = live(&mut body);
        assert!(wave.hooked);
        assert_eq!(body.casting, WAVE);
        assert_eq!(body.cast_motion, Some(WAVE_MOTION));
        assert_eq!(body.cast_hook, Some(BLAST_HOOK));
        assert_eq!(wave.drain, 0.2 * 0.012);
        assert_eq!(wave.wave.chunks.len(), 32);
    }

    #[test]
    fn a_later_cast_fizzles_it_and_a_shortfall_ends_the_cast() {
        let mut body = CastBody::default();
        let mut wave = live(&mut body);
        let mut draws = EffectDraws(Mt19937::new(2));
        body.focus = 1.0;
        assert!(casting::start_cast(&mut body, 0x22, 0x201));
        // Fizzling from no charge, the same tick's loss fizzles it out.
        assert_eq!(
            tick_wave(&mut wave, &mut body, Vec3::ZERO, &mut draws),
            WaveEvent::Removed
        );
        assert_eq!(wave.wave.state, WaveState::Fizzled);

        let mut body = CastBody {
            focus: 0.0,
            ..CastBody::default()
        };
        let mut wave = live(&mut body);
        // No focus: the first payment falls short, the hook fizzles the
        // wave and the cast ends.
        tick_wave(&mut wave, &mut body, Vec3::ZERO, &mut draws);
        assert_eq!(wave.wave.state, WaveState::Fizzling);
        assert_eq!((body.casting, body.cast_hook), (0, None));
        assert_eq!(body.condition, 0);
        assert!(casting::take_stance_return(&mut body), "the stance return");
    }

    #[test]
    fn a_paid_wave_is_thrown_and_clears_the_hook() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let mut wave = live(&mut body);
        let mut draws = EffectDraws(Mt19937::new(2));
        let mut ticks = 0;
        while wave.wave.state == WaveState::Charging {
            tick_wave(&mut wave, &mut body, Vec3::ZERO, &mut draws);
            ticks += 1;
            assert!(ticks < 100);
        }
        assert_eq!(wave.wave.state, WaveState::Thrown);
        assert_eq!((body.casting, body.cast_hook), (0, None));
        // The throw is not the cast end: the cast motion plays on.
        assert_eq!(body.condition, casting::CASTING);
        assert!(!casting::take_stance_return(&mut body));
        assert!(body.spent > 0.0);
    }
}
