//! The original's local reflection (`Shd.LocalRef`, shader word `0x400000`).
//!
//! The engine has no mirrored camera and no clip plane: nothing in it
//! renders the scene a second time about a plane. What its programs call a
//! local reflection is a read of the scene's low field, the post-processor's
//! quarter-size blurred bright pass. The post-processor runs at the end of
//! the frame, so a surface drawn in the next frame reads the previous
//! frame's glow. The read lands just off the fragment's own screen position,
//! pushed along the view-space normal:
//!
//! ```text
//! n  = view-space normal
//! k  = 1.05 / (1.05 − n.z) − 1
//! uv = n.xy · k · 0.05 · ViewPrj / w + gl_FragCoord.xy / viewport
//! local = FrmBuf(uv) · 1.5
//! ```
//!
//! `ViewPrj` is the projection's x and y scales times 500 and `w` the
//! fragment's clip depth in centimetres. Inside the GI volume, with a
//! reflection layer, the surface's specular GI mixes the cube reflection
//! toward it by `a = min(2·(1 − Gloss), 1)²·(1 − N·V)`, the programs'
//! `Gloss` being the roughness (one less the material's gloss):
//! `ref = mix(cube · light · Ambient.w · 2, local, a)`.
//!
//! The material factory sets the bit for a record with a reflection layer
//! and extra flag `0x800` above shading quality 4. Of the shipped libraries
//! only `glsdark02` (common.rml) has it: the dark glass of the lamp globes,
//! the hourglass, the glasses and the sewer lamps. The sewer's water is the
//! terrain's wet floor (`Env.WetFloor`), a gloss term that reflects the
//! cube; there is no water plane.
//!
//! Exarust copies its post chain's blurred texture into [`SceneLowField`]
//! after each window frame's post pass, and every
//! [`flags2::LOCAL_REFLECTION`] material binds that image. Rules:
//! `docs/RENDERING.md` (Local reflection).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crate::material::{ExaMaterial, flags2};

/// The constant inside the normal's push: `1.05 / (1.05 − n.z) − 1`.
pub const PUSH_BASE: f32 = 1.05;
/// The push's scale before `ViewPrj`.
pub const PUSH_SCALE: f32 = 0.05;
/// `ViewPrj` is the projection's scales times this many centimetres.
pub const VIEW_PRJ_CM: f32 = 500.0;
/// The low field's read is scaled by this.
pub const READ_GAIN: f32 = 1.5;
/// The low field is this fraction of the window on each axis.
pub const LOW_FIELD_DIVISOR: u32 = 4;

/// `ViewPrj` from the projection's x scale and the viewport: `(P₀₀·500,
/// (w/h)·P₀₀·500)`, the aspect divided in double precision and the products
/// in single, as the original's global block fills it.
pub fn view_prj(p00: f32, width: u32, height: u32) -> [f32; 2] {
    let x = p00 * VIEW_PRJ_CM;
    let aspect = f64::from(width) / f64::from(height.max(1));
    [x, (aspect * f64::from(x)) as f32]
}

/// The screen offset (GL texture units, y up) the read takes for a
/// view-space normal `n` at clip depth `w_cm`.
pub fn read_offset(n: Vec3, w_cm: f32, view_prj: [f32; 2]) -> Vec2 {
    let k = PUSH_BASE / (PUSH_BASE - n.z) - 1.0;
    let mut o = n.truncate() * (k * PUSH_SCALE);
    o *= Vec2::from(view_prj);
    o * (1.0 / w_cm)
}

/// How far the specular GI moves from the cube toward the local read:
/// `min((1 − Gloss)·2, 1)²·(1 − N·V)`. The templates' `Gloss` is the
/// roughness (`1 − MatSpc.w·…`), so `1 − Gloss` is the material's `gloss`:
/// a glossy surface seen edge on takes the local read, a rough one or one
/// seen face on keeps the cube.
pub fn mix_weight(gloss: f32, ndv: f32) -> f32 {
    let a = (gloss * 2.0).min(1.0);
    a * a * (1.0 - ndv)
}

/// The image every local-reflection surface reads: the previous window
/// frame's blurred bright pass, a quarter of the window on each axis.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct SceneLowField {
    pub image: Handle<Image>,
    pub size: UVec2,
}

pub struct LocalReflectionPlugin;

impl Plugin for LocalReflectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<SceneLowField>::default())
            .add_systems(PostUpdate, (follow_window, bind_scene_low).chain())
            .add_systems(
                PostUpdate,
                log_surfaces.after(bevy::transform::TransformSystems::Propagate),
            );
    }
}

fn low_field_image(size: UVec2) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    // The engine's frame texture clamps and filters linearly.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    image
}

/// The low field's size for a window: a quarter on each axis, as the post
/// chain's blurred texture (integer division, at least one texel).
pub fn low_field_size(window: UVec2) -> UVec2 {
    (window / LOW_FIELD_DIVISOR).max(UVec2::ONE)
}

/// Builds the low field for the post chain's last camera (the window's,
/// or the off-screen frame's) and rebuilds it when that camera's target
/// changes size. The handle stays the same, so materials keep it.
fn follow_window(
    mut commands: Commands,
    field: Option<ResMut<SceneLowField>>,
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
    let size = low_field_size(target);
    match field {
        None => {
            let image = images.add(low_field_image(size));
            commands.insert_resource(SceneLowField { image, size });
        }
        Some(mut field) if field.size != size => {
            if let Some(mut image) = images.get_mut(&field.image) {
                *image = low_field_image(size);
            }
            field.size = size;
        }
        Some(_) => {}
    }
}

