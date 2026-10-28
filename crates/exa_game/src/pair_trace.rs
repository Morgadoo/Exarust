//! `--pair-trace <csv>`: every contact between two body parts that a tick's
//! body–body detection found (`exa_physics::body::Pairs`), one row per pair
//! of parts: the two bodies and bones (the side that brought its spheres
//! first), whether each is living and whether they are one body, the hits,
//! the pushes summed and the longest (cm), and the mean touched point (cm).
//! Columns: `docs/DEBUGGING.md`.

use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::Path;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::{BodySystems, BodyTicks, Pairs, Ragdoll, SolverStages, Touch};

/// Centimetres per metre.
const CM: f32 = 100.0;
/// Ticks between flushes of the file (a second).
const FLUSH_EVERY: u64 = 83;

/// The trace's columns.
pub const COLUMNS: [&str; 14] = [
    "tick",
    "body_a",
    "bone_a",
    "living_a",
    "body_b",
    "bone_b",
    "living_b",
    "own",
    "hits",
    "push_cm",
    "deepest_cm",
    "x",
    "y",
    "z",
];

/// The column names, comma-separated.
pub fn header() -> String {
    COLUMNS.join(",")
}

/// One side of a contact as the trace names it: its body root, its part's
/// bone and whether the body lives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairSide {
    pub body: Entity,
    pub bone: usize,
    pub living: bool,
}

/// Appends one row.
pub fn write_row(out: &mut String, tick: u64, a: PairSide, b: PairSide, touch: &Touch) {
    let p = touch.point * CM;
    let _ = writeln!(
        out,
        "{tick},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        a.body.to_bits(),
        a.bone,
        u8::from(a.living),
        b.body.to_bits(),
        b.bone,
        u8::from(b.living),
        u8::from(a.body == b.body),
        touch.hits,
        touch.push.length() * CM,
        touch.deepest * CM,
        p.x,
        p.y,
        p.z,
    );
}

/// The open pair trace.
#[derive(Resource)]
pub struct PairTrace {
    out: BufWriter<File>,
    ticks: u64,
    failed: bool,
}

impl PairTrace {
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

/// Writes the pair trace after every tick of the body solver, when a
/// [`PairTrace`] is open.
pub struct PairTracePlugin;

impl Plugin for PairTracePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            trace_pairs
                .after(BodySystems)
                .run_if(resource_exists::<PairTrace>),
        );
    }
}

fn trace_pairs(
    mut trace: ResMut<PairTrace>,
    clock: Res<BodyTicks>,
    stages: Res<SolverStages>,
    bodies: Query<(Entity, &Ragdoll)>,
    mut text: Local<String>,
) {
    if trace.failed {
        return;
    }
    text.clear();
    let touches = stages.get::<Pairs>().map_or(&[][..], Pairs::touches);
    if !touches.is_empty() {
        // The pairs know a body by its first part's entity.
        let by_key: HashMap<Entity, (Entity, &Ragdoll)> = bodies
            .iter()
            .filter_map(|(root, body)| Some((body.parts.first()?.entity, (root, body))))
            .collect();
        let side = |(key, part): (Entity, usize)| {
            let (root, body) = by_key.get(&key)?;
            Some(PairSide {
                body: *root,
                bone: body.parts.get(part)?.bone,
                living: body.drive.is_some(),
            })
        };
        for touch in touches {
            if let (Some(a), Some(b)) = (side(touch.a), side(touch.b)) {
                write_row(&mut text, clock.0, a, b, touch);
            }
        }
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
        error!("pair trace: {e}; tracing stops");
        trace.failed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row names both sides, says they are two bodies, and gives the
    /// pushes and the point in cm.
    #[test]
    fn a_row_gives_both_sides_in_cm() {
        assert_eq!(header().split(',').count(), COLUMNS.len());
        let a = Entity::from_raw_u32(7).unwrap();
        let b = Entity::from_raw_u32(9).unwrap();
        let touch = Touch {
            a: (a, 16),
            b: (b, 1),
            met: true,
            hits: 2,
            point: Vec3::new(0.5, 1.25, -2.0),
            push: Vec3::new(0.0, 0.03, 0.04),
            deepest: 0.03,
            travel: Vec3::ZERO,
            travel2: 0.0,
            struck: [None; 2],
            hit_event: false,
        };
        let mut out = String::new();
        let side = |body, bone, living| PairSide { body, bone, living };
        write_row(&mut out, 12, side(a, 17, true), side(b, 1, false), &touch);
        let row: Vec<&str> = out.trim_end().split(',').collect();
        assert_eq!(row.len(), COLUMNS.len());
        assert_eq!(row[0], "12");
        assert_eq!(&row[2..4], ["17", "1"]);
        assert_eq!(&row[5..9], ["1", "0", "0", "2"]);
        assert_eq!(row[9].parse::<f32>().unwrap(), 5.0);
        assert_eq!(row[10].parse::<f32>().unwrap(), 3.0);
        assert_eq!(&row[11..], ["50", "125", "-200"]);
    }
}
