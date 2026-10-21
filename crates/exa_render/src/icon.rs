//! Item and character thumbnails: the original's offscreen icon renderer.
//!
//! A private scene and a 3-D view draw one object into a render target twice
//! the icon size on each axis, encoded as sRGB, and the pixels are read
//! back into an RGBA image the inventory draws. The view takes a 30° field
//! of view, a 10 cm near plane, bloom 0.8 and no sharpen; the private scene
//! gets the fixed ambient (0.35, 0.36, 0.37) and a global light of 100.
//! The object is framed by the scene's bounding sphere with a margin (0.75
//! for the still capture, 0.8 in the render path) and the 2× target is
//! averaged down to the icon's size, rows bottom-up.
//!
//! Exarust drives it through [`IconRenderRequest`]: the caller names the
//! entities to show and the icon size; the plugin builds a private camera
//! and target on [`ICON_LAYER`], moves the subjects onto that layer,
//! captures one frame through Bevy's GPU readback, and sends
//! [`IconRendered`] with the downscaled RGBA image (bottom-up rows, four
//! channels). The subjects go back to the default layer afterwards.
//!
//! The placement uses the bounding sphere and the view's field of view
//! (`distance = radius / sin(fov/2) / margin`), which the original's fit
//! routine does; the exact fit call is outside this group, so the framing
//! is Exarust's [guess].

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Camera3d, ClearColorConfig, Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

/// The render layer of a private icon scene (the game draws layers 0..).
pub const ICON_LAYER: usize = 31;
/// The view's vertical field of view, degrees (the original's 30).
pub const FIELD_OF_VIEW_DEGREES: f32 = 30.0;
/// The near plane, metres (10 cm).
pub const NEAR: f32 = 0.1;
/// The private scene's bloom strength.
pub const BLOOM: f32 = 0.8;
/// The private scene's ambient colour (the original's record).
pub const AMBIENT: [f32; 3] = [0.35, 0.36, 0.37];
/// The private scene's global light.
pub const SCENE_LIGHT: f32 = 100.0;
/// The private scene's global light scale (`+0x1e38`).
pub const SCENE_LIGHT_SCALE: f32 = 0.5;
/// The still capture's fit margin.
pub const STILL_MARGIN: f32 = 0.75;
/// The asynchronous render's fit margin.
pub const ASYNC_MARGIN: f32 = 0.8;
/// The scene's bounding radius the original sets before fitting (`+0x24`),
/// metres.
pub const SCENE_RADIUS: f32 = 6.0;
/// The target is this many times the icon's size on each axis.
pub const TARGET_SCALE: u32 = 2;

/// The icon view's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconSettings {
    /// The icon's size in pixels; the target is twice it.
    pub size: u32,
    pub field_of_view: f32,
    pub near: f32,
    pub bloom: f32,
    pub sharpen: bool,
    pub ambient: [f32; 3],
    pub light: f32,
    pub margin: f32,
    /// A transparent background (`GL_SRGB8_ALPHA8` in the original).
    pub transparent: bool,
}

impl IconSettings {
    /// The settings of an icon `size` pixels square. `transparent` picks
    /// the alpha target (otherwise black).
    pub fn of(size: u32, transparent: bool) -> Self {
        Self {
            size: size.max(1),
            field_of_view: FIELD_OF_VIEW_DEGREES.to_radians(),
            near: NEAR,
            bloom: BLOOM,
            sharpen: false,
            ambient: AMBIENT,
            light: SCENE_LIGHT,
            margin: if transparent {
                ASYNC_MARGIN
            } else {
                STILL_MARGIN
            },
            transparent,
        }
    }

    /// The target's side: twice the icon size.
    pub fn target_size(&self) -> u32 {
        self.size * TARGET_SCALE
    }

    /// The camera distance that frames a sphere of `radius` metres:
    /// `radius / sin(fov / 2) / margin`.
    pub fn fit_distance(&self, radius: f32) -> f32 {
        let half = self.field_of_view * 0.5;
        radius / half.sin() / self.margin
    }
}

impl Default for IconSettings {
    fn default() -> Self {
        Self::of(64, true)
    }
}

