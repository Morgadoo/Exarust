//! Level lighting: ambient colour, sunlight, fog and background, from the
//! level's environment record and the per-level presets of the original.
//!
//! The original overrides the record by level: story levels and the indoor
//! arenas use an indoor preset (no sun, sky or fog); the outdoor arenas
//! (`arenao*`) get sunlight, sky and fog. Elsewhere the record's sun mode
//! applies, where "auto" lights a sun only in levels without lights.
//! Evidence: `docs/formats/RFC.md` (Environment).
//!
//! The original's sky shader is not decoded; outdoor skies use Bevy's
//! physically based atmosphere lit by the level's sun, as a stand-in.
//!
//! The rules themselves (sun and moon placement, the day record, clouds,
//! fog narrowing and weather) live in [`crate::scene_env`], which is pure and
//! tested without an app; this module only turns them into a
//! [`Lighting`] and Bevy resources.

use bevy::light::Atmosphere;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::pbr::{AtmosphereSettings, DistanceFog, FogFalloff};
use bevy::prelude::*;
use exa_formats::environment::{Environment, SunMode};

use crate::scene_env::{OUTDOOR_FOG_COLOUR, SceneEnvironment};

/// The render quality the original reads when it builds the environment's
/// sky map (the top setting, 96 texels).
const SKY_QUALITY: i32 = 5;
/// Ambient colour of a typical level (sRGB grey).
const REFERENCE_AMBIENT: f32 = 0.6275;
/// The original's ambient is the environment cube's blurred colour × the
/// level's ambient × 16; the shader samples the cube, the scene carries the
/// rest.
const AMBIENT_SCALE: f32 = 16.0;
/// Sun strength in the original's units: without and with the sky.
const SUN_INTENSITY: f32 = 3.0;
const SUN_INTENSITY_WITH_SKY: f32 = 4.5;
/// Lux per unit of the original's light intensity.
const LUX_PER_INTENSITY: f32 = 160.0;
/// The sun of the original's lit outdoor levels (no sky), in lux: the light
/// a scene without a level is shown under. The shading divides lux back into
/// the original's units, so a brighter light only washes the picture out.
pub const SUN_LUX: f32 = SUN_INTENSITY * LUX_PER_INTENSITY;
/// The outdoor preset's fog reaches this far, metres (the level records'
/// distance words give about this).
const OUTDOOR_FOG_DISTANCE: f32 = 1000.0;
/// Shadowed range of the sun's Bevy cascades, metres (levels without the
/// fitted map).
const SUN_SHADOW_RANGE: f32 = 60.0;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        // The sun's map size comes from `exa_render` (the original's top
        // shadow detail); do not override it here.
        app.add_systems(PostUpdate, (fog_cameras, sky_cameras));
    }
}

/// How a level is lit.
#[derive(Debug, Clone, PartialEq)]
pub struct Lighting {
    /// The level's ambient colour, linear.
    pub ambient: Vec3,
    pub sun: Option<Sun>,
    pub fog: Option<LevelFog>,
    /// An open sky over the level.
    pub sky: bool,
    /// Linear background colour.
    pub background: Vec3,
}

/// Present while the level has an open sky.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LevelSky;

/// The atmosphere entity of the current sky.
#[derive(Component)]
struct Sky;

/// Resolution of the atmosphere's scattering tables.
const SKY_RESOLUTION: u32 = 256;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sun {
    /// Unit vector towards the sun.
    pub towards: Vec3,
    /// Linear colour.
    pub colour: Vec3,
    pub illuminance: f32,
}

/// Fog a level asks the cameras for.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct LevelFog {
    /// Linear colour.
    pub colour: Vec3,
    /// Where the fog starts, metres.
    pub near: f32,
    /// Where it reaches its full strength, metres.
    pub far: f32,
}

