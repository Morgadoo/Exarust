//! `--frame-digest <file.csv>`: a debug digest of what each frame draws,
//! one row a frame past the loading screen (the first is frame 1, as the
//! offscreen host counts them), so two `--fixed-frames` runs of one binary
//! can be compared column by column and the first frame where a part of the
//! picture's inputs differs found (`context/tools/frame_digest_diff.py`).
//!
//! Each column is a hash of one kind of input, with entity and asset ids
//! left out (they are numbered in load order, which may differ between
//! runs): the cameras, the lights, the drawn meshes (their places, tags and
//! light lists), the surface materials (their parameters and which of their
//! maps are loaded), the images (their sizes and formats), the GI state and
//! warm-up, and whether the texture loader and the content stream are idle.
//! A row costs a few milliseconds on a level; a run without the option
//! does nothing. `EXA_FRAME_DIGEST_DUMP=F1,F2` also lists those frames'
//! items in full (`<file>.fNNNNN.txt`, sorted), to diff two runs' items.

use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::Path;

use bevy::camera::visibility::ViewVisibility;
use bevy::light::{DirectionalLight, PointLight, SpotLight};
use bevy::mesh::{Mesh3d, MeshTag};
use bevy::prelude::*;
use exa_render::lights::MeshLights;
use exa_render::material::ExaMaterial;

/// The open digest.
#[derive(Resource)]
pub(crate) struct FrameDigest {
    out: BufWriter<File>,
    frame: u64,
    path: std::path::PathBuf,
    /// Frames whose items are listed in full beside the digest
    /// (`EXA_FRAME_DIGEST_DUMP=F1,F2`, as `<file>.fNNNNN.txt`).
    dump: Vec<u64>,
}

impl FrameDigest {
    pub(crate) fn create(path: &Path) -> std::io::Result<Self> {
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(
            out,
            "frame,ticks,virtual_us,overstep,cameras,lights,meshes,drawn,materials,images,gi,textures_idle,stream_idle"
        )?;
        let dump = std::env::var("EXA_FRAME_DIGEST_DUMP")
            .ok()
            .map(|v| v.split(',').filter_map(|w| w.trim().parse().ok()).collect())
            .unwrap_or_default();
        Ok(Self {
            out,
            frame: 0,
            path: path.to_path_buf(),
            dump,
        })
    }
}

pub(crate) struct FrameDigestPlugin;

impl Plugin for FrameDigestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, write_row.run_if(resource_exists::<FrameDigest>));
    }
}