/// The image the icon captures into: RGBA rows from the bottom up, as the
/// icon atlas stores them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconOutput {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl IconOutput {
    /// The bytes a pixel takes.
    pub const CHANNELS: u8 = 4;

    /// The pixel at a window-style column and row counted from the top
    /// left, for a caller whose atlas counts rows from the bottom.
    pub fn pixel_top_down(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let row = self.height.checked_sub(1 + y)?;
        let at = ((row * self.width + x) * 4) as usize;
        let p = self.pixels.get(at..at + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }
}

/// Asks for one icon capture.
#[derive(Message, Debug, Clone)]
pub struct IconRenderRequest {
    /// The entities to show, in the private scene.
    pub subjects: Vec<Entity>,
    pub size: u32,
    pub transparent: bool,
}

/// One finished icon.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct IconRendered {
    pub output: IconOutput,
}

/// Drops every queued capture (the original's cancel: the render queue's
/// entries are flagged cancelled and never read back).
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct IconRenderCancel;

/// The icon renderer's state.
#[derive(Resource, Default)]
pub struct IconRenderer {
    jobs: Vec<Job>,
}

struct Job {
    readback: Entity,
    camera: Entity,
    target: Handle<Image>,
    settings: IconSettings,
    /// Each subject's layer before the capture, restored afterwards.
    layers: Vec<(Entity, Option<RenderLayers>)>,
}

/// The original's pixel offset from a 2× target to the icon: the average
/// of each 2 × 2 block.
pub fn downscale_2x(pixels: &[u8], width: u32, height: u32) -> IconOutput {
    let out_w = width / 2;
    let out_h = height / 2;
    let mut out = vec![0u8; (out_w * out_h * 4) as usize];
    if out_w == 0 || out_h == 0 {
        return IconOutput {
            width: out_w,
            height: out_h,
            pixels: out,
        };
    }
    for y in 0..out_h {
        // The target's rows arrive top-down; the icon stores them from the
        // bottom up.
        let src_y = 2 * (out_h - 1 - y);
        for x in 0..out_w {
            let src_x = 2 * x;
            let mut sum = [0u32; 4];
            for dy in 0..2 {
                for dx in 0..2 {
                    let at = (((src_y + dy) * width + src_x + dx) * 4) as usize;
                    for (c, s) in sum.iter_mut().enumerate() {
                        *s += u32::from(pixels.get(at + c).copied().unwrap_or(0));
                    }
                }
            }
            let at = ((y * out_w + x) * 4) as usize;
            for (c, s) in sum.iter().enumerate() {
                out[at + c] = ((s + 2) / 4) as u8;
            }
        }
    }
    IconOutput {
        width: out_w,
        height: out_h,
        pixels: out,
    }
}

/// The icon image from a readback of a `width` × `height` RGBA target: rows
/// unpadded (the copy aligns each row to 256 bytes) and averaged 2 × 2.
pub fn output_from_readback(data: &[u8], width: u32, height: u32) -> IconOutput {
    const ALIGN: usize = 256;
    let row_bytes = (width * 4) as usize;
    let stride = row_bytes.div_ceil(ALIGN) * ALIGN;
    let mut tight = Vec::with_capacity(row_bytes * height as usize);
    if stride == row_bytes {
        tight.extend_from_slice(&data[..row_bytes * height as usize]);
    } else {
        for y in 0..height as usize {
            let at = y * stride;
            tight.extend_from_slice(&data[at..at + row_bytes]);
        }
    }
    downscale_2x(&tight, width, height)
}

/// The camera transform that frames a subject at `centre` with `radius`:
/// looking along −Z from the front, at the fit distance.
pub fn icon_camera_transform(settings: &IconSettings, centre: Vec3, radius: f32) -> Transform {
    let distance = settings.fit_distance(radius.max(0.001));
    Transform::from_translation(centre + Vec3::Z * distance).looking_at(centre, Vec3::Y)
}

/// Requests the capture of an entity set, one frame per problem: the icon
/// is rendered in the next draw and read back asynchronously.
pub struct IconRenderPlugin;

impl Plugin for IconRenderPlugin {
    fn build(&self, app: &mut App) {
        // The readback plugin comes with Bevy's render app in the game
        // (GpuReadbackPlugin); a headless app has no renderer.
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            let _ = render_app;
        }
        app.init_resource::<IconRenderer>()
            .add_message::<IconRenderRequest>()
            .add_message::<IconRendered>()
            .add_message::<IconRenderCancel>()
            .add_observer(finish_captures)
            .add_systems(Update, (start_captures, cancel_captures).chain());
    }
}