/// How the save unit lights a level it loads.
///
/// The original picks between two scene presets when a scene is created:
/// the indoor/underground preset for the first arenas and the story levels,
/// the outdoor preset for the later arenas, which also draws the day's
/// weather. Every other level keeps its own environment record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelPreset {
    /// No sun, no sky, fog at distance 2, a full-strength grey global light,
    /// a black background and grey fog.
    Indoor,
    /// The sun on, a sky, and a seeded draw of the time of day, the rain,
    /// the fog distance, the cloudiness and the wind. `night_allowed` is the
    /// original's per-arena flag: only the arenas it names may draw a night.
    Outdoor { night_allowed: bool },
    /// The level's own environment record decides the sun.
    Record,
}

/// The outdoor arenas whose entry in the original's table allows the
/// preset's night draw. The fifth outdoor arena does not.
pub const NIGHT_ARENAS: [&str; 4] = ["arenao1", "arenao2", "arenao3", "arenao4"];

/// The scene preset a level's name selects, as the original's scene creation
/// chooses it from the level it was asked to load.
pub fn level_preset(level: &str) -> LevelPreset {
    let stem = level.to_ascii_lowercase();
    let stem = stem.strip_suffix(".rfc").unwrap_or(&stem);
    if stem.starts_with("arenao") {
        LevelPreset::Outdoor {
            night_allowed: NIGHT_ARENAS.contains(&stem),
        }
    } else if stem.starts_with("exanima") || stem.starts_with("arena") && stem != "arenahub" {
        LevelPreset::Indoor
    } else {
        LevelPreset::Record
    }
}

/// What the scene presets draw from at a level load: the seed of the outdoor
/// preset's local stream (zero asks the caller for a fresh one), the seed of
/// the engine's shared weather stream (the thunder draw) and the tick the
/// weather schedules on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PresetInput {
    pub seed: u32,
    pub engine_seed: u32,
    pub now: u32,
}

/// A seed for the presets' draws from the process clock, as the original
/// takes one from the engine's clock when it creates a scene. This is read
/// once per level load, never inside the fixed step.
pub fn clock_seed() -> u32 {
    let clock = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos() as u64);
    (clock as u32) ^ ((clock >> 32) as u32)
}

fn linear(srgb: [f32; 3]) -> Vec3 {
    let c = Color::srgb(srgb[0], srgb[1], srgb[2]).to_linear();
    Vec3::new(c.red, c.green, c.blue)
}

/// The lighting of `level`, given its environment record and the number of
/// lights its own nodes and props bring.
pub fn level_lighting(level: &str, record: Option<&Environment>, lights: usize) -> Lighting {
    level_lighting_with(level, record, lights, None)
}

/// The lighting of `level`; `legacy_grid` is `Some(terrain)` for a
/// tile-grid level built the original's older way, which forces a black
/// background and grey fog and, without terrain, a grey ambient. Nothing in
/// Exarust builds levels that way yet, so the game passes `None`.
///
/// This is the look the level's own data gives without the scene presets:
/// the indoor and outdoor arenas take their preset only when
/// [`level_lighting_preset`] is given the draw's input, as a load is.
pub fn level_lighting_with(
    level: &str,
    record: Option<&Environment>,
    lights: usize,
    legacy_grid: Option<bool>,
) -> Lighting {
    level_lighting_preset(level, record, lights, legacy_grid, None)
}

