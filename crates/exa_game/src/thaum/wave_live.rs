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
//!   caster's hook and power id. Thrown, its live chunks take the blast's
//!   contact slots, which the wave's class shares: the level's static
//!   meshes and the cloths after its tick, and each other body part by
//!   part in the next tick's first pass ([`super::pre_pass`],
//!   [`super::blast_live::strike_part`]), the wave's setup giving the
//!   pushes; a struck character takes the blast's knock, counter, concern
//!   and loosened pins, and a body met is woken. The entity leaves the
//!   scene when the wave is spent or fizzled.
//!
//! The throw alerts the caster's AI's known characters within 1800 cm as
//! the blast's does ([`super::blast_live::alert_known`]; the aim is cleared
//! first, so only the distance weighs it).
//!
//! The sounds are the blast's (the wave shares its frame): the charge loop
//! `thmfrc_cu01` from the start, `length·0.8·r` into its buffer in double
//! precision (`r` the runtime stream's draw after the ring's), and the
//! release `thmfrc_wav` at the pitch the throw draws
//! ([`super::blast_sound::sound_blasts`]).
//!
//! The draw and the light are the blast's (the wave's class shares the
//! blast's frame and its creator makes the blast's emissive): the strip is
//! built from the wave's live chunks in every state ([`draw_waves`]), and
//! the multi-point GI light takes the life records' light points the frame
//! writes ([`wave_lights`], kept on the frame's [`WaveLights`]); both read
//! the wave as a blast ([`wave_as_blast`]).
//!
//! Not wired: the stun; and the contact's wear, which the wave's chunks do
//! not keep.
//!
//! Evidence: the wave's start and tick, which share the blast's hook and
//! contact handlers, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_physics::Physics;

use super::blast_live::{BLAST_HOOK, ChunkSet, STATIC_SCALE, stir_cloth, strike_level};
use super::cast::PowerCaster;
use super::casting::{self, CastBody, spend_tick};
use super::force::{BlastSetup, WAVE_SETUP};
use super::live::EffectDraws;
use crate::effects::blast::{Blast, BlastChunk, ChunkLife};
use crate::effects::force_wave::LIFE_TICKS;
use crate::effects::force_wave::{Wave, WaveChunk, WaveEvent, WaveState};

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The wave's power id and its cast motion.
pub const WAVE: u32 = 0x204;
pub const WAVE_MOTION: u32 = 0x24;

/// A wave in the scene.
#[derive(Component, Clone, Debug)]
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
    /// The runtime stream's draw the charge loop's start takes (after the
    /// ring's draws), and the release's pitch once thrown.
    pub charge_roll: f64,
    pub release_pitch: Option<f32>,
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
    let wave = Wave::new(root, facing, tick_ms, &mut draws.0);
    // The charge loop's start, `length·0.8·r` into its buffer, draws after
    // the ring.
    let charge_roll = draws.0.unit();
    LiveWave {
        wave,
        setup: WAVE_SETUP,
        caster,
        drain: cost * seconds,
        hooked,
        generation,
        charge_roll,
        release_pitch: None,
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

/// The light points the shared frame writes on the wave's life records
/// (world cm, one a chunk), kept by the frame only: the simulation's wave
/// never reads them.
#[derive(Component, Debug, Clone, Default)]
pub struct WaveLights(pub Vec<Vec3>);

/// The wave as the blast's frame and emissive read it: its chunks, their
/// life records (ticks, draws and the frame's `lights`), its state, charge,
/// trail step, life length and setup.
pub fn wave_as_blast(live: &LiveWave, lights: &[Vec3]) -> Blast {
    let wave = &live.wave;
    Blast {
        setup: live.setup,
        drain: live.drain,
        state: wave.state as i32,
        charge: wave.charge,
        rate: wave.rate,
        life_ticks: LIFE_TICKS,
        aim: wave.aim,
        frame: crate::effects::blast::Frame {
            rows: [Vec3::X, Vec3::Y, Vec3::Z],
            origin: Vec3::ZERO,
        },
        chunks: wave.chunks.iter().map(as_blast_chunk).collect(),
        lives: wave
            .chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| ChunkLife {
                light_at: lights.get(i).copied().unwrap_or(Vec3::ZERO),
                draws: chunk.draws,
                ticks: chunk.life,
            })
            .collect(),
        flags: wave.flags,
        trail_step: wave.trail_step,
        centre: wave.centre,
        half: wave.half,
        bound_radius: wave.bound_radius,
        linked: true,
    }
}

