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

enum Preset {
    Indoor,
    Outdoor,
    FromRecord,
}

fn preset(level: &str) -> Preset {
    let stem = level
        .strip_suffix(".rfc")
        .unwrap_or(level)
        .to_ascii_lowercase();
    if stem.starts_with("arenao") {
        Preset::Outdoor
    } else if stem.starts_with("exanima") || stem.starts_with("arena") && stem != "arenahub" {
        Preset::Indoor
    } else {
        Preset::FromRecord
    }
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
pub fn level_lighting_with(
    level: &str,
    record: Option<&Environment>,
    lights: usize,
    legacy_grid: Option<bool>,
) -> Lighting {
    let mut env = SceneEnvironment::new(SKY_QUALITY);
    if let Some(record) = record {
        env.apply_record(record);
    }
    let sky = match preset(level) {
        Preset::Outdoor => true,
        Preset::Indoor => false,
        Preset::FromRecord => false,
    };
    if let Some(terrain) = legacy_grid {
        env.force_indoor(terrain);
    }
    if sky {
        // Turning the sky on places the sun or moon and dims the record.
        env.set_sky(true);
        env.fog.enabled = true;
        env.fog.colour = OUTDOOR_FOG_COLOUR;
        env.fog.span = OUTDOOR_FOG_DISTANCE * 100.0;
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
    let lit = sky
        || match (preset(level), env.sun_mode) {
            (Preset::FromRecord, SunMode::On) => true,
            (Preset::FromRecord, SunMode::Off) => false,
            (Preset::FromRecord, SunMode::Auto) => lights == 0,
            _ => false,
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
}