/// The lighting a level load settles on: the preset its name selects,
/// applied from the load's [`PresetInput`] as the original applies it when
/// it creates the scene, then the same derivation as
/// [`level_lighting_with`].
///
/// The indoor preset changes the visible picture only where the level had a
/// fog or a sky (it takes both away); an indoor level already had neither,
/// so its look stands. The outdoor preset draws a new day and weather, which
/// is the original's look for those arenas.
pub fn level_lighting_preset(
    level: &str,
    record: Option<&Environment>,
    lights: usize,
    legacy_grid: Option<bool>,
    input: Option<PresetInput>,
) -> Lighting {
    let mut env = SceneEnvironment::new(SKY_QUALITY);
    if let Some(record) = record {
        env.apply_record(record);
    }
    let preset = level_preset(level);
    let mut sky = matches!(preset, LevelPreset::Outdoor { .. });
    let mut preset_outdoor = false;
    match (preset, input) {
        (LevelPreset::Indoor, Some(_)) => env.apply_indoor_preset(),
        (LevelPreset::Outdoor { night_allowed }, Some(input)) => {
            let mut engine = crate::scene_env::WeatherRng::new(input.engine_seed);
            env.apply_outdoor_preset(
                night_allowed,
                input.seed,
                clock_seed,
                input.now,
                &mut engine,
            );
            preset_outdoor = true;
        }
        _ => {}
    }
    if let Some(terrain) = legacy_grid {
        env.force_indoor(terrain);
        sky = false;
    }
    if sky {
        // Turning the sky on places the sun or moon and dims the record.
        env.set_sky(true);
        env.fog.enabled = true;
        env.fog.colour = OUTDOOR_FOG_COLOUR;
        if !preset_outdoor {
            env.fog.span = OUTDOOR_FOG_DISTANCE * 100.0;
        }
        env.fog.near = 0.0;
        // Under a sky the fog takes its colour from the day record, which
        // is not among the level files Exarust reads: the outdoor preset
        // keeps the original's haze colour as a stand-in.
        env.record_fog = crate::scene_env::linearise([
            OUTDOOR_FOG_COLOUR[0],
            OUTDOOR_FOG_COLOUR[1],
            OUTDOOR_FOG_COLOUR[2],
        ]);
        env.background = OUTDOOR_FOG_COLOUR;
    }
    let lit = if input.is_some() && matches!(preset, LevelPreset::Indoor) {
        // The indoor preset took the sun away.
        false
    } else if input.is_some() && preset_outdoor {
        env.sun_light
    } else {
        sky || match (preset, env.sun_mode) {
            (LevelPreset::Record, SunMode::On) => true,
            (LevelPreset::Record, SunMode::Off) => false,
            (LevelPreset::Record, SunMode::Auto) => lights == 0,
            _ => false,
        }
    };
    env.sun_light = lit;
    let placed = env.set_sun_position(env.azimuth, env.time_of_day);
    let frame = env.apply_frame([0.0, 0.0, 1.0]);
    let sun = lit.then(|| Sun {
        towards: Vec3::from_array(placed.towards).normalize(),
        // Under a sky the original takes the day record's sun colour; until
        // Exarust has the record track, the level's own colour stands in.
        colour: linear(if sky && env.track.is_some() {
            env.sky_sun_colour
        } else {
            env.sun_colour
        }),
        illuminance: (if sky {
            SUN_INTENSITY_WITH_SKY
        } else {
            SUN_INTENSITY
        }) * LUX_PER_INTENSITY,
    });
    Lighting {
        ambient: Vec3::from_array(frame.ambient),
        sun,
        fog: frame.fog.map(|fog| LevelFog {
            colour: Vec3::from_array(fog.colour),
            near: fog.near,
            far: fog.far,
        }),
        sky,
        background: Vec3::from_array(frame.background),
    }
}

/// The scene's ambient light for a level ambient colour (linear), in the
/// original's units: the shading reads the scene's ambient as the level's
/// ambient × 16, so Bevy's usual brightness washes surfaces lit by the
/// environment cube out.
pub fn ambient_light(level: Vec3) -> GlobalAmbientLight {
    let a = level * AMBIENT_SCALE;
    GlobalAmbientLight {
        color: Color::linear_rgb(a.x, a.y, a.z),
        brightness: 1.0,
        ..default()
    }
}

/// The ambient of a typical level: what a scene without a level (a model)
/// is shown under.
pub fn reference_ambient() -> GlobalAmbientLight {
    ambient_light(linear([REFERENCE_AMBIENT; 3]))
}