/// `EXA_LOCAL_REF_DEBUG=1`: local-reflection surfaces show their read.
struct DebugView(bool);

impl Default for DebugView {
    fn default() -> Self {
        Self(std::env::var("EXA_LOCAL_REF_DEBUG").is_ok_and(|v| v == "1"))
    }
}

impl std::ops::Deref for DebugView {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

/// Points every local-reflection material at the low field: new ones as
/// they come, and all of them once the field exists.
fn bind_scene_low(
    field: Option<Res<SceneLowField>>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut events: MessageReader<AssetEvent<ExaMaterial>>,
    debug: Local<DebugView>,
) {
    let Some(field) = field else {
        events.clear();
        return;
    };
    let ids: Vec<AssetId<ExaMaterial>> = if field.is_added() {
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
        if m.params.flags2 & flags2::LOCAL_REFLECTION == 0
            || m.scene_low.as_ref() == Some(&field.image)
        {
            continue;
        }
        if let Some(mut m) = materials.get_mut(id) {
            m.scene_low = Some(field.image.clone());
            if **debug {
                m.params.flags2 |= flags2::DEBUG_LOCAL_REFLECTION;
            }
        }
    }
}

type NewSurface = (
    &'static MeshMaterial3d<ExaMaterial>,
    &'static GlobalTransform,
    Option<&'static bevy::camera::primitives::Aabb>,
    Option<&'static Name>,
);

/// With `RUST_LOG=exa_render::local_reflection=debug`, logs where each new
/// local-reflection surface stands (its bounds centre, metres), so a shot
/// can be framed on one.
fn log_surfaces(
    materials: Res<Assets<ExaMaterial>>,
    added: Query<NewSurface, Added<MeshMaterial3d<ExaMaterial>>>,
) {
    if !bevy::log::tracing::enabled!(bevy::log::Level::DEBUG) {
        return;
    }
    for (m, t, aabb, name) in &added {
        let Some(m) = materials.get(&m.0) else {
            continue;
        };
        if m.params.flags2 & flags2::LOCAL_REFLECTION == 0 {
            continue;
        }
        let at = aabb.map_or(t.translation(), |b| crate::gi_tag::bounds_centre(t, b));
        debug!(
            "local reflection surface {:?} at {at:.2}",
            name.map(Name::as_str)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_prj_is_the_projection_scale_at_five_metres() {
        // A 70° vertical field at 16:9: P₁₁ = cot 35°, P₀₀ = P₁₁·9/16.
        let p11 = 1.0 / (35f32.to_radians()).tan();
        let p00 = p11 * 9.0 / 16.0;
        let [x, y] = view_prj(p00, 1920, 1080);
        assert_eq!(x, p00 * 500.0);
        assert!((y - p11 * 500.0).abs() < 1e-3);
    }

    #[test]
    fn a_normal_facing_the_eye_or_edge_on_does_not_push() {
        let prj = [400.0, 714.0];
        // Edge on: n.z = 0 gives k = 0.
        assert_eq!(read_offset(Vec3::X, 100.0, prj), Vec2::ZERO);
        // Straight at the eye: no x or y to push along.
        assert_eq!(read_offset(Vec3::Z, 100.0, prj), Vec2::ZERO);
    }

    #[test]
    fn the_push_grows_toward_the_eye_and_shrinks_with_depth() {
        let prj = [400.0, 714.0];
        let n = Vec3::new(0.6, 0.0, 0.8);
        let near = read_offset(n, 100.0, prj);
        // k = 1.05/0.25 − 1 = 3.2; 0.6·3.2·0.05·400/100.
        assert!((near.x - 0.384).abs() < 1e-5, "{near}");
        assert_eq!(near.y, 0.0);
        let far = read_offset(n, 400.0, prj);
        assert!((far.x - near.x / 4.0).abs() < 1e-6);
    }

    #[test]
    fn the_mix_takes_the_local_read_on_glossy_grazing_surfaces() {
        assert_eq!(mix_weight(0.0, 0.0), 0.0);
        assert_eq!(mix_weight(1.0, 1.0), 0.0);
        assert_eq!(mix_weight(1.0, 0.0), 1.0);
        assert_eq!(mix_weight(0.25, 0.5), 0.125);
    }

    #[test]
    fn the_low_field_is_a_quarter_of_the_window() {
        assert_eq!(low_field_size(UVec2::new(1920, 1080)), UVec2::new(480, 270));
        assert_eq!(low_field_size(UVec2::new(2, 3)), UVec2::ONE);
    }

    #[test]
    fn the_surface_program_mirrors_the_rules() {
        let src = include_str!("exa.wesl");
        assert!(src.contains("const LOCAL_PUSH_BASE: f32 = 1.05;"));
        assert!(src.contains("const LOCAL_PUSH_SCALE: f32 = 0.05;"));
        assert!(src.contains("const LOCAL_READ_GAIN: f32 = 1.5;"));
        assert!(src.contains("const LOCAL_VIEW_PRJ_CM: f32 = 500.0;"));
    }
}
