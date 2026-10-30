//! The scene weather's effects, live: the rain drops and their draw, the
//! rain loop, thunder, and the outdoor arenas' weather ambience (the wind
//! bed). The weather itself (the rain fading towards its target, the
//! lightning, the time of day) is `exa_world::scene_env`, ticked once a
//! frame by `exa_world::environment`; this module acts on what the tick
//! asks for, after it.
//!
//! - **Rain effect**: the original creates its rain effect with the first
//!   rain request, gives it the rain amount as its intensity on every tick
//!   the amount moves, and frees it when the rain falls to 0.01. While the
//!   amount is above 0.01 the effect follows the eye and steps its drops
//!   on the game clock (`effects::rain`), takes the scene's scratch colour,
//!   and draws its streaks (`exa_render::rain`). Its update's drift figure
//!   becomes the rain loop's volume and its fall figure the loop's pitch;
//!   the next tick reads them, as the original's tick reads the values the
//!   last drawn frame left.
//! - **Rain loop**: the first request also takes a voice whose flag word is
//!   set to listener-relative alone. Each tick writes the loop's volume and
//!   pitch into it; a voice that is not playing starts the `rainfall`
//!   buffer at the listener (position zero, looping) once the volume is
//!   above 0.01, and a playing one is asked to stop below 0.01. The voice
//!   is the world bus's, so the distance model leaves it at full gain.
//! - **Thunder**: a clap plays one variant of the `Thunder` set (`thunder01`
//!   to `04`) at gain `4·attenuation + 0.1` and pitch `0.9 + 0.3·r`, as a
//!   listener-relative one-shot placed at the strike's position. That
//!   position already holds the view's own world position (the original
//!   moves the strike next to the camera, then plays it relative to the
//!   listener), so the clap is heard from the listener plus the view's
//!   offset from the world's origin, quieter the further the view is from
//!   it. Reproduced as the original does it.
//! - **Arena ambience**: when an outdoor arena's scene is set up (once its
//!   preset has drawn the day), the ambience follows the weather: a rain
//!   target above 0.01 clears it (the rain loop takes over), a dark scene
//!   plays the night bed, a calm arena the day bed, and the other arenas
//!   the wind bed `NtrW` at `wind² · 0.2` gain and `0.6 + 0.5·wind` pitch
//!   when the wind is above 0.1 (`exa_audio::arena_ambience`). The indoor
//!   arenas clear it.
//!
//! Evidence: the scene environment's rain setter, its weather tick and its
//! strike routine, the rain effect's intensity setter, update and frame
//! routine with its material, the audio environment's voice allocator,
//! voice start and set player, and the arena setup's ambience branch, read
//! from `Exanima.exe` (REA, Ghidra). Not reproduced: the original draws the
//! thunder set's variant and every other engine draw from one shared
//! generator; Exarust's sound bank keeps its own.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::pbr::ScreenSpaceTransmission;
use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, Voice, voice_flags};
use exa_render::rain::{RainMaterial, rain_mesh};
use exa_world::LevelRoot;
use exa_world::environment::{LiveEnvironment, SceneEnvRuntime};
use exa_world::scene_env::{RainSound, Thunder, WeatherEvents};

use crate::arena::Mt19937;
use crate::effects::rain::{Rain, box_rotation, box_scale};

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherEffects>().add_systems(
            PostUpdate,
            (rain_effect, weather_sounds, arena_weather_ambience)
                .chain()
                .after(LiveEnvironment),
        );
    }
}

/// The rain loop's buffer: `rainfall` (`Sound.rpk`).
pub const RAIN_SOUND: &str = "rainfall";
/// The thunder set's prefix: `thunder01` … `thunder04` (`Sound.rpk`).
pub const THUNDER_SET: &str = "Thunder";
/// The first arena index that takes the weather ambience: the outdoor
/// arenas (`exa_game::arena_match::ARENA_MAPS`).
pub const FIRST_OUTDOOR_ARENA: usize = 7;
/// Centimetres in a metre.
const CM_PER_M: f32 = 100.0;
/// The seed of the run-time library's generator the rain draws from (its
/// drop set and gust walk): the standard MT19937 default, the original's
/// own seeding not being traced.
const RAIN_RANDOM_SEED: u32 = 5489;