fn hash_of(value: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// The order-free hash of a set of item hashes.
fn set_hash(mut items: Vec<u64>) -> u64 {
    items.sort_unstable();
    hash_of(items)
}

fn bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn place(g: &GlobalTransform) -> Vec<u32> {
    bits(&g.affine().to_cols_array())
}

/// One kind of input as item lines, their order-free hash and count.
fn summary(items: &[String]) -> String {
    let hash = set_hash(items.iter().map(hash_of).collect());
    format!("{hash:016x}/{}", items.len())
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn write_row(
    mut digest: ResMut<FrameDigest>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    time: Res<Time<Virtual>>,
    fixed: Res<Time<Fixed>>,
    cameras: Query<(&GlobalTransform, &Projection, &Camera)>,
    (points, spots, suns): (
        Query<(&GlobalTransform, &PointLight)>,
        Query<(&GlobalTransform, &SpotLight)>,
        Query<(&GlobalTransform, &DirectionalLight)>,
    ),
    meshes: Query<
        (
            &GlobalTransform,
            &ViewVisibility,
            Option<&MeshTag>,
            Option<&MeshLights>,
            Option<&Name>,
        ),
        With<Mesh3d>,
    >,
    (materials, images): (Res<Assets<ExaMaterial>>, Res<Assets<Image>>),
    (gi, warm_up, textures, content): (
        Option<Res<exa_world::gi_volume::GiState>>,
        Option<Res<exa_render::gi::GiWarmUp>>,
        Option<Res<exa_assets::TextureLoader>>,
        Option<Res<exa_game::content_requests::ContentRequests>>,
    ),
) {
    if loading.is_some() {
        return;
    }
    digest.frame += 1;
    let frame = digest.frame;
    let dump = digest.dump.contains(&frame);
    let cameras: Vec<String> = cameras
        .iter()
        .map(|(g, p, c)| {
            let projection = match p {
                Projection::Perspective(p) => bits(&[p.fov, p.near, p.far, p.aspect_ratio]),
                Projection::Orthographic(o) => bits(&[o.near, o.far, o.scale]),
                Projection::Custom(_) => Vec::new(),
            };
            format!("{:?} {projection:?} {} {}", place(g), c.order, c.is_active)
        })
        .collect();
    let mut lights: Vec<String> = points
        .iter()
        .map(|(g, l)| {
            format!(
                "point {:?} {:?} {:?} {}",
                place(g),
                bits(&l.color.to_linear().to_f32_array()),
                bits(&[l.intensity, l.range, l.radius]),
                l.shadow_maps_enabled
            )
        })
        .collect();
    lights.extend(spots.iter().map(|(g, l)| {
        format!(
            "spot {:?} {:?} {:?} {}",
            place(g),
            bits(&l.color.to_linear().to_f32_array()),
            bits(&[l.intensity, l.range, l.radius, l.inner_angle, l.outer_angle]),
            l.shadow_maps_enabled
        )
    }));
    lights.extend(suns.iter().map(|(g, l)| {
        format!(
            "sun {:?} {:?} {} {}",
            place(g),
            bits(&l.color.to_linear().to_f32_array()),
            l.illuminance.to_bits(),
            l.shadow_maps_enabled
        )
    }));
    let mut drawn = 0usize;
    let mesh_count = meshes.iter().len();
    let mut mesh_lines = Vec::new();
    let mesh_hash = set_hash(
        meshes
            .iter()
            .filter(|(_, v, ..)| v.get())
            .map(|(g, _, tag, list, name)| {
                drawn += 1;
                // The tag's light-list slot (bits 3 to 19) is a buffer place
                // handed out in arrival order: left out, the list itself
                // is in.
                let tag = tag.map(|t| t.value & !0x000F_FFF8);
                let list = list.map(|l| l.0.clone());
                if dump {
                    mesh_lines.push(format!(
                        "{} {:?} {tag:?} {list:?}",
                        name.map_or("-", |n| n.as_str()),
                        place(g)
                    ));
                }
                hash_of((place(g), tag, list))
            })
            .collect(),
    );
    let loaded = |h: &Option<Handle<Image>>| h.as_ref().map(|h| images.contains(h.id()));
    let materials: Vec<String> = materials
        .iter()
        .map(|(_, m)| {
            format!(
                "{:?} {:?}",
                m.params,
                [
                    loaded(&m.diffuse_map),
                    loaded(&m.normal_map),
                    loaded(&m.specular_map),
                    loaded(&m.emissive_map),
                    loaded(&m.opacity_map),
                    loaded(&m.environment),
                    loaded(&m.dirt_map),
                    loaded(&m.gloss_map),
                    loaded(&m.gi_volume),
                    loaded(&m.height_map),
                ]
            )
        })
        .collect();
    let images: Vec<String> = images
        .iter()
        .map(|(_, i)| {
            let d = i.texture_descriptor.size;
            format!(
                "{}x{}x{} mips {} {:?} {:?}",
                d.width,
                d.height,
                d.depth_or_array_layers,
                i.texture_descriptor.mip_level_count,
                i.texture_descriptor.format,
                i.data.as_ref().map(Vec::len)
            )
        })
        .collect();
    let gi = format!(
        "{:?}/{:.4}",
        gi.as_deref(),
        warm_up.map_or(-1.0, |w| w.progress())
    );
    let textures_idle = textures.map(|t| t.is_idle());
    let stream_idle = content.map(|c| c.worker().is_idle());
    let _ = writeln!(
        digest.out,
        "{frame},{},{},{:.6},{},{},{mesh_hash:016x}/{mesh_count},{drawn},{},{},{gi},{textures_idle:?},{stream_idle:?}",
        ticks.map_or(0, |t| t.0),
        time.elapsed().as_micros(),
        fixed.overstep_fraction(),
        summary(&cameras),
        summary(&lights),
        summary(&materials),
        summary(&images),
    );
    if dump {
        let mut path = digest.path.clone().into_os_string();
        path.push(format!(".f{frame:05}.txt"));
        let write = || -> std::io::Result<()> {
            let mut out = BufWriter::new(File::create(&path)?);
            for (kind, mut items) in [
                ("camera", cameras),
                ("light", lights),
                ("mesh", mesh_lines),
                ("material", materials),
                ("image", images),
            ] {
                items.sort();
                for item in items {
                    writeln!(out, "{kind} {item}")?;
                }
            }
            Ok(())
        };
        if let Err(e) = write() {
            warn!(
                "frame digest: cannot write {}: {e}",
                Path::new(&path).display()
            );
        }
    }
}
