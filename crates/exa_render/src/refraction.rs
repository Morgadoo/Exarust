//! The original's refraction (`Mat.Refract`, its `{$refraction}` snippet).
//!
//! A lit record in the liquid mode (material word bits `0x300` = `0x200`;
//! only type 1 keeps them, so only the PBR program refracts) is drawn
//! opaque: the loader rewrites its transparent blend word to replace. What
//! looks through it is a field of the previous frame that the post chain
//! left behind, read at the fragment's own screen position shifted along
//! the view-space normal. The material writer binds it to unit 7
//! (`FrmBuf`): the post chain's half-size bright pass (the high field) for
//! a glossiness above 0.85, its quarter-size blurred bright pass (the low
//! field, which the local reflection also reads) otherwise. The view's
//! constructor asks for the scene's own fields on every view; the built-in
//! `BkgRfr01`/`BkgRfr02` pictures are only for a view that does not.
//!
//! ```text
//! r  = view-space normal
//! GI:     r *= 1.5 − r.z;  r.xy *= 0.01·strength
//! no GI:  r *= r.z·0.01·strength
//! uv = r.xy · ViewPrj / w + gl_FragCoord.xy / viewport
//! a  = dot(eye→surface, n)
//! b  = min(max(1 − |a + 0.9|·1.5, 0) + (1 + a)³·0.7, 1)
//! tint = mix(diffuse colour, 0.2, −0.1 − 0.2·a)
//! rfr = FrmBuf(uv) · b · tint · (8 with GI, else 12)
//! colour = lit · diffuse colour · alpha + rfr · (1 − alpha);  alpha = 1
//! ```
//!
//! `strength` is the record's `0x58` word (one when it is nearly zero),
//! which the writer packs into `OpcFll.w` through its hundredths
//! (`exa_assets::texture::f32_to_hundredths`, then `hundredths_to_f32`); `alpha` is the diffuse alpha
//! after the opacity fall-off. The specular, emissive and fog follow as on
//! any PBR surface. Rules: `docs/RENDERING.md` (Refraction).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crate::local_reflection::SceneLowField;
use crate::material::{ExaMaterial, flags2};
use crate::shader_uniforms::{RefractionMap, SceneRefraction, refraction_map};

/// Above this glossiness the refraction reads the high field.
pub const SHARP_GLOSS: f64 = 0.85;
/// The normal's push per unit of strength.
pub const PUSH_SCALE: f32 = 0.01;
/// Inside the GI volume the whole normal is first scaled by `1.5 − n.z`.
pub const GI_PUSH_BASE: f32 = 1.5;
/// The tint's grey end.
pub const TINT_GREY: f32 = 0.2;
/// The read's gain inside the GI volume, and outside it.
pub const GAIN_GI: f32 = 8.0;
pub const GAIN: f32 = 12.0;
/// The high field is this fraction of the window on each axis (the post
/// chain's bright pass).
pub const HIGH_FIELD_DIVISOR: u32 = 2;

/// Which scene field a refracting record reads: the material writer's
/// choice for a view that asks for the scene's own fields (every view the
/// constructor makes), from the material and shader words and the
/// glossiness. `None` for a record that neither refracts nor reflects
/// locally.
pub fn field_of(material_word: u32, shader_word: u32, gloss: f32) -> Option<RefractionMap> {
    refraction_map(
        material_word,
        shader_word,
        gloss,
        true,
        Some(SceneRefraction { low: 0, high: 1 }),
    )
}

/// The screen offset (GL texture units, y up) the read takes for a
/// view-space normal `n` at clip depth `w_cm`, refraction strength
/// `strength`, inside the GI volume or not.
pub fn read_offset(n: Vec3, w_cm: f32, view_prj: [f32; 2], strength: f32, gi: bool) -> Vec2 {
    let r = if gi {
        let r = n * (GI_PUSH_BASE - n.z);
        r.truncate() * (PUSH_SCALE * strength)
    } else {
        (n * (n.z * PUSH_SCALE * strength)).truncate()
    };
    r * Vec2::from(view_prj) * (1.0 / w_cm)
}

/// How much of the read shows: `a` is the cosine between the eye-to-surface
/// direction and the normal (−1 facing the eye).
pub fn read_weight(a: f32) -> f32 {
    let b = (1.0 - (a + 0.9).abs() * 1.5).max(0.0);
    let c = 1.0 + a;
    (b + c * c * c * 0.7).min(1.0)
}

