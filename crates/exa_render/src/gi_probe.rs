//! A debug probe of the voxel GI's light volume (`EXA_GI_PROBE`): on one
//! frame the camera block's light texture is read back from the GPU and its
//! contents are logged, so the volume's levels can be compared with the
//! original's numbers without reading them off a debug render.
//!
//! - `EXA_GI_PROBE=<frame>` (default 600) picks the frame read back.
//! - `EXA_GI_PROBE_AT=<x>,<z>` (world metres) picks the column logged
//!   slice by slice; the block's centre column by default.
//!
//! The log lists, per slice, the mean and the largest light (linear, the
//! sRGB bytes decoded) over the open voxels of the block's drawn interior
//! (alpha byte at least 200, the margins left out), then the chosen column,
//! and the lights the injection took that frame (see
//! [`crate::gi::GiLights::describe`]).

use bevy::prelude::*;
use bevy::render::gpu_readback::{ReadbackComplete, ReadbackOnce};

use crate::gi::{GiBaseAmbient, GiLights, GiSkyLight, GiTextures};

pub struct GiProbePlugin;

impl Plugin for GiProbePlugin {
    fn build(&self, app: &mut App) {
        let Some(frame) = probe_frame() else {
            return;
        };
        app.insert_resource(GiProbe {
            frame,
            seen: 0,
            pending: None,
        })
        .add_systems(Last, start_probe)
        .add_observer(probe_complete);
    }
}

fn probe_frame() -> Option<u64> {
    let value = std::env::var("EXA_GI_PROBE").ok()?;
    Some(value.trim().parse().unwrap_or(600))
}

fn probe_column() -> Option<Vec2> {
    let value = std::env::var("EXA_GI_PROBE_AT").ok()?;
    let mut it = value.split(',').map(|s| s.trim().parse::<f32>());
    match (it.next(), it.next()) {
        (Some(Ok(x)), Some(Ok(z))) => Some(Vec2::new(x, z)),
        _ => None,
    }
}

#[derive(Resource)]
struct GiProbe {
    frame: u64,
    seen: u64,
    /// The readback entity and the block it reads.
    pending: Option<(Entity, ProbeBlock)>,
}

#[derive(Clone, Copy)]
struct ProbeBlock {
    origin: Vec3,
    voxel: f32,
    dims: UVec3,
}

fn start_probe(
    mut probe: ResMut<GiProbe>,
    textures: Option<Res<GiTextures>>,
    lights: Option<Res<GiLights>>,
    sky: Option<Res<GiSkyLight>>,
    base: Option<Res<GiBaseAmbient>>,
    mut commands: Commands,
) {
    probe.seen += 1;
    if probe.seen != probe.frame {
        return;
    }
    let Some(textures) = textures else {
        info!("GI probe: no GI block at frame {}", probe.frame);
        return;
    };
    let (voxel, dims) = textures.voxel_and_dims();
    info!(
        "GI probe at frame {}: block origin {:?}, {:?} voxels of {} m, base ambient {:?} per step, sky light colour {:?}",
        probe.frame,
        textures.origin,
        dims,
        voxel,
        base.and_then(|b| b.0).unwrap_or(textures.base_ambient()),
        sky.map(|s| s.0),
    );
    if let Some(lights) = lights {
        for line in lights.describe() {
            info!("GI probe: {line}");
        }
    }
    let entity = commands
        .spawn((
            Name::new("_GiProbeReadback"),
            ReadbackOnce::texture(textures.light.clone()),
        ))
        .id();
    probe.pending = Some((
        entity,
        ProbeBlock {
            origin: textures.origin,
            voxel,
            dims,
        },
    ));
}

fn decode(byte: u8) -> f32 {
    let c = f32::from(byte) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn probe_complete(trigger: On<ReadbackComplete>, mut probe: ResMut<GiProbe>) {
    let Some((entity, block)) = probe.pending else {
        return;
    };
    if trigger.event().entity != entity {
        return;
    }
    probe.pending = None;
    let data = &trigger.event().data;
    let d = block.dims;
    let row = (d.x * 4).div_ceil(256) * 256;
    let at = |x: u32, z: u32, y: u32| -> [u8; 4] {
        let i = ((y * d.y + z) * row + x * 4) as usize;
        [data[i], data[i + 1], data[i + 2], data[i + 3]]
    };
    if data.len() < (row * d.y * d.z) as usize {
        warn!("GI probe: readback of {} bytes is short", data.len());
        return;
    }
    const MARGIN: u32 = 8;
    for y in 0..d.z {
        let mut sum = Vec3::ZERO;
        let mut max = Vec3::ZERO;
        let mut n = 0u32;
        for z in MARGIN..d.y - MARGIN {
            for x in MARGIN..d.x - MARGIN {
                let v = at(x, z, y);
                if v[3] < 200 {
                    continue;
                }
                let c = Vec3::new(decode(v[0]), decode(v[1]), decode(v[2]));
                sum += c;
                max = max.max(c);
                n += 1;
            }
        }
        if n > 0 {
            let mean = sum / n as f32;
            info!(
                "GI probe slice {y:3} (y {:7.2} m): {n:6} open voxels, mean ({:.4}, {:.4}, {:.4}), max ({:.4}, {:.4}, {:.4})",
                block.origin.y + (y as f32 + 0.5) * block.voxel,
                mean.x,
                mean.y,
                mean.z,
                max.x,
                max.y,
                max.z
            );
        }
    }
    let column = probe_column()
        .map(|p| {
            let local = (p - Vec2::new(block.origin.x, block.origin.z)) / block.voxel;
            UVec2::new(
                (local.x.max(0.0) as u32).min(d.x - 1),
                (local.y.max(0.0) as u32).min(d.y - 1),
            )
        })
        .unwrap_or(UVec2::new(d.x / 2, d.y / 2));
    info!(
        "GI probe column ({}, {}) = world ({:.2}, {:.2}) m:",
        column.x,
        column.y,
        block.origin.x + (column.x as f32 + 0.5) * block.voxel,
        block.origin.z + (column.y as f32 + 0.5) * block.voxel
    );
    for y in 0..d.z {
        let v = at(column.x, column.y, y);
        if v == [0, 0, 0, 0] {
            continue;
        }
        info!(
            "GI probe   slice {y:3} (y {:7.2} m): light ({:.4}, {:.4}, {:.4}), alpha {}",
            block.origin.y + (y as f32 + 0.5) * block.voxel,
            decode(v[0]),
            decode(v[1]),
            decode(v[2]),
            v[3]
        );
    }
}