/// Starts a capture: builds the 2× sRGB target, the private camera and its
/// readback, and moves the subjects to the icon layer.
fn start_captures(
    mut commands: Commands,
    mut requests: MessageReader<IconRenderRequest>,
    mut renderer: ResMut<IconRenderer>,
    mut images: ResMut<Assets<Image>>,
    subjects: Query<&GlobalTransform>,
    bounds: Query<&bevy::camera::primitives::Aabb>,
    layers: Query<&RenderLayers>,
) {
    for request in requests.read() {
        let settings = IconSettings::of(request.size, request.transparent);
        let side = settings.target_size();
        let mut image = Image::new(
            Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0; (side * side * 4) as usize],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage |=
            TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT;
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
        let target = images.add(image);
        let (centre, radius) = subjects_span(&request.subjects, &subjects, &bounds);
        let camera = commands
            .spawn((
                Camera3d::default(),
                Camera {
                    clear_color: if settings.transparent {
                        ClearColorConfig::Custom(Color::NONE)
                    } else {
                        ClearColorConfig::Custom(Color::BLACK)
                    },
                    ..default()
                },
                RenderTarget::Image(target.clone().into()),
                Projection::Perspective(PerspectiveProjection {
                    fov: settings.field_of_view,
                    near: settings.near,
                    far: 1000.0,
                    ..default()
                }),
                icon_camera_transform(&settings, centre, radius),
                RenderLayers::layer(ICON_LAYER),
                Hdr,
                Tonemapping::None,
            ))
            .id();
        let readback = commands.spawn(Readback::texture(target.clone())).id();
        let mut originals = Vec::with_capacity(request.subjects.len());
        for &subject in &request.subjects {
            originals.push((subject, layers.get(subject).ok().cloned()));
            commands
                .entity(subject)
                .insert(RenderLayers::layer(ICON_LAYER));
        }
        renderer.jobs.push(Job {
            readback,
            camera,
            target,
            settings,
            layers: originals,
        });
    }
}

/// The centre and radius of the subjects' world bounds, from their
/// transforms; a subject without a transform is at the origin with radius
/// zero.
fn subjects_span(
    subjects: &[Entity],
    transforms: &Query<&GlobalTransform>,
    bounds: &Query<&bevy::camera::primitives::Aabb>,
) -> (Vec3, f32) {
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for &e in subjects {
        if let Ok(at) = transforms.get(e) {
            let scale = at.scale().abs().max(Vec3::splat(1e-6));
            let half = bounds
                .get(e)
                .map(|b| (Vec3::from(b.half_extents) * scale).length())
                .unwrap_or(0.0);
            let p = at.translation();
            low = low.min(p - Vec3::splat(half));
            high = high.max(p + Vec3::splat(half));
        }
    }
    if !low.is_finite() || !high.is_finite() {
        return (Vec3::ZERO, SCENE_RADIUS);
    }
    let centre = (low + high) * 0.5;
    let radius = (high - centre).length().max(0.001);
    (centre, radius)
}

/// Drops the queued captures: their cameras, readbacks and targets go and
/// their subjects return to their own layers.
fn cancel_captures(
    mut commands: Commands,
    mut requests: MessageReader<IconRenderCancel>,
    mut renderer: ResMut<IconRenderer>,
    mut images: ResMut<Assets<Image>>,
) {
    if requests.read().next().is_none() {
        return;
    }
    for job in std::mem::take(&mut renderer.jobs) {
        for (subject, layer) in &job.layers {
            match layer {
                Some(layer) => {
                    commands.entity(*subject).insert(layer.clone());
                }
                None => {
                    commands.entity(*subject).remove::<RenderLayers>();
                }
            }
        }
        images.remove(&job.target);
        commands.entity(job.camera).despawn();
        commands.entity(job.readback).despawn();
    }
}