/// Applies `lighting`: ambient and background resources, a sun under
/// `parent`, and fog for the cameras.
pub fn apply_lighting(commands: &mut Commands, lighting: &Lighting, parent: Entity) {
    commands.insert_resource(ambient_light(lighting.ambient));
    let bg = lighting.background;
    commands.insert_resource(ClearColor(Color::linear_rgb(bg.x, bg.y, bg.z)));
    if let Some(sun) = lighting.sun {
        commands.spawn((
            Name::new("_Sunlight"),
            DirectionalLight {
                color: Color::linear_rgb(sun.colour.x, sun.colour.y, sun.colour.z),
                illuminance: sun.illuminance,
                shadow_maps_enabled: true,
                ..default()
            },
            // Bevy's cascades. In a terrain level `sun_shadow` replaces them
            // each frame with the original's one fitted map (and this
            // configuration with its single bound); elsewhere they stand in
            // for the original's whole-scene map.
            CascadeShadowConfigBuilder {
                num_cascades: 2,
                maximum_distance: SUN_SHADOW_RANGE,
                ..default()
            }
            .build(),
            crate::sun_shadow::SunFitState::default(),
            // The light shines along its −Z, away from the sun.
            Transform::default().looking_to(-sun.towards, Vec3::Y),
            ChildOf(parent),
        ));
    }
    match lighting.fog {
        Some(fog) => commands.insert_resource(fog),
        None => commands.remove_resource::<LevelFog>(),
    }
    if lighting.sky {
        commands.insert_resource(LevelSky);
    } else {
        commands.remove_resource::<LevelSky>();
    }
}

/// Spawns the atmosphere of an open sky and renders it on every 3D camera;
/// removes both when the level has none.
fn sky_cameras(
    mut commands: Commands,
    sky: Option<Res<LevelSky>>,
    skies: Query<Entity, With<Sky>>,
    cameras: Query<(Entity, Has<AtmosphereSettings>), exa_render::supersample::SceneCamera>,
    mut media: ResMut<Assets<ScatteringMedium>>,
) {
    match sky {
        Some(_) => {
            if skies.is_empty() {
                let medium = media.add(ScatteringMedium::earth(SKY_RESOLUTION, SKY_RESOLUTION));
                commands.spawn((Name::new("_Sky"), Sky, Atmosphere::earth(medium)));
            }
            for (camera, has) in &cameras {
                if !has {
                    commands
                        .entity(camera)
                        .insert(AtmosphereSettings::default());
                }
            }
        }
        None => {
            for sky in &skies {
                commands.entity(sky).despawn();
            }
            for (camera, has) in &cameras {
                if has {
                    commands.entity(camera).remove::<AtmosphereSettings>();
                }
            }
        }
    }
}

