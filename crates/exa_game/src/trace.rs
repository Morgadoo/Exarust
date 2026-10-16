//! `--trace <csv>`: the body state at the end of every 83 Hz tick, one row
//! per traced body (the player's, or, without a player, every simulated
//! body). Lengths are in the original's centimetres and the clip clock in
//! frames, in the columns of the retail body probe, so that the two can be
//! compared row by row. What Exarust does not simulate yet is left empty.
//! Columns: `docs/formats/RSQ.md` (Physical body, per-tick trace).

use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::Path;

use bevy::prelude::*;
use exa_anim::{Animator, BodyPart, ClipPose, Motions, Stature};
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll, world_with};

use crate::balance::Balance;
use crate::limp::clip_pose;
use crate::player::PlayerBody;

/// Centimetres per metre.
const CM: f32 = 100.0;
/// Bones of the human skeleton.
const BONES: usize = 18;
/// Ticks between flushes of the file (a second).
const FLUSH_EVERY: u64 = 83;
/// The body's scalars, in the probe's order.
const SCALARS: [&str; 11] = [
    "pelvis_height",
    "root_clip_height",
    "frame_height",
    "target_pelvis_height",
    "stance",
    "tone_target",
    "tone",
    "energy",
    "support",
    "balance",
    "balance_counter",
];
/// A part target's columns: its place (cm) and its rotation.
const TARGET: [&str; 7] = ["x", "y", "z", "qx", "qy", "qz", "qw"];

/// The open trace.
#[derive(Resource)]
pub struct BodyTrace {
    out: BufWriter<File>,
    ticks: u64,
    failed: bool,
}

impl BodyTrace {
    /// Creates the file and writes the header.
    pub fn create(path: &Path) -> std::io::Result<Self> {
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(out, "{}", header())?;
        Ok(Self {
            out,
            ticks: 0,
            failed: false,
        })
    }
}

/// Writes the trace after every tick of the body solver, when a
/// [`BodyTrace`] is open.
pub struct TracePlugin;

impl Plugin for TracePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            trace_bodies
                .after(BodySystems)
                .run_if(resource_exists::<BodyTrace>),
        );
    }
}

/// The column names, comma-separated.
pub fn header() -> String {
    let mut columns: Vec<String> = vec!["tick".into(), "body".into()];
    for k in 0..4 {
        for axis in ["x", "y", "z"] {
            columns.push(format!("pelvis_p{k}_{axis}"));
        }
    }
    columns.extend(SCALARS.iter().map(|s| s.to_string()));
    for bone in 1..=BONES {
        for c in TARGET {
            columns.push(format!("target{bone}_{c}"));
        }
    }
    for foot in ["r", "l"] {
        columns.push(format!("foot_{foot}_flags"));
        for axis in ["x", "y", "z"] {
            columns.push(format!("foot_{foot}_anchor_{axis}"));
        }
    }
    columns.push("motion".into());
    columns.push("clock".into());
    columns.join(",")
}

/// One row, column by column.
struct Row(String);

impl Row {
    fn new(tick: u64, body: Entity) -> Self {
        Self(format!("{tick},{}", body.to_bits()))
    }

    fn value(&mut self, v: Option<f32>) {
        self.0.push(',');
        if let Some(v) = v {
            let _ = write!(self.0, "{v}");
        }
    }

    fn whole(&mut self, v: Option<usize>) {
        self.0.push(',');
        if let Some(v) = v {
            let _ = write!(self.0, "{v}");
        }
    }

    /// Columns Exarust has no value for yet.
    fn empty(&mut self, n: usize) {
        for _ in 0..n {
            self.0.push(',');
        }
    }
}

#[allow(clippy::type_complexity)]
fn trace_bodies(
    mut trace: ResMut<BodyTrace>,
    clock: Res<BodyTicks>,
    motions: Option<Res<Motions>>,
    bodies: Query<(
        Entity,
        &Ragdoll,
        Option<&Animator>,
        Option<&Stature>,
        Option<&Balance>,
        Has<PlayerBody>,
    )>,
    parents: Query<&ChildOf>,
    nodes: Query<(&Transform, Option<&BodyPart>)>,
) {
    if trace.failed {
        return;
    }
    let player = bodies.iter().any(|(.., p)| p);
    let mut text = String::new();
    for (root, body, anim, stature, balance, is_player) in &bodies {
        if player && !is_player {
            continue;
        }
        text.push_str(&row(
            clock.0,
            root,
            body,
            anim,
            stature.map_or(1.0, |s| s.0),
            balance,
            motions.as_deref(),
            &parents,
            &nodes,
        ));
        text.push('\n');
    }
    let ticks = trace.ticks + 1;
    trace.ticks = ticks;
    let written = trace.out.write_all(text.as_bytes()).and_then(|()| {
        if ticks.is_multiple_of(FLUSH_EVERY) {
            trace.out.flush()
        } else {
            Ok(())
        }
    });
    if let Err(e) = written {
        error!("body trace: {e}; tracing stops");
        trace.failed = true;
    }
}

