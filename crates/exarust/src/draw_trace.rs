//! `--draw-trace <file.csv>`: where the frame drew the player each frame,
//! after the transforms propagated: the frame, the real clock (ms since the
//! start), the body ticks the frame has seen, the render fraction the frame drew at, the player's drawn pelvis
//! and the camera (cm). A smoothness check for the drawing between steps:
//! a lockstep run (`--sim-lockstep`) and a free-running one compare by the
//! pelvis's speed from frame to frame.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_game::player::PlayerBody;
use exa_physics::body::Ragdoll;

/// The open trace.
#[derive(Resource)]
pub(crate) struct DrawTrace {
    out: BufWriter<File>,
    start: Instant,
    frame: u64,
}

impl DrawTrace {
    pub(crate) fn create(path: &Path) -> std::io::Result<Self> {
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(
            out,
            "frame,real_ms,ticks,fraction,pelvis_x,pelvis_y,pelvis_z,camera_x,camera_y,camera_z"
        )?;
        Ok(Self {
            out,
            start: Instant::now(),
            frame: 0,
        })
    }
}

pub(crate) struct DrawTracePlugin;

impl Plugin for DrawTracePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            write_row
                .after(TransformSystems::Propagate)
                .run_if(resource_exists::<DrawTrace>),
        );
    }
}

fn write_row(
    mut trace: ResMut<DrawTrace>,
    fixed: Res<Time<Fixed>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    bodies: Query<&Ragdoll, With<PlayerBody>>,
    places: Query<&GlobalTransform>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    trace.frame += 1;
    let pelvis = bodies
        .iter()
        .next()
        .and_then(|b| b.parts.first())
        .and_then(|p| places.get(p.entity).ok())
        .map(|g| g.translation() * 100.0);
    let Some(pelvis) = pelvis else {
        return;
    };
    let camera = cameras
        .iter()
        .next()
        .map_or(Vec3::NAN, |g| g.translation() * 100.0);
    let (frame, ms) = (trace.frame, trace.start.elapsed().as_secs_f64() * 1000.0);
    let fraction = fixed.overstep_fraction();
    let ticks = ticks.map_or(0, |t| t.0);
    let _ = writeln!(
        trace.out,
        "{frame},{ms:.3},{ticks},{fraction:.4},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
        pelvis.x, pelvis.y, pelvis.z, camera.x, camera.y, camera.z
    );
}
