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

use bevy::light::Atmosphere;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::pbr::{AtmosphereSettings, DistanceFog, FogFalloff};
use bevy::prelude::*;
use exa_formats::environment::{Environment, SunMode};

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
/// Outdoor fog: distance (m) and colour (sRGB) of the original's sky-less
/// outdoor default.
const OUTDOOR_FOG_DISTANCE: f32 = 1000.0;
const OUTDOOR_FOG: [f32; 3] = [0.47, 0.53, 0.70];
/// Shadowed range of the sun, metres.
const SUN_SHADOW_RANGE: f32 = 60.0;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap { size: 2048 })
            .add_systems(PostUpdate, (fog_cameras, sky_cameras));
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
    /// Metres at which the fog reaches about two thirds.
    pub distance: f32,
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
    let env = record.cloned().unwrap_or_default();
    let ambient = linear(env.ambient);
    let sun = |intensity: f32| Sun {
        towards: Vec3::from_array(env.sun_direction()).normalize(),
        colour: linear(env.sun_colour),
        illuminance: intensity * LUX_PER_INTENSITY,
    };
    match preset(level) {
        Preset::Indoor => Lighting {
            ambient,
            sun: None,
            fog: None,
            sky: false,
            background: Vec3::ZERO,
        },
        Preset::Outdoor => {
            let colour = linear(OUTDOOR_FOG);
            Lighting {
                ambient,
                sun: Some(sun(SUN_INTENSITY_WITH_SKY)),
                fog: Some(LevelFog {
                    colour,
                    distance: OUTDOOR_FOG_DISTANCE,
                }),
                sky: true,
                background: colour,
            }
        }
        Preset::FromRecord => {
            let lit = match env.sun_mode {
                SunMode::On => true,
                SunMode::Off => false,
                SunMode::Auto => lights == 0,
            };
            Lighting {
                ambient,
                sun: lit.then(|| sun(SUN_INTENSITY)),
                fog: None,
                sky: false,
                background: Vec3::ZERO,
            }
        }
    }
}

/// Applies `lighting`: ambient and background resources, a sun under
/// `parent`, and fog for the cameras.
/// The ambient of a typical level, in the original's units: what a scene
/// without a level (a model or the character view) is shown under. The
/// shading reads the scene's ambient as the level's ambient × 16, so Bevy's
/// usual brightness washes surfaces lit by the environment cube out.
pub fn reference_ambient() -> GlobalAmbientLight {
    let grey = Color::srgb(REFERENCE_AMBIENT, REFERENCE_AMBIENT, REFERENCE_AMBIENT).to_linear();
    GlobalAmbientLight {
        color: Color::linear_rgb(
            grey.red * AMBIENT_SCALE,
            grey.green * AMBIENT_SCALE,
            grey.blue * AMBIENT_SCALE,
        ),
        brightness: 1.0,
        ..default()
    }
}

pub fn apply_lighting(commands: &mut Commands, lighting: &Lighting, parent: Entity) {
    let ambient = lighting.ambient * AMBIENT_SCALE;
    commands.insert_resource(GlobalAmbientLight {
        color: Color::linear_rgb(ambient.x, ambient.y, ambient.z),
        brightness: 1.0,
        ..default()
    });
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
            CascadeShadowConfigBuilder {
                num_cascades: 2,
                maximum_distance: SUN_SHADOW_RANGE,
                ..default()
            }
            .build(),
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
                    falloff: FogFalloff::Exponential {
                        density: 1.0 / fog.distance,
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
    fn ambient_is_linearised() {
        let env = Environment {
            ambient: [REFERENCE_AMBIENT; 3],
            ..Default::default()
        };
        let l = level_lighting("exanima02", Some(&env), 5);
        assert!((l.ambient.x - 0.3515).abs() < 1e-3);
    }
}