/// Draws every wave's strip as the blast's frame builds it
/// ([`super::blast_live::draw_blast_ribbon`]).
#[allow(clippy::type_complexity)]
pub(crate) fn draw_waves(
    mut commands: Commands,
    waves: Query<(
        Entity,
        &LiveWave,
        Option<(
            &super::live::EffectMesh,
            &super::effect_draw::RibbonMaterial,
        )>,
    )>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    noise: Res<super::effect_draw::ForceNoise>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::force_effects::ForceRibbonMaterial>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let view = super::blast_live::camera_frame(camera);
    let alpha = time.overstep_fraction();
    for (entity, live, drawn) in &waves {
        let blast = wave_as_blast(live, &[]);
        super::blast_live::draw_blast_ribbon(
            &mut commands,
            entity,
            &blast,
            drawn,
            alpha,
            &view,
            &noise,
            &mut meshes,
            &mut materials,
        );
    }
}

/// The waves' lights, as the blasts' ([`super::effect_light::blast_lights`]):
/// each wave registers the blast's multi-point shape and refreshes it from
/// this frame's light points; the frame that builds nothing frees it.
#[allow(clippy::type_complexity)]
pub(crate) fn wave_lights(
    mut commands: Commands,
    mut waves: Query<(
        Entity,
        &LiveWave,
        Option<&mut WaveLights>,
        Option<&super::effect_light::EffectLight>,
        Has<super::effect_light::EffectLightFreed>,
    )>,
    mut shapes: Query<(
        &mut exa_render::gi::GiShape,
        &mut exa_render::gi::ShapeVolume,
    )>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let view = super::blast_live::camera_frame(camera);
    let alpha = time.overstep_fraction();
    for (entity, live, lights, light, freed) in &mut waves {
        if freed {
            continue;
        }
        let kept = lights.as_deref().map_or(&[][..], |l| &l.0[..]);
        let mut blast = wave_as_blast(live, kept);
        if blast.builds_nothing() {
            if let Some(light) = light {
                commands.entity(light.0).despawn();
                debug!("wave light freed for {entity:?}");
            }
            commands
                .entity(entity)
                .remove::<super::effect_light::EffectLight>()
                .insert(super::effect_light::EffectLightFreed);
            continue;
        }
        blast.frame_lights(alpha, &view);
        let points: Vec<Vec3> = blast.lives.iter().map(|life| life.light_at).collect();
        match lights {
            Some(mut lights) => lights.0 = points,
            None => {
                commands.entity(entity).insert(WaveLights(points));
            }
        }
        if let Some(placed) =
            super::effect_light::blast_shape(&blast, super::effect_light::GI_LIGHT_SCALE)
        {
            super::effect_light::place_light(
                &mut commands,
                entity,
                light,
                &mut shapes,
                placed,
                "wave light",
            );
        }
    }
}

/// The wave's chunks as the blast's contacts walk them, and which live:
/// `chunks` and `alive` are the caller's scratch buffers.
pub(crate) fn wave_chunks(live: &LiveWave, chunks: &mut Vec<BlastChunk>, alive: &mut Vec<bool>) {
    chunks.clear();
    chunks.extend(live.wave.chunks.iter().map(as_blast_chunk));
    alive.clear();
    alive.extend(live.wave.chunks.iter().map(|c| c.life >= 1));
}