/// The streaks' entity.
#[derive(Component)]
struct RainStreaks;

/// The weather's live objects: the rain effect and its draw, the rain
/// loop's voice and the sounds it loaded.
#[derive(Resource)]
pub struct WeatherEffects {
    /// The rain effect's drops while it exists.
    pub rain: Option<Rain>,
    /// The run-time library's generator the rain draws from.
    random: Mt19937,
    /// The streaks' entity, mesh and material.
    streaks: Option<(Entity, Handle<Mesh>, Handle<RainMaterial>)>,
    /// The rain loop's voice, taken with the first rain request.
    pub rain_voice: Option<u32>,
    /// The `rainfall` buffer; `Some(None)` once a load failed. The original
    /// keeps it for the session.
    rain_buffer: Option<Option<u32>>,
    /// The thunder set, once loaded.
    thunder_set: Option<u32>,
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self {
            rain: None,
            random: Mt19937::new(RAIN_RANDOM_SEED),
            streaks: None,
            rain_voice: None,
            rain_buffer: None,
            thunder_set: None,
        }
    }
}

/// What the rain loop's voice did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopChange {
    None,
    Started,
    StopRequested,
}

/// One tick's rain loop over its voice, as the original's tick drives it:
/// the levels always (the model leaves the pitch alone and zeroes the
/// volume when it does not rain), a start of `buffer` at the listener when
/// the tick asks for one, a stop request when it asks for that. The start
/// raises the allocated and restart bits, then the looping bit; the voice
/// keeps its listener-relative bit from its creation.
pub fn drive_rain_voice(
    voice: &mut Voice,
    sound: &RainSound,
    events: &WeatherEvents,
    buffer: Option<u32>,
) -> LoopChange {
    voice.gain = sound.volume;
    voice.pitch = sound.pitch;
    if events.start_sound
        && voice.flags & voice_flags::ACTIVE == 0
        && let Some(buffer) = buffer
    {
        voice.flags |= voice_flags::ALLOCATED | voice_flags::RESTART;
        voice.buffer = buffer;
        voice.position = Vec3::ZERO;
        voice.flags |= voice_flags::LOOPING;
        return LoopChange::Started;
    }
    if events.stop_sound && voice.flags & voice_flags::ACTIVE != 0 {
        voice.flags |= voice_flags::STOP;
        return LoopChange::StopRequested;
    }
    LoopChange::None
}

/// Whether the loop's voice counts as started for the next tick: playing,
/// or asked to start (the mixer raises its active bit when it binds it).
pub fn loop_started(voice: &Voice) -> bool {
    voice.flags & (voice_flags::ACTIVE | voice_flags::RESTART) != 0
}

/// Where a clap plays, in metres relative to the listener: the strike's
/// position, which the original passes to a listener-relative source.
pub fn thunder_offset(thunder: &Thunder) -> Vec3 {
    Vec3::from_array(thunder.position) / CM_PER_M
}

/// The drawn streaks as line-list attributes: each drop's head and tail in
/// the box's frame (cm), and its colour bytes as plain numbers.
pub fn streak_vertices(rain: &Rain) -> (Vec<[f32; 3]>, Vec<[f32; 4]>) {
    rain.vertices()
        .into_iter()
        .map(|(p, c)| (p.to_array(), c.map(f32::from)))
        .unzip()
}

/// The streaks' placement: the box at its place (cm to m), turned and
/// scaled for the view's field (`fov_degrees`), its drops in cm.
pub fn streak_transform(rain: &Rain, fov_degrees: f32) -> Transform {
    Transform {
        translation: rain.at / CM_PER_M,
        rotation: box_rotation(),
        scale: Vec3::splat(box_scale(fov_degrees) / CM_PER_M),
    }
}