/// The readback's pixels become the icon: unpadded, averaged 2 × 2 and
/// sent out.
fn finish_captures(
    event: On<ReadbackComplete>,
    mut commands: Commands,
    mut renderer: ResMut<IconRenderer>,
    mut images: ResMut<Assets<Image>>,
    mut out: MessageWriter<IconRendered>,
) {
    let Some(index) = renderer
        .jobs
        .iter()
        .position(|job| job.readback == event.entity)
    else {
        return;
    };
    let job = renderer.jobs.remove(index);
    let side = job.settings.target_size();
    let output = output_from_readback(&event.data, side, side);
    for (subject, layer) in &job.layers {
        match layer {
            Some(layer) => {
                commands.entity(*subject).insert(layer.clone());
            }
            None => {
                commands.entity(*subject).remove::<RenderLayers>();
            }
        }
    }
    images.remove(&job.target);
    commands.entity(job.camera).despawn();
    commands.entity(job.readback).despawn();
    out.write(IconRendered { output });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_twice_the_icon_and_the_view_is_the_originals() {
        let settings = IconSettings::of(32, true);
        assert_eq!(settings.size, 32);
        assert_eq!(settings.target_size(), 64);
        assert_eq!(settings.field_of_view, 30.0f32.to_radians());
        assert_eq!(settings.near, 0.1);
        assert_eq!(settings.bloom, 0.8);
        assert!(!settings.sharpen);
        assert_eq!(settings.ambient, [0.35, 0.36, 0.37]);
        assert_eq!(settings.light, 100.0);
        assert_eq!(settings.margin, 0.8);
        assert_eq!(IconSettings::of(32, false).margin, 0.75);
    }

    #[test]
    fn the_camera_distance_frames_the_bounding_sphere() {
        let settings = IconSettings::of(64, false);
        let d = settings.fit_distance(1.0);
        // 1 / sin(15°) / 0.75.
        let expected = 1.0 / (15.0f32.to_radians()).sin() / 0.75;
        assert!((d - expected).abs() < 1e-6);
        let at = icon_camera_transform(&settings, Vec3::new(1.0, 2.0, 3.0), 1.0);
        assert_eq!(at.translation, Vec3::new(1.0, 2.0, 3.0 + d));
        assert_eq!(settings.target_size(), 128);
    }

    #[test]
    fn the_output_averages_each_2x2_block_and_flips_its_rows() {
        // A 2×2 target with four distinct pixels.
        let pixels = [
            10, 20, 30, 255, 20, 40, 60, 255, // top row
            30, 60, 90, 255, 40, 80, 120, 255, // bottom row
        ];
        let out = downscale_2x(&pixels, 2, 2);
        assert_eq!((out.width, out.height), (1, 1));
        // (10+20+30+40)/4 = 25, (20+40+60+80)/4 = 50, (30+60+90+120)/4 = 75.
        assert_eq!(out.pixels, [25, 50, 75, 255]);
        // The icon's first row is the target's last: a 2×2 icon from a 4×4
        // target puts the bottom pair first.
        let mut pixels = vec![0u8; 4 * 4 * 4];
        for x in 0..4 {
            for y in 0..4 {
                let at = (y * 4 + x) * 4;
                pixels[at..at + 4].copy_from_slice(&[(10 * y) as u8, 0, 0, 255]);
            }
        }
        let out = downscale_2x(&pixels, 4, 4);
        assert_eq!((out.width, out.height), (2, 2));
        // The icon's first row averages target rows 2 and 3 (y = 2), the
        // second rows 0 and 1 (y = 0).
        assert_eq!(out.pixels[0], 25);
        assert_eq!(out.pixels[8], 5);
        assert_eq!(out.pixel_top_down(0, 0).unwrap()[0], 5);
        assert_eq!(out.pixel_top_down(0, 1).unwrap()[0], 25);
    }

    #[test]
    fn the_readback_rows_are_unpadded() {
        // 3 pixels wide: 12 bytes of row, padded to 256.
        let mut data = vec![0u8; 256 * 2];
        for x in 0..3 {
            data[x * 4..x * 4 + 4].copy_from_slice(&[100, 50, 25, 255]);
            data[256 + x * 4..256 + x * 4 + 4].copy_from_slice(&[100, 50, 25, 255]);
        }
        let out = output_from_readback(&data, 3, 2);
        assert_eq!((out.width, out.height), (1, 1));
        assert_eq!(out.pixels, [100, 50, 25, 255]);
    }
}