/// The contacts' moves and weights back on the wave's chunks (the wave
/// keeps no wear).
pub(crate) fn take_back(live: &mut LiveWave, chunks: &[BlastChunk]) {
    for (wave, blast) in live.wave.chunks.iter_mut().zip(chunks) {
        wave.at = blast.at;
        wave.weight = blast.weight;
    }
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
    if let WaveEvent::Release { pitch, .. } = event {
        live.hooked = false;
        body.cast_hook = None;
        body.casting = 0;
        // The release sound plays at the drawn pitch
        // ([`super::blast_sound::sound_blasts`]).
        live.release_pitch = Some(pitch);
    }
    event
}

/// Ticks every wave in the scene.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn tick_waves(
    mut commands: Commands,
    mut waves: Query<(Entity, &mut LiveWave)>,
    mut casters: Query<(&mut PowerCaster, Option<&Locomotor>)>,
    bodies: Query<(Entity, &Ragdoll)>,
    mut draws: ResMut<EffectDraws>,
    mut chunks: Local<Vec<BlastChunk>>,
    mut alive: Local<Vec<bool>>,
    mut triangles: Local<Vec<[Vec3; 3]>>,
    (physics, statics, places): (
        Option<Res<Physics>>,
        Option<Res<exa_physics::StaticColliders>>,
        Query<&GlobalTransform>,
    ),
    (mut combatants, npcs, mut cloths): (
        Query<&mut Combatant>,
        Query<&Npc>,
        Query<&mut exa_world::cloth::ClothInstance>,
    ),
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
        if let WaveEvent::Release { .. } = event {
            // The throw alerts the caster's AI's known characters as the
            // blast's does, its aim already cleared.
            super::blast_live::alert_known(
                live.caster,
                live.wave.aim,
                &bodies,
                &mut combatants,
                &npcs,
            );
        }
        if live.wave.state == WaveState::Thrown {
            // The blast's static and object slots, as the sector's stages
            // run them after the wave's step: the level, then the simulated
            // objects. The bodies meet it in the next tick's first pass
            // (`super::pre_pass`).
            wave_chunks(live, &mut chunks, &mut alive);
            let mut set = ChunkSet {
                chunks: &mut chunks,
                alive: &alive,
                setup: live.setup,
                centre: live.wave.centre,
                radius: live.wave.bound_radius,
            };
            let mut scale = 0.0;
            if let (Some(physics), Some(statics)) = (physics.as_deref(), statics.as_deref()) {
                let hits = strike_level(&mut set, statics, physics, &places, &mut triangles);
                if 0 < hits {
                    debug!("wave {entity:?} meets the level: {hits} chunk contacts");
                }
                scale = STATIC_SCALE;
            }
            for mut cloth in &mut cloths {
                if stir_cloth(&set, scale, &mut cloth) {
                    debug!("wave {entity:?} stirs a cloth");
                }
            }
            take_back(live, &chunks);
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

    /// The charge loop's start takes the runtime stream's draw after the
    /// ring's; a charging wave has no release yet.
    #[test]
    fn the_charge_loop_draws_after_the_ring() {
        let mut body = CastBody::default();
        let wave = live(&mut body);
        let mut stream = Mt19937::new(1);
        Wave::new(Vec3::ZERO, Vec3::Z, 12, &mut stream);
        assert_eq!(wave.charge_roll, stream.unit());
        assert_eq!(wave.release_pitch, None);
        // The blast's frame levels its voices: charging, the loop at 0.4
        // times the charge's share and no release.
        assert_eq!(
            crate::effects::blast::voice_levels(0.15, false, 1.0),
            (Some(0.2), None)
        );
        assert_eq!(
            crate::effects::blast::voice_levels(0.5, true, 0.7),
            (Some(((0.9 - 0.5f64) * 0.444) as f32), Some(0.7))
        );
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
        // The release sound's pitch is the throw's draw, `0.9 + 0.2·r`.
        let pitch = wave.release_pitch.expect("the release sounds");
        assert!((0.9..=1.1).contains(&pitch), "{pitch}");
        assert_eq!((body.casting, body.cast_hook), (0, None));
        // The throw is not the cast end: the cast motion plays on.
        assert_eq!(body.condition, casting::CASTING);
        assert!(!casting::take_stance_return(&mut body));
        assert!(body.spent > 0.0);
    }

    /// The shared frame draws the wave as the blast's strip from its first
    /// charged tick, in the charge's colour, and lights it with the blast's
    /// multi-point shape; a charging blast draws its strip too.
    #[test]
    fn a_charging_wave_and_blast_draw_their_strips_and_the_wave_lights() {
        use crate::effects::blast::{BlastView, CHARGING};
        use crate::thaum::blast_live::{LiveBlast, draw_blasts};
        use crate::thaum::effect_draw::{ForceNoise, RibbonMaterial};
        use crate::thaum::effect_light::{EffectLight, drop_orphan_lights};
        use crate::thaum::force::BLAST_SETUP;
        use crate::thaum::live::EffectMesh;
        use bevy::asset::AssetPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .add_plugins(exa_render::gi_shape::GiShapePlugin)
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<exa_render::force_effects::ForceRibbonMaterial>()
            .init_resource::<Time<Fixed>>()
            .add_systems(
                Update,
                (draw_blasts, draw_waves, wave_lights, drop_orphan_lights).chain(),
            );
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let mut noise = ForceNoise::default();
        noise.image = Some(image);
        app.insert_resource(noise);
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::new(0.0, 1.7, -3.0)),
        ));
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let mut wave = live(&mut body);
        let mut draws = EffectDraws(Mt19937::new(2));
        for _ in 0..3 {
            tick_wave(&mut wave, &mut body, Vec3::ZERO, &mut draws);
        }
        assert_eq!(wave.wave.state, WaveState::Charging);
        let charge = wave.wave.charge;
        let wave = app.world_mut().spawn(wave).id();
        let view = BlastView {
            target: Vec3::new(0.0, 150.0, 1000.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
        };
        let mut blast = Blast::new(BLAST_SETUP, 0.1, 12, &view, &mut Mt19937::new(5));
        let mut caster = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        for _ in 0..3 {
            blast.tick(&view, &mut caster, false, &mut Mt19937::new(6));
        }
        assert_eq!(blast.state, CHARGING);
        let blast = app
            .world_mut()
            .spawn(LiveBlast {
                blast,
                caster: Entity::PLACEHOLDER,
                target: None,
                hooked: false,
                generation: 0,
                contact_scale: 0.0,
                sounds: Default::default(),
            })
            .id();
        app.update();
        app.update();
        for effect in [wave, blast] {
            let handle = app
                .world()
                .get::<EffectMesh>(effect)
                .expect("the strip is drawn while charging")
                .0
                .clone();
            assert!(app.world().get::<RibbonMaterial>(effect).is_some());
            let meshes = app.world().resource::<Assets<Mesh>>();
            let mesh = meshes.get(&handle).expect("a mesh");
            assert!(mesh.count_vertices() > 0);
        }
        // Charging, each chunk's colour carries the charge byte.
        let mirror = wave_as_blast(app.world().get::<LiveWave>(wave).unwrap(), &[]);
        assert_eq!(
            mirror.chunk_colour(0)[2],
            (charge * 255.0).round_ties_even() as u8
        );
        // The wave's light is the blast's multi-point shape over its 32
        // light points.
        let light = app.world().get::<EffectLight>(wave).expect("lit").0;
        let shape = app.world().get::<exa_render::gi::GiShape>(light).unwrap();
        assert_eq!(shape.points.len(), 32);
        assert_eq!(app.world().get::<WaveLights>(wave).unwrap().0.len(), 32);
        assert!(
            app.world()
                .get::<WaveLights>(wave)
                .unwrap()
                .0
                .iter()
                .all(|p| p.is_finite())
        );
    }
}
