//! `--joint-trace <csv>`: every joint of every simulated body at the end of
//! every 83 Hz tick, one row per body part: its turn from its rest (total,
//! and split into a swing of the bone axis and a twist about it), the
//! clip's turn split the same way and how far the two are apart (a living
//! body's), the link of the part furthest past
//! its slack, how much of its range its slack links use and how open its
//! pivot is, whether a blow landed on it in the tick, and the body's sleep
//! state. Angles are in degrees and lengths in cm. A body in deep sleep is
//! left out (nothing in it changes). Columns: `docs/DEBUGGING.md`.

use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::Path;

use bevy::prelude::*;
use exa_anim::Animator;
use exa_physics::body::inspect::{JointAngles, JointReport, LinkReport};
use exa_physics::body::{BodySystems, BodyTicks, DealtShoves, Ragdoll, Sleep};

use crate::player::PlayerBody;

/// Centimetres per metre.
const CM: f32 = 100.0;
/// Ticks between flushes of the file (a second).
const FLUSH_EVERY: u64 = 83;

/// The trace's columns.
pub const COLUMNS: [&str; 33] = [
    "tick",
    "body",
    "player",
    "living",
    "bone",
    "name",
    "parent",
    "total_deg",
    "swing_deg",
    "twist_deg",
    "swing_axis_x",
    "swing_axis_y",
    "swing_axis_z",
    "anchor_error_deg",
    "clip_total_deg",
    "clip_swing_deg",
    "clip_twist_deg",
    "clip_swing_axis_x",
    "clip_swing_axis_y",
    "clip_swing_axis_z",
    "worst_link",
    "worst_gap_cm",
    "worst_slack_cm",
    "worst_excess_cm",
    "range_pct",
    "range_link",
    "pivot_gap_mm",
    "shoved",
    "shove_speed",
    "sleep",
    "countdown",
    "action",
    "clock",
];

/// The column names, comma-separated.
pub fn header() -> String {
    COLUMNS.join(",")
}

/// One part of one body after a tick.
#[derive(Debug, Clone)]
pub struct JointRow<'a> {
    pub tick: u64,
    /// The body model root.
    pub body: Entity,
    pub player: bool,
    pub living: bool,
    pub bone: usize,
    /// The parent part's bone.
    pub parent: Option<usize>,
    pub joint: Option<JointReport>,
    /// The part's link furthest past its slack.
    pub worst: Option<LinkReport>,
    /// The largest share of its slack a slack link of the part is open,
    /// and that link.
    pub range: Option<(f32, LinkReport)>,
    /// The part's pivot at its origin.
    pub pivot: Option<LinkReport>,
    /// The speed (m/s) of the blows that landed on the part in the tick.
    pub shove: Option<f32>,
    pub sleep: Sleep,
    pub countdown: i32,
    /// The clip and its clock (frames).
    pub action: Option<&'a str>,
    pub clock: Option<f32>,
}

/// Appends `row` to `out` as a line of the trace.
pub fn write_row(out: &mut String, row: &JointRow) {
    fn opt<T: std::fmt::Display>(out: &mut String, v: Option<T>) {
        out.push(',');
        if let Some(v) = v {
            let _ = write!(out, "{v}");
        }
    }
    let deg = |r: f32| format!("{:.2}", r.to_degrees());
    let cm = |m: f32| format!("{:.3}", m * CM);
    let _ = write!(
        out,
        "{},{},{},{},{}",
        row.tick,
        row.body.to_bits(),
        u8::from(row.player),
        u8::from(row.living),
        row.bone
    );
    opt(out, Some(exa_physics::body::inspect::bone_name(row.bone)));
    opt(out, row.parent);
    let turn = |out: &mut String, angles: Option<JointAngles>| {
        opt(out, angles.map(|a| deg(a.total)));
        opt(out, angles.map(|a| deg(a.swing)));
        opt(out, angles.map(|a| deg(a.twist)));
        for k in 0..3 {
            opt(out, angles.map(|a| format!("{:.3}", a.swing_axis[k])));
        }
    };
    turn(out, row.joint.map(|j| j.angles));
    opt(out, row.joint.and_then(|j| j.anchor_error).map(deg));
    turn(out, row.joint.and_then(|j| j.clip));
    opt(
        out,
        row.worst.map(|w| {
            if w.origin {
                "pivot".to_string()
            } else {
                w.rank.to_string()
            }
        }),
    );
    opt(out, row.worst.map(|w| cm(w.gap)));
    opt(out, row.worst.map(|w| cm(w.slack)));
    opt(out, row.worst.map(|w| cm(w.excess())));
    opt(out, row.range.map(|(r, _)| format!("{:.1}", r * 100.0)));
    opt(out, row.range.map(|(_, l)| l.rank));
    opt(out, row.pivot.map(|p| format!("{:.2}", p.gap * 1000.0)));
    opt(out, Some(u8::from(row.shove.is_some())));
    opt(out, row.shove.map(|s| format!("{s:.2}")));
    opt(out, Some(row.sleep.name()));
    opt(out, Some(row.countdown));
    // Commas would split the column.
    opt(out, row.action.map(|a| a.replace(',', ";")));
    opt(out, row.clock.map(|c| format!("{c:.3}")));
    out.push('\n');
}