fn fog_cameras(
    mut commands: Commands,
    fog: Option<Res<LevelFog>>,
    cameras: Query<(Entity, Has<DistanceFog>), exa_render::supersample::SceneCamera>,
) {
    for (camera, fogged) in &cameras {
        match &fog {
            Some(fog) if !fogged || fog.is_changed() => {
                let c = fog.colour;
                commands.entity(camera).insert(DistanceFog {
                    color: Color::linear_rgb(c.x, c.y, c.z),
                    falloff: FogFalloff::Linear {
                        start: fog.near,
                        end: fog.far,
                    },
                    ..default()
                });
            }
            None if fogged => {
                commands.entity(camera).remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_env::{SceneEnvironment, WeatherRng};

    #[test]
    fn presets_by_level() {
        let story = level_lighting("exanima01.rfc", None, 0);
        assert!(story.sun.is_none() && story.fog.is_none());
        let outdoor = level_lighting("arenao3", None, 0);
        assert!(outdoor.sun.is_some() && outdoor.fog.is_some());
        let sun = outdoor.sun.unwrap();
        assert!(sun.towards.y > 0.0);
        // Auto sunlight only without lights.
        assert!(level_lighting("menuscene", None, 0).sun.is_some());
        assert!(level_lighting("menuscene", None, 3).sun.is_none());
    }

    #[test]
    fn the_level_name_selects_the_preset_and_the_night_flag() {
        assert_eq!(
            level_preset("arenao1"),
            LevelPreset::Outdoor {
                night_allowed: true
            }
        );
        assert_eq!(
            level_preset("ArenaO4.RFC"),
            LevelPreset::Outdoor {
                night_allowed: true
            }
        );
        // The fifth outdoor arena's entry does not allow a night.
        assert_eq!(
            level_preset("arenao5"),
            LevelPreset::Outdoor {
                night_allowed: false
            }
        );
        assert_eq!(level_preset("arena1"), LevelPreset::Indoor);
        assert_eq!(level_preset("arena7.rfc"), LevelPreset::Indoor);
        assert_eq!(level_preset("exanima01"), LevelPreset::Indoor);
        // The hub and the menu scene follow their own records.
        assert_eq!(level_preset("arenahub"), LevelPreset::Record);
        assert_eq!(level_preset("menuscene"), LevelPreset::Record);
    }

    #[test]
    fn the_indoor_preset_keeps_an_indoor_level_s_look() {
        let record = Environment {
            ambient: [0.62, 0.55, 0.48],
            sun_colour: [0.9, 0.8, 0.7],
            sun_mode: SunMode::On,
            ..Default::default()
        };
        let plain = level_lighting("exanima01", Some(&record), 7);
        let preset = level_lighting_preset(
            "exanima01",
            Some(&record),
            7,
            None,
            Some(PresetInput::default()),
        );
        // The preset takes the sun and the sky away, so the level's lighting
        // is the record's ambient with a black background and no sun: the
        // picture the level already had.
        assert_eq!(preset.ambient, plain.ambient);
        assert_eq!(preset.background, plain.background);
        assert!(preset.sun.is_none() && preset.sky == plain.sky);
        assert_eq!(preset.fog.is_none(), plain.fog.is_none());
    }

    #[test]
    fn the_outdoor_preset_draws_the_day_and_the_weather() {
        // The first six-way roll of seed 1 is zero: a night when allowed.
        let mut night = SceneEnvironment::new(SKY_QUALITY);
        let mut engine = WeatherRng::new(7);
        let drawn = night.apply_outdoor_preset(true, 1, || 9, 0, &mut engine);
        assert!((drawn.time_of_day - 0.966_651_6).abs() < 1e-5, "{drawn:?}");
        // Without the night flag the same seed draws a day.
        let mut day = SceneEnvironment::new(SKY_QUALITY);
        let mut engine = WeatherRng::new(7);
        let drawn = day.apply_outdoor_preset(false, 1, || 9, 0, &mut engine);
        assert!((drawn.time_of_day - 0.320_827_16).abs() < 1e-5, "{drawn:?}");
        assert!((drawn.cloudiness - 1.202_580_9).abs() < 1e-5);
        assert!((drawn.fog_distance - 0.536_592_7).abs() < 1e-6);
        assert!((drawn.wind - 0.835_827_2).abs() < 1e-6);
        assert_eq!(drawn.rain, 0.0);

        // The loader's lighting is this environment's frame: the same sun
        // direction and the same fog.
        let lit = level_lighting_preset(
            "arenao5",
            None,
            0,
            None,
            Some(PresetInput {
                seed: 1,
                engine_seed: 7,
                now: 0,
            }),
        );
        assert!(lit.sky);
        let sun = lit.sun.expect("the preset's sun");
        let want = Vec3::from_array(day.sun_towards).normalize();
        assert!((sun.towards - want).abs().max_element() < 1e-6);
        assert_eq!(sun.illuminance, SUN_INTENSITY_WITH_SKY * LUX_PER_INTENSITY);
        let fog = lit.fog.expect("the preset's fog");
        assert_eq!(fog.near, 0.0);
        // The drawn fog distance reaches `distance² · 100 000 + 100` cm.
        let span = drawn.fog_distance * drawn.fog_distance * 100_000.0 + 100.0;
        assert!(
            (fog.far - span * 0.01).abs() < 1e-2,
            "{} vs {}",
            fog.far,
            span * 0.01
        );
    }

    #[test]
    fn a_level_without_the_preset_input_keeps_the_record_look() {
        let lit = level_lighting("arenao3", None, 0);
        let preset = level_lighting_preset(
            "arenao3",
            None,
            0,
            None,
            Some(PresetInput {
                seed: 4,
                engine_seed: 4,
                now: 0,
            }),
        );
        // Both are outdoor levels; the preset only redraws the day.
        assert!(lit.sky && preset.sky);
        assert!(lit.fog.is_some() && preset.fog.is_some());
    }

    #[test]
    fn the_outdoor_fog_is_linear_and_a_kilometre_deep() {
        let outdoor = level_lighting("arenao3", None, 0);
        let fog = outdoor.fog.unwrap();
        assert_eq!(fog.near, 0.0);
        // 1 km, narrowed by the low sun the default time puts up (a
        // daylight factor of 0.4978): 0.8310.
        assert!((fog.far - 831.011).abs() < 2e-2, "{}", fog.far);
        // 0.47 sRGB.
        assert!((fog.colour.x - 0.1873).abs() < 1e-3, "{}", fog.colour.x);
    }

    #[test]
    fn the_record_s_choice_of_sun_is_followed() {
        let mut env = Environment {
            sun_mode: SunMode::On,
            ..Default::default()
        };
        assert!(level_lighting("menuscene", Some(&env), 4).sun.is_some());
        env.sun_mode = SunMode::Off;
        assert!(level_lighting("menuscene", Some(&env), 0).sun.is_none());
        env.sun_mode = SunMode::Auto;
        assert!(level_lighting("menuscene", Some(&env), 0).sun.is_some());
        assert!(level_lighting("menuscene", Some(&env), 4).sun.is_none());
    }

    #[test]
    fn a_legacy_grid_level_is_forced_indoor() {
        let env = Environment {
            ambient: [0.5, 0.4, 0.3],
            ..Default::default()
        };
        let lit = level_lighting_with("menuscene", Some(&env), 0, Some(false));
        // 0.6275 grey, laid over the record.
        assert!((lit.ambient.x - 0.3516).abs() < 1e-3);
        assert!((lit.ambient.y - 0.3516).abs() < 1e-3);
        assert_eq!(lit.ambient.z, lit.ambient.y);
        // With terrain, or without the legacy build, the record's own
        // ambient stands.
        let lit = level_lighting_with("menuscene", Some(&env), 0, Some(true));
        assert!((lit.ambient.x - 0.2140).abs() < 1e-3);
        let lit = level_lighting_with("menuscene", Some(&env), 0, None);
        assert!((lit.ambient.x - 0.2140).abs() < 1e-3);
        assert!((lit.ambient.z - 0.0732).abs() < 1e-3);
    }

    #[test]
    fn indoor_and_record_levels_keep_their_earlier_lighting() {
        // The story levels' lighting before the environment model: the
        // record's ambient through the sRGB curve, no sun, fog or sky, and a
        // black background. The model must not move it.
        let record = Environment {
            ambient: [0.62, 0.55, 0.48],
            sun_colour: [0.9, 0.8, 0.7],
            sun_mode: SunMode::On,
            ..Default::default()
        };
        for level in ["exanima01", "exanima02"] {
            let lit = level_lighting(level, Some(&record), 7);
            let want = linear(record.ambient);
            assert!((lit.ambient - want).abs().max_element() < 1e-6, "{level}");
            assert!(lit.sun.is_none() && lit.fog.is_none() && !lit.sky);
            assert_eq!(lit.background, Vec3::ZERO);
        }
        // A level that follows its record keeps the record's sun.
        let lit = level_lighting("menuscene", Some(&record), 7);
        let sun = lit.sun.unwrap();
        let want = Vec3::from_array(record.sun_direction()).normalize();
        assert!((sun.towards - want).abs().max_element() < 1e-6);
        assert!((sun.colour - linear(record.sun_colour)).abs().max_element() < 1e-6);
        assert_eq!(sun.illuminance, SUN_LUX);
    }

    #[test]
    fn ambient_is_linearised() {
        let env = Environment {
            ambient: [REFERENCE_AMBIENT; 3],
            ..Default::default()
        };
        let l = level_lighting("exanima02", Some(&env), 5);
        assert!((l.ambient.x - 0.3515).abs() < 1e-3);
    }

    /// The installed game's arenas (with `EXANIMA_DIR`): the seven indoor
    /// arenas take the indoor preset (no sun, no sky, no fog), the five
    /// outdoor ones the seeded preset (a sky, a sun or moon and fog), and
    /// only the first four outdoor arcs may draw a night.
    #[test]
    fn shipped_arena_levels_take_their_preset() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        for number in 1..=7 {
            let name = format!("arena{number}");
            let bytes = pack
                .get(&format!("{name}.rfc"))
                .unwrap_or_else(|| panic!("{name} is shipped"));
            let scene = exa_formats::rfc::Scene::parse(bytes).unwrap();
            assert_eq!(level_preset(&name), LevelPreset::Indoor);
            let lit = level_lighting_preset(
                &name,
                scene.environment.as_ref(),
                0,
                None,
                Some(PresetInput::default()),
            );
            assert!(lit.sun.is_none() && !lit.sky && lit.fog.is_none(), "{name}");
        }
        // Every indoor level keeps the picture it had before the presets
        // were applied at a load: the same ambient, background, sun and fog.
        let mut indoor = 0;
        for entry in pack.entries() {
            let name = entry.name.to_ascii_lowercase();
            let Some(stem) = name.strip_suffix(".rfc") else {
                continue;
            };
            if level_preset(stem) != LevelPreset::Indoor {
                continue;
            }
            let Some(bytes) = pack.get(&entry.name) else {
                continue;
            };
            let Ok(scene) = exa_formats::rfc::Scene::parse(bytes) else {
                continue;
            };
            indoor += 1;
            // The light count only decides a record level's "auto" sun.
            let plain = level_lighting(stem, scene.environment.as_ref(), 0);
            let preset = level_lighting_preset(
                stem,
                scene.environment.as_ref(),
                0,
                None,
                Some(PresetInput::default()),
            );
            assert_eq!(preset, plain, "{stem}");
        }
        assert!(indoor >= 7, "{indoor} indoor levels");
        for number in 1..=5 {
            let name = format!("arenao{number}");
            let bytes = pack
                .get(&format!("{name}.rfc"))
                .unwrap_or_else(|| panic!("{name} is shipped"));
            let scene = exa_formats::rfc::Scene::parse(bytes).unwrap();
            let night = number <= 4;
            assert_eq!(
                level_preset(&name),
                LevelPreset::Outdoor {
                    night_allowed: night
                }
            );
            // A seed whose six-way roll is zero draws a night when allowed
            // and a day otherwise.
            let lit = level_lighting_preset(
                &name,
                scene.environment.as_ref(),
                0,
                None,
                Some(PresetInput {
                    seed: 1,
                    engine_seed: 1,
                    now: 0,
                }),
            );
            assert!(lit.sky, "{name}");
            let sun = lit.sun.unwrap_or_else(|| panic!("{name} has a sun"));
            // A night draw places the moon below the day path's sun: its
            // height is still positive after the preset's lift.
            assert!(sun.towards.y > 0.0, "{name}");
            assert!(lit.fog.is_some(), "{name}");
        }
        assert_eq!(level_preset("arenahub"), LevelPreset::Record);
    }
}