/// The read's tint for diffuse colour `diffuse` at cosine `a`.
pub fn tint(diffuse: Vec3, a: f32) -> Vec3 {
    diffuse.lerp(Vec3::splat(TINT_GREY), -0.1 - a * 0.2)
}

/// The image a sharp refraction reads: the previous window frame's bright
/// pass, half the window on each axis.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct SceneHighField {
    pub image: Handle<Image>,
    pub size: UVec2,
}

pub struct RefractionPlugin;

impl Plugin for RefractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<SceneHighField>::default())
            .add_systems(PostUpdate, (follow_window, bind_fields).chain());
    }
}

/// The high field's size for a window: half on each axis, as the post
/// chain's bright pass (integer division, at least one texel).
pub fn high_field_size(window: UVec2) -> UVec2 {
    (window / HIGH_FIELD_DIVISOR).max(UVec2::ONE)
}

fn high_field_image(size: UVec2) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // The bright pass's own format (the original's RGB8, not sRGB).
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    image
}

/// Builds the high field for the post chain's last camera and rebuilds it
/// when that camera's target changes size; the handle stays the same.
fn follow_window(
    mut commands: Commands,
    field: Option<ResMut<SceneHighField>>,
    cameras: Query<&Camera, With<crate::post::ExaPost>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(target) = cameras
        .iter()
        .filter(|c| c.is_active)
        .max_by_key(|c| c.order)
        .and_then(Camera::physical_target_size)
    else {
        return;
    };
    let size = high_field_size(target);
    match field {
        None => {
            let image = images.add(high_field_image(size));
            commands.insert_resource(SceneHighField { image, size });
        }
        Some(mut field) if field.size != size => {
            if let Some(mut image) = images.get_mut(&field.image) {
                *image = high_field_image(size);
            }
            field.size = size;
        }
        Some(_) => {}
    }
}