/// The open joint trace.
#[derive(Resource)]
pub struct JointTrace {
    out: BufWriter<File>,
    ticks: u64,
    failed: bool,
}

impl JointTrace {
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

/// Writes the joint trace after every tick of the body solver, when a
/// [`JointTrace`] is open.
pub struct JointTracePlugin;

impl Plugin for JointTracePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            trace_joints
                .after(BodySystems)
                .run_if(resource_exists::<JointTrace>),
        );
    }
}

fn trace_joints(
    mut trace: ResMut<JointTrace>,
    clock: Res<BodyTicks>,
    dealt: Option<Res<DealtShoves>>,
    bodies: Query<(Entity, &Ragdoll, Option<&Animator>, Has<PlayerBody>)>,
    mut text: Local<String>,
) {
    if trace.failed {
        return;
    }
    text.clear();
    for (root, body, anim, player) in &bodies {
        if body.sleep == Sleep::Deep {
            continue;
        }
        for (i, part) in body.parts.iter().enumerate() {
            let shove = dealt.as_ref().and_then(|d| {
                d.0.iter()
                    .filter(|s| s.body == root && s.bone == part.bone)
                    .map(|s| s.velocity.length())
                    .reduce(|a, b| a + b)
            });
            write_row(
                &mut text,
                &JointRow {
                    tick: clock.0,
                    body: root,
                    player,
                    living: body.drive.is_some(),
                    bone: part.bone,
                    parent: part.parent.map(|j| body.parts[j].bone),
                    joint: body.joint(i),
                    worst: body.worst_link(i),
                    range: body.range_used(i),
                    pivot: body.origin_pivot(i),
                    shove,
                    sleep: body.sleep,
                    countdown: body.countdown(),
                    action: anim.map(|a| a.action.as_str()),
                    clock: anim.map(|a| a.time),
                },
            );
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
        error!("joint trace: {e}; tracing stops");
        trace.failed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_names_every_column() {
        let h = header();
        assert_eq!(h.split(',').count(), COLUMNS.len());
        assert!(h.starts_with("tick,body,player,living,bone,name,parent,total_deg"));
        assert!(h.ends_with("sleep,countdown,action,clock"));
    }

    #[test]
    fn a_row_writes_every_column() {
        let body = Entity::from_raw_u32(7).unwrap();
        let mut out = String::new();
        let row = JointRow {
            tick: 12,
            body,
            player: true,
            living: true,
            bone: 13,
            parent: Some(12),
            joint: Some(JointReport {
                angles: JointAngles {
                    total: std::f32::consts::FRAC_PI_2,
                    swing: std::f32::consts::FRAC_PI_3,
                    twist: -std::f32::consts::FRAC_PI_6,
                    swing_axis: Vec3::Y,
                },
                anchor_error: Some(0.5f32.to_radians()),
                clip: Some(JointAngles {
                    total: 0.2,
                    swing: 0.2,
                    twist: 0.0,
                    swing_axis: Vec3::NEG_Z,
                }),
            }),
            worst: Some(LinkReport {
                index: 30,
                rank: 1,
                origin: false,
                pivot: false,
                slack: 0.12,
                gap: 0.135,
            }),
            range: Some((
                1.125,
                LinkReport {
                    index: 30,
                    rank: 1,
                    origin: false,
                    pivot: false,
                    slack: 0.12,
                    gap: 0.135,
                },
            )),
            pivot: Some(LinkReport {
                index: 31,
                rank: 2,
                origin: true,
                pivot: true,
                slack: 0.0,
                gap: 0.0004,
            }),
            shove: Some(2.5),
            sleep: Sleep::Awake,
            countdown: 256,
            action: Some("Cmbt - Idle, held"),
            clock: Some(3.25),
        };
        write_row(&mut out, &row);
        let line = out.strip_suffix('\n').unwrap();
        let cells: Vec<&str> = line.split(',').collect();
        assert_eq!(cells.len(), COLUMNS.len(), "{line}");
        let at = |name: &str| cells[COLUMNS.iter().position(|c| *c == name).unwrap()];
        assert_eq!(at("tick"), "12");
        assert_eq!(at("body"), body.to_bits().to_string());
        assert_eq!((at("player"), at("living")), ("1", "1"));
        assert_eq!(
            (at("bone"), at("name"), at("parent")),
            ("13", "R forearm", "12")
        );
        assert_eq!(at("total_deg"), "90.00");
        assert_eq!(at("swing_deg"), "60.00");
        assert_eq!(at("twist_deg"), "-30.00");
        assert_eq!(
            (at("swing_axis_x"), at("swing_axis_y"), at("swing_axis_z")),
            ("0.000", "1.000", "0.000")
        );
        assert_eq!(at("anchor_error_deg"), "0.50");
        assert_eq!(
            (at("clip_total_deg"), at("clip_twist_deg")),
            ("11.46", "0.00")
        );
        assert_eq!(at("clip_swing_axis_z"), "-1.000");
        assert_eq!(at("worst_link"), "1");
        assert_eq!(at("worst_gap_cm"), "13.500");
        assert_eq!(at("worst_slack_cm"), "12.000");
        assert_eq!(at("worst_excess_cm"), "1.500");
        assert_eq!((at("range_pct"), at("range_link")), ("112.5", "1"));
        assert_eq!(at("pivot_gap_mm"), "0.40");
        assert_eq!((at("shoved"), at("shove_speed")), ("1", "2.50"));
        assert_eq!((at("sleep"), at("countdown")), ("awake", "256"));
        assert_eq!(at("action"), "Cmbt - Idle; held");
        assert_eq!(at("clock"), "3.250");
        // A root part of a limp body without a clip: its joint columns stay
        // empty.
        out.clear();
        write_row(
            &mut out,
            &JointRow {
                player: false,
                living: false,
                bone: 1,
                parent: None,
                joint: None,
                worst: None,
                range: None,
                pivot: None,
                shove: None,
                sleep: Sleep::Asleep,
                countdown: 0,
                action: None,
                clock: None,
                ..row
            },
        );
        let cells: Vec<&str> = out.trim_end().split(',').collect();
        assert_eq!(cells.len(), COLUMNS.len());
        let at = |name: &str| cells[COLUMNS.iter().position(|c| *c == name).unwrap()];
        assert_eq!(at("name"), "pelvis");
        for empty in [
            "parent",
            "total_deg",
            "anchor_error_deg",
            "clip_swing_deg",
            "worst_link",
            "range_pct",
            "pivot_gap_mm",
            "action",
        ] {
            assert!(at(empty).is_empty(), "{empty}");
        }
        assert_eq!(
            (at("shoved"), at("sleep"), at("countdown")),
            ("0", "asleep", "0")
        );
    }

    /// The trace of a ticking physical body: a row per part per tick, with
    /// its joint measured and its links within their slack.
    #[test]
    fn the_trace_has_a_row_per_part_per_tick() {
        let path = std::env::temp_dir().join(format!("exarust-joints-{}.csv", std::process::id()));
        let (mut app, _) = crate::limp::tests::app(1.0 / 60.0);
        app.insert_resource(JointTrace::create(&path).unwrap())
            .add_plugins(JointTracePlugin);
        while app.world().resource::<BodyTicks>().0 < 10 {
            app.update();
        }
        let ticks = app.world().resource::<BodyTicks>().0;
        app.world_mut().remove_resource::<JointTrace>();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let mut lines = text.lines();
        assert_eq!(lines.next().unwrap(), header());
        let rows: Vec<Vec<&str>> = lines.map(|l| l.split(',').collect()).collect();
        // Two parts: the pelvis and the thigh.
        assert_eq!(rows.len() as u64, ticks * 2);
        let at = |row: &[&str], name: &str| -> String {
            row[COLUMNS.iter().position(|c| *c == name).unwrap()].to_string()
        };
        for (k, pair) in rows.chunks(2).enumerate() {
            let tick = (k + 1).to_string();
            assert!(
                pair.iter()
                    .all(|r| at(r, "tick") == tick && r.len() == COLUMNS.len())
            );
            let thigh = pair.iter().find(|r| at(r, "bone") == "2").unwrap();
            assert_eq!(at(thigh, "parent"), "1");
            assert_eq!(at(thigh, "living"), "1");
            assert_eq!(at(thigh, "worst_link"), "pivot");
            assert!(at(thigh, "total_deg").parse::<f32>().unwrap() < 45.0);
            assert!(at(thigh, "anchor_error_deg").parse::<f32>().is_ok());
            assert!(at(thigh, "worst_excess_cm").parse::<f32>().unwrap() < 0.1);
            assert_eq!(
                (at(thigh, "action"), at(thigh, "sleep")),
                ("Idle".into(), "awake".into())
            );
        }
    }
}