/// The outdoor arena setup's ambience for `level` over the scene's
/// daylight, wind and rain target; `None` for a level that is not an arena.
pub fn arena_setup_ambience(
    level: &str,
    daylight: f32,
    wind: f32,
    rain_target: f32,
) -> Option<exa_audio::ArenaAmbience> {
    let index = crate::arena_match::arena_index(level)?;
    if index < FIRST_OUTDOOR_ARENA {
        return Some(exa_audio::ArenaAmbience::default());
    }
    let calm = crate::arena_match::ARENA_MAPS[index].steady_bed;
    Some(exa_audio::arena_ambience(daylight, wind, rain_target, calm))
}

/// Creates, steps, draws and frees the rain effect from the weather's
/// state, and writes its volume and pitch back for the next tick.
#[allow(clippy::too_many_arguments)]
fn rain_effect(
    mut commands: Commands,
    runtime: Option<ResMut<SceneEnvRuntime>>,
    cameras: Query<(&GlobalTransform, Option<&Projection>), exa_render::supersample::SceneCamera>,
    mut effects: ResMut<WeatherEffects>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RainMaterial>>,
    mut streaks: Query<(&mut Transform, &mut Visibility), With<RainStreaks>>,
    time: Res<Time<Virtual>>,
    copies: Query<(Entity, Has<ScreenSpaceTransmission>), exa_render::supersample::SceneCamera>,
) {
    let effects = &mut *effects;
    let exists = runtime
        .as_ref()
        .is_some_and(|r| r.env.weather.effect.exists);
    if !exists {
        if effects.rain.take().is_some() {
            info!("weather: rain effect freed");
            for (camera, copied) in &copies {
                if copied {
                    commands.entity(camera).remove::<ScreenSpaceTransmission>();
                }
            }
        }
        if let Some((entity, _, _)) = effects.streaks.take() {
            commands.entity(entity).despawn();
        }
        return;
    }
    // The streaks read the frame behind them: the scene's cameras copy it
    // before the transmissive pass while the effect exists (no copy is made
    // in a frame without rain to draw).
    for (camera, copied) in &copies {
        if !copied {
            commands
                .entity(camera)
                .insert(ScreenSpaceTransmission::default());
        }
    }
    let Some(mut runtime) = runtime else { return };
    let effect = runtime.env.weather.effect;
    let rain = effects.rain.get_or_insert_with(|| {
        info!(
            "weather: rain effect created (rain target {:.2})",
            runtime.env.weather.target
        );
        Rain::new(&mut effects.random)
    });
    if rain.intensity != effect.intensity {
        rain.set_intensity(effect.intensity);
    }
    let camera = cameras.iter().next();
    let live = runtime.env.rain_effect_live() && camera.is_some();
    if !live {
        for (_, mut visibility) in &mut streaks {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Some((at, projection)) = camera else {
        return;
    };
    let fov = match projection {
        Some(Projection::Perspective(p)) => p.fov.to_degrees(),
        _ => exa_render::camera::CAMERA_FOV_DEGREES,
    };
    let eye = at.translation() * CM_PER_M;
    let backward = at.back().as_vec3();
    // The paused game's colour scale (the original tests its pause counter).
    let paused = time.is_paused();
    rain.frame(eye, backward, runtime.now, &mut effects.random);
    runtime.env.weather.effect.volume = rain.drift;
    runtime.env.weather.effect.pitch = rain.fall;
    let scratch = runtime
        .env
        .rain_effect_colour()
        .unwrap_or(runtime.env.frame.clear);
    let colour = Vec4::from_array(rain.colour(
        Vec3::new(scratch[0], scratch[1], scratch[2]),
        paused,
        backward.y,
    ));
    runtime.env.weather.effect.colour = scratch;
    let (positions, colours) = streak_vertices(rain);
    let transform = streak_transform(rain, fov);
    match &effects.streaks {
        Some((entity, mesh, material)) => {
            if let Some(mut m) = meshes.get_mut(mesh) {
                m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
            }
            if let Some(mut m) = materials.get_mut(material)
                && m.colour != colour
            {
                m.colour = colour;
            }
            if let Ok((mut t, mut visibility)) = streaks.get_mut(*entity) {
                *t = transform;
                *visibility = Visibility::Inherited;
            }
        }
        None => {
            let mut mesh = rain_mesh();
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
            let mesh = meshes.add(mesh);
            let material = materials.add(RainMaterial { colour });
            let entity = commands
                .spawn((
                    Name::new("RainEffect"),
                    RainStreaks,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    transform,
                    Visibility::Inherited,
                    NoFrustumCulling,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ))
                .id();
            info!(
                "weather: rain drawn, {} drops at intensity {:.3}, colour {:?}",
                rain.drawn / 2,
                rain.intensity,
                colour
            );
            effects.streaks = Some((entity, mesh, material));
        }
    }
}

/// The rain loop and thunder.
fn weather_sounds(
    game: Option<Res<Game>>,
    runtime: Option<ResMut<SceneEnvRuntime>>,
    mut env: ResMut<AudioEnvironment>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut effects: ResMut<WeatherEffects>,
) {
    let Some(mut runtime) = runtime else {
        // The level (and its scene environment) went away: so does the loop.
        if let Some(id) = effects.rain_voice.take() {
            env.release_voice(&mut bank, Some(id));
        }
        return;
    };
    if !env.is_live() {
        return;
    }
    let events = runtime.events;
    let sound = runtime.env.weather.sound;
    if sound.exists {
        let id = *effects.rain_voice.get_or_insert_with(|| {
            let id = env.allocate_or_dummy();
            if let Some(voice) = env.voice_mut(id) {
                voice.flags = voice_flags::LISTENER_RELATIVE;
            }
            id
        });
        let buffer = if events.start_sound {
            match (effects.rain_buffer, game.as_deref()) {
                (Some(buffer), _) => buffer,
                (None, Some(game)) => {
                    let buffer = bank.load_buffer(game, &mut sources, RAIN_SOUND);
                    effects.rain_buffer = Some(buffer);
                    buffer
                }
                (None, None) => None,
            }
        } else {
            None
        };
        if let Some(voice) = env.voice_mut(id) {
            match drive_rain_voice(voice, &sound, &events, buffer) {
                LoopChange::Started => info!(
                    "weather: rain loop {RAIN_SOUND} started, gain {:.4}, pitch {:.4} (rain {:.3})",
                    voice.gain, voice.pitch, runtime.env.weather.rain
                ),
                LoopChange::StopRequested => info!(
                    "weather: rain loop stop requested (rain {:.3})",
                    runtime.env.weather.rain
                ),
                LoopChange::None => {}
            }
            runtime.env.weather.sound.started = loop_started(voice);
        }
    }
    if let Some(thunder) = events.thunder {
        let set = match (effects.thunder_set, game.as_deref()) {
            (Some(set), _) => set,
            (None, Some(game)) => {
                let set = bank.load_set(game, &mut sources, THUNDER_SET);
                effects.thunder_set = Some(set);
                set
            }
            (None, None) => return,
        };
        let offset = thunder_offset(&thunder);
        let voice = env.play_set(
            &mut bank,
            set,
            offset,
            thunder.volume,
            thunder.pitch,
            voice_flags::LISTENER_RELATIVE,
        );
        let heard = exa_audio::heard_gain(thunder.volume.min(4.0), None, offset, Vec3::ZERO);
        info!(
            "weather: thunder, gain {:.3}, pitch {:.3}, {:.1} m from the listener (heard {:.3}), voice {:?}",
            thunder.volume,
            thunder.pitch,
            offset.length(),
            heard,
            voice
        );
    }
}

/// Sets an arena's weather ambience once its scene environment is in.
fn arena_weather_ambience(
    mut commands: Commands,
    runtime: Option<Res<SceneEnvRuntime>>,
    roots: Query<&LevelRoot>,
) {
    let Some(runtime) = runtime else { return };
    if !runtime.is_added() {
        return;
    }
    let env = &runtime.env;
    let Some((level, choice)) = roots.iter().find_map(|root| {
        arena_setup_ambience(&root.name, env.daylight, env.wind, env.weather.target)
            .map(|c| (root.name.clone(), c))
    }) else {
        return;
    };
    info!(
        "weather: {level} ambience {:?} (daylight {:.2}, wind {:.2}, rain target {:.2})",
        choice
            .area
            .as_ref()
            .map(|a| (a.name.as_str(), a.volume, choice.bed)),
        env.daylight,
        env.wind,
        env.weather.target
    );
    commands.insert_resource(exa_audio::LevelAmbience(choice.area));
    commands.insert_resource(exa_audio::LevelAmbienceBed(choice.bed));
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::scene_env::{SceneEnvironment, WeatherRng};

    fn sound(volume: f32, pitch: f32) -> RainSound {
        RainSound {
            exists: true,
            volume,
            pitch,
            ..RainSound::default()
        }
    }

    /// A quiet voice starts the buffer at the listener, looping, keeping
    /// its listener-relative bit; a playing one is asked to stop.
    #[test]
    fn the_loop_starts_and_stops_on_the_ticks_requests() {
        let mut voice = Voice {
            flags: voice_flags::LISTENER_RELATIVE,
            ..Voice::default()
        };
        let start = WeatherEvents {
            start_sound: true,
            ..WeatherEvents::default()
        };
        let change = drive_rain_voice(&mut voice, &sound(0.12, 1.3), &start, Some(7));
        assert_eq!(change, LoopChange::Started);
        assert_eq!(
            voice.flags,
            voice_flags::LISTENER_RELATIVE
                | voice_flags::ALLOCATED
                | voice_flags::RESTART
                | voice_flags::LOOPING
        );
        assert_eq!((voice.buffer, voice.gain, voice.pitch), (7, 0.12, 1.3));
        assert_eq!(voice.position, Vec3::ZERO);
        assert!(loop_started(&voice));
        // The levels follow the effect every tick.
        voice.flags |= voice_flags::ACTIVE;
        let quiet = drive_rain_voice(
            &mut voice,
            &sound(0.2, 1.1),
            &WeatherEvents::default(),
            None,
        );
        assert_eq!(quiet, LoopChange::None);
        assert_eq!((voice.gain, voice.pitch), (0.2, 1.1));
        let stop = WeatherEvents {
            stop_sound: true,
            ..WeatherEvents::default()
        };
        let change = drive_rain_voice(&mut voice, &sound(0.0, 1.1), &stop, None);
        assert_eq!(change, LoopChange::StopRequested);
        assert_ne!(voice.flags & voice_flags::STOP, 0);
        // Without its buffer, a start request does nothing.
        let mut idle = Voice::default();
        assert_eq!(
            drive_rain_voice(&mut idle, &sound(0.5, 1.0), &start, None),
            LoopChange::None
        );
    }

    /// The loop's level is the effect's drift `(0.3·gust + 0.15)·I²`: at a
    /// rain of 0.9 and a still gust, 0.1215 at pitch `0.9·0.9 + 0.4`. The
    /// tick hands both to the sound and starts it.
    #[test]
    fn the_loop_takes_the_effects_drift_and_fall() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(1);
        let mut system = || 0.5;
        env.set_rain(0.9, 0, &mut rng);
        env.weather.rain = 0.9;
        let mut rain = Rain::new(&mut Mt19937::new(RAIN_RANDOM_SEED));
        rain.set_intensity(0.9);
        rain.step(Vec3::ZERO, 0.0, 0.0, &mut Mt19937::new(1));
        assert_eq!(rain.gust, 0.0);
        assert!((rain.drift - 0.1215).abs() < 1e-6);
        assert!((rain.fall - 1.21).abs() < 1e-6);
        env.weather.effect.volume = rain.drift;
        env.weather.effect.pitch = rain.fall;
        let events = env.tick(16, &mut rng, &mut system);
        assert!(events.start_sound);
        assert_eq!(env.weather.sound.volume, rain.drift);
        assert_eq!(env.weather.sound.pitch, rain.fall);
    }

    /// A clap from a strike 3 m out, moved 2 m ahead of a view 10 m out
    /// along x, plays 15 m from the listener: the original's
    /// listener-relative source at the strike's world position.
    #[test]
    fn a_clap_plays_relative_to_the_listener_at_the_strike() {
        let mut env = SceneEnvironment::new(5);
        let mut rng = WeatherRng::new(3);
        let mut system = || 0.5;
        let view = exa_world::scene_env::View {
            forward: [1.0, 0.0, 0.0],
            position: [1_000.0, 170.0, 0.0],
        };
        env.weather.thunder_allowed = true;
        env.strike_lightning([300.0, -100.0, 0.0], Some(view), 0, &mut rng, &mut system);
        let due = env.weather.thunder_at;
        let thunder = env.tick(due + 1, &mut rng, &mut system).thunder.unwrap();
        let offset = thunder_offset(&thunder);
        assert!((offset - Vec3::new(15.0, 0.7, 0.0)).length() < 1e-4);
        // (1 − 316.23 · 0.0002)² · 4 + 0.1.
        assert!((thunder.volume - 3.610).abs() < 1e-3);
        assert!((thunder.pitch - 1.05).abs() < 1e-6);
    }

    /// The streaks are the drawn heads and tails with their colour bytes
    /// as numbers; the box stands at its place in metres with its drops in
    /// centimetres.
    #[test]
    fn the_streaks_carry_the_drawn_drops() {
        let mut rain = Rain::new(&mut Mt19937::new(RAIN_RANDOM_SEED));
        rain.set_intensity(0.5);
        rain.at = Vec3::new(100.0, 200.0, -450.0);
        let (positions, colours) = streak_vertices(&rain);
        assert_eq!(positions.len(), 2048);
        assert_eq!(colours[0], [160.0, 160.0, 160.0, 0.0]);
        assert_eq!(colours[1], [160.0, 160.0, 160.0, 255.0]);
        assert_eq!(positions[0], rain.drops[0].head.to_array());
        let t = streak_transform(&rain, 70.0);
        assert_eq!(t.translation, Vec3::new(1.0, 2.0, -4.5));
        assert!((t.scale.x - 0.01).abs() < 1e-8);
    }

    /// The arena setup's choice for the arenas: the indoor ones clear the
    /// ambience; a rainy arenao3 leaves it to the rain loop; the calm
    /// arenao3 plays the day bed in a wind, the windy arenao2 the wind bed.
    #[test]
    fn the_arena_setup_follows_the_weather() {
        let indoor = arena_setup_ambience("arena4.rfc", 0.9, 0.6, 0.0).unwrap();
        assert_eq!(indoor, exa_audio::ArenaAmbience::default());
        assert_eq!(
            arena_setup_ambience("arenao3", 0.9, 0.6, 0.9).unwrap(),
            exa_audio::ArenaAmbience::default()
        );
        let calm = arena_setup_ambience("arenao3", 0.9, 0.6, 0.0).unwrap();
        assert_eq!(calm.area.unwrap().name, "NtrA");
        let windy = arena_setup_ambience("ARENAO2", 0.9, 0.6, 0.0).unwrap();
        assert_eq!(windy.area.unwrap().name, "NtrW");
        let (gain, pitch) = windy.bed.unwrap();
        assert!((gain - 0.072).abs() < 1e-6 && (pitch - 0.9).abs() < 1e-6);
        assert_eq!(arena_setup_ambience("exanima01", 0.9, 0.6, 0.0), None);
    }
}