/// Points every refracting material at its field: new ones as they come,
/// and all of them once both fields exist.
fn bind_fields(
    low: Option<Res<SceneLowField>>,
    high: Option<Res<SceneHighField>>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut events: MessageReader<AssetEvent<ExaMaterial>>,
) {
    let (Some(low), Some(high)) = (low, high) else {
        events.clear();
        return;
    };
    let ids: Vec<AssetId<ExaMaterial>> = if low.is_added() || high.is_added() {
        events.clear();
        materials.ids().collect()
    } else {
        events
            .read()
            .filter_map(|e| match e {
                AssetEvent::Added { id } => Some(*id),
                _ => None,
            })
            .collect()
    };
    for id in ids {
        let Some(m) = materials.get(id) else { continue };
        if m.params.flags2 & flags2::REFRACTION == 0 {
            continue;
        }
        let image = if m.params.flags2 & flags2::FRAME_HIGH != 0 {
            &high.image
        } else {
            &low.image
        };
        if m.scene_field.as_ref() == Some(image) {
            continue;
        }
        let image = image.clone();
        if let Some(mut m) = materials.get_mut(id) {
            m.scene_field = Some(image);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glossy_refraction_reads_the_high_field() {
        // Liquid mode, glossiness compared widened to double.
        assert_eq!(field_of(0x207, 0, 0.9), Some(RefractionMap::SceneHigh));
        assert_eq!(field_of(0x207, 0, 0.85), Some(RefractionMap::SceneHigh));
        assert_eq!(
            field_of(0x207, 0, 0.849_999_9),
            Some(RefractionMap::SceneLow)
        );
        assert_eq!(field_of(0x207, 0, f32::NAN), Some(RefractionMap::SceneLow));
        // The subsurface mode (0x300) does not refract.
        assert_eq!(field_of(0x307, 0, 0.9), None);
    }

    #[test]
    fn the_push_follows_the_normal_and_the_strength() {
        let prj = [400.0, 714.0];
        // Facing the eye: nothing to push along.
        assert_eq!(read_offset(Vec3::Z, 100.0, prj, 1.0, true), Vec2::ZERO);
        assert_eq!(read_offset(Vec3::Z, 100.0, prj, 1.0, false), Vec2::ZERO);
        let n = Vec3::new(0.6, 0.0, 0.8);
        // GI: 0.6·(1.5 − 0.8)·0.01·0.5·400/100.
        let gi = read_offset(n, 100.0, prj, 0.5, true);
        assert!((gi.x - 0.6 * 0.7 * 0.005 * 4.0).abs() < 1e-6, "{gi}");
        // No GI: 0.6·0.8·0.01·0.5·400/100.
        let plain = read_offset(n, 100.0, prj, 0.5, false);
        assert!((plain.x - 0.6 * 0.8 * 0.005 * 4.0).abs() < 1e-6, "{plain}");
        // Edge on (n.z = 0) the plain push vanishes, the GI one does not.
        assert_eq!(read_offset(Vec3::X, 100.0, prj, 1.0, false), Vec2::ZERO);
        assert!(read_offset(Vec3::X, 100.0, prj, 1.0, true).x > 0.0);
    }

    #[test]
    fn the_read_shows_most_face_on_and_at_the_rim() {
        // Facing the eye (a = −1): 1 − 0.15 = 0.85.
        assert!((read_weight(-1.0) - 0.85).abs() < 1e-6);
        // At a = −0.9 it peaks at 1.
        assert!((read_weight(-0.9) - 1.0).abs() < 1e-6);
        // Edge on (a = 0): the cubic term, 0.7.
        assert!((read_weight(0.0) - 0.7).abs() < 1e-6);
        // In between it dips: at a = −0.2 only the cubic term, 0.8³·0.7.
        assert!((read_weight(-0.2) - 0.512 * 0.7).abs() < 1e-6);
    }

    #[test]
    fn the_tint_leans_toward_grey_at_the_rim() {
        let d = Vec3::new(0.1, 0.4, 0.2);
        // Face on: 10 % of the way to 0.2.
        let face = tint(d, -1.0);
        assert!((face - (d * 0.9 + Vec3::splat(0.02))).abs().max_element() < 1e-6);
        // Edge on: 10 % past the colour, away from grey.
        let edge = tint(d, 0.0);
        assert!((edge - (d * 1.1 - Vec3::splat(0.02))).abs().max_element() < 1e-6);
    }

    #[test]
    fn the_high_field_is_half_the_window() {
        assert_eq!(
            high_field_size(UVec2::new(1920, 1080)),
            UVec2::new(960, 540)
        );
        assert_eq!(high_field_size(UVec2::new(1, 3)), UVec2::new(1, 1));
    }

    /// The live binding (the plugin's systems in a headless app): a dull
    /// refraction gets the low field, a glossy one the high field, a
    /// surface that neither refracts nor reflects keeps none, and a
    /// material added later is bound when it comes.
    #[test]
    fn refracting_materials_bind_their_scene_field() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<ExaMaterial>()
            .add_plugins(RefractionPlugin);
        let refracting = |high: bool| ExaMaterial {
            params: crate::material::ExaParams {
                flags2: flags2::REFRACTION | if high { flags2::FRAME_HIGH } else { 0 },
                ..default()
            },
            ..default()
        };
        let (dull, glossy, plain) = {
            let mut materials = app.world_mut().resource_mut::<Assets<ExaMaterial>>();
            (
                materials.add(refracting(false)),
                materials.add(refracting(true)),
                materials.add(ExaMaterial::default()),
            )
        };
        app.update();
        let (low, high) = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            (
                images.add(crate::local_reflection::low_field_image_for_tests()),
                images.add(high_field_image(UVec2::new(4, 4))),
            )
        };
        app.insert_resource(SceneLowField {
            image: low.clone(),
            size: UVec2::ONE,
        });
        app.insert_resource(SceneHighField {
            image: high.clone(),
            size: UVec2::new(4, 4),
        });
        app.update();
        let field = |app: &App, h: &Handle<ExaMaterial>| {
            app.world()
                .resource::<Assets<ExaMaterial>>()
                .get(h)
                .unwrap()
                .scene_field
                .clone()
        };
        assert_eq!(field(&app, &dull), Some(low.clone()));
        assert_eq!(field(&app, &glossy), Some(high.clone()));
        assert_eq!(field(&app, &plain), None);
        let later = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(refracting(true));
        app.update();
        app.update();
        assert_eq!(field(&app, &later), Some(high));
    }

    #[test]
    fn the_surface_program_mirrors_the_rules() {
        let src = include_str!("exa.wesl");
        assert!(src.contains("const REFRACT_PUSH_SCALE: f32 = 0.01;"));
        assert!(src.contains("const REFRACT_GI_PUSH_BASE: f32 = 1.5;"));
        assert!(src.contains("const REFRACT_TINT_GREY: f32 = 0.2;"));
        assert!(src.contains("const REFRACT_GAIN_GI: f32 = 8.0;"));
        assert!(src.contains("const REFRACT_GAIN: f32 = 12.0;"));
    }
}