#[allow(clippy::too_many_arguments)]
fn row(
    tick: u64,
    root: Entity,
    body: &Ragdoll,
    anim: Option<&Animator>,
    stature: f32,
    balance: Option<&Balance>,
    motions: Option<&Motions>,
    parents: &Query<&ChildOf>,
    nodes: &Query<(&Transform, Option<&BodyPart>)>,
) -> String {
    let mut row = Row::new(tick, root);
    let pelvis = body.part_of(1);
    for k in 0..4 {
        let p = pelvis.map(|p| p.positions[k] * CM);
        for axis in 0..3 {
            row.value(p.map(|p| p[axis]));
        }
    }
    let root_world = world_with(root, parents, |e| {
        nodes.get(e).ok().map(|(t, _)| t.compute_affine())
    });
    let pose = match (motions, anim) {
        (Some(m), Some(a)) => {
            m.0.clip(&a.moveset, &a.action)
                .and_then(|c| ClipPose::at(c, a.time, stature))
        }
        _ => None,
    };
    let drive = body.drive.as_ref();
    // The scalars, in the order of `SCALARS`. The original's frame sits at
    // the pelvis height; Exarust's is the body root, at the feet.
    row.value(pelvis.map(|p| p.transform().translation.y * CM));
    row.value(pose.as_ref().zip(pelvis).map(|(pose, p)| {
        clip_pose(p.entity, root, pose, parents, nodes)
            .translation
            .y
            * CM
    }));
    row.value(Some(root_world.translation.y * CM));
    // The lift servo's target is not simulated yet.
    row.empty(1);
    row.value(drive.map(|d| d.stance()));
    row.value(drive.map(|d| d.tone_target()));
    row.value(drive.map(|d| d.tone));
    row.value(drive.map(|d| d.energy));
    row.value(drive.map(|d| d.support()));
    row.value(balance.map(|b| b.value));
    row.value(balance.map(Balance::counter));
    let targets = drive.map_or(&[][..], |d| d.targets());
    for bone in 1..=BONES {
        let target = body
            .parts
            .iter()
            .position(|p| p.bone == bone)
            .and_then(|i| targets.get(i));
        match target {
            Some(t) => {
                let (_, r, x) = t.to_scale_rotation_translation();
                let x = x * CM;
                for v in [x.x, x.y, x.z, r.x, r.y, r.z, r.w] {
                    row.value(Some(v));
                }
            }
            None => row.empty(TARGET.len()),
        }
    }
    // Foot contacts are not simulated yet.
    row.empty(8);
    row.whole(
        anim.zip(motions)
            .and_then(|(a, m)| m.0.action_index(&a.action)),
    );
    row.value(anim.map(|a| a.time));
    row.0
}

#[cfg(test)]
mod tests {
    use exa_physics::body::BODY_HZ;

    use super::*;

    /// The trace has a row per tick for the body, with the header's
    /// columns: its clip clock advancing by a tick's worth of frames, its
    /// tone, stance factor, tone target and support (a standing human: 1,
    /// 0.5, 0.5 and 1), its targets in cm (the pelvis's at the clip root's
    /// height above the body root), and empty columns for what is not
    /// simulated yet.
    #[test]
    fn the_trace_has_a_row_per_tick() {
        let path = std::env::temp_dir().join(format!("exarust-trace-{}.csv", std::process::id()));
        let (mut app, _) = crate::limp::tests::app(1.0 / 60.0);
        app.insert_resource(BodyTrace::create(&path).unwrap())
            .add_plugins(TracePlugin);
        while app.world().resource::<BodyTicks>().0 < 20 {
            app.update();
        }
        let ticks = app.world().resource::<BodyTicks>().0;
        // Closing the trace flushes it.
        app.world_mut().remove_resource::<BodyTrace>();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let mut lines = text.lines();
        let columns: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(
            columns.len(),
            2 + 12 + SCALARS.len() + BONES * TARGET.len() + 8 + 2
        );
        let rows: Vec<Vec<&str>> = lines.map(|l| l.split(',').collect()).collect();
        assert_eq!(rows.len() as u64, ticks);
        let at = |name: &str| columns.iter().position(|c| *c == name).unwrap();
        let number = |row: &[&str], name: &str| row[at(name)].parse::<f32>().unwrap();
        for (k, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), columns.len());
            assert_eq!(row[at("tick")], (k + 1).to_string());
            let clock = (k + 1) as f32 * 60.0 / BODY_HZ;
            assert!((number(row, "clock") - clock).abs() < 1e-3);
            assert_eq!(row[at("motion")], "0");
            assert_eq!(number(row, "tone"), 1.0);
            assert_eq!(number(row, "stance"), 0.5);
            assert_eq!(number(row, "tone_target"), 0.5);
            assert_eq!(number(row, "support"), 1.0);
            assert!(number(row, "pelvis_p3_z").is_finite());
            let lifted = number(row, "frame_height") + number(row, "root_clip_height");
            assert!((number(row, "target1_y") - lifted).abs() < 1e-2);
            assert!(row[at("target3_x")].is_empty() && row[at("target_pelvis_height")].is_empty());
        }
    }
}
