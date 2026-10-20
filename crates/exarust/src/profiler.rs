//! The frame profiler: where each frame's time goes, as engines' "stat
//! unit" views show it. `--profiler` turns it on with its overlay shown
//! (F10 toggles the overlay); `--perf-log <seconds>` turns it on and logs a
//! report every so often.
//!
//! - Main thread: a timestamp between each of the main schedules (First,
//!   PreUpdate, the fixed ticks with their count, Update, PostUpdate, Last)
//!   and the rest of the frame (extracting into the render world and
//!   waiting for it).
//! - Render thread: the render schedule's time.
//! - GPU: Bevy's render diagnostics (timestamp queries inside its passes)
//!   and Exarust's own passes (`exa_cube_shadows`, `exa_gi`). Bevy 0.20
//!   keys a pass by name only, so the supersampled scene's passes and the
//!   window's display passes share a line; and its spans around whole views
//!   (`auxiliary_views`, the point-light shadow views) are submitted apart
//!   from the passes they enclose, so they read as nothing on the GPU. GPU
//!   load as a whole is `nvidia-smi`'s.
//!
//! Rules: `docs/PERFORMANCE.md` (Frame profiler).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::app::MainScheduleOrder;
use bevy::diagnostic::DiagnosticsStore;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use bevy::render::{Render, RenderApp, RenderSystems};
use exa_ui::LoadingStatus;

use crate::Options;

/// Shows and hides the overlay.
pub const OVERLAY_KEY: KeyCode = KeyCode::F10;

/// Frames kept for the graph and the percentiles.
const HISTORY: usize = 240;
/// Frames drawn in the graph.
const GRAPH_BARS: usize = 120;
/// Graph height (px) and the frame time at its top (ms).
const GRAPH_HEIGHT: f32 = 90.0;
const GRAPH_TOP_MS: f32 = 66.7;
/// Overlay text refresh interval (s).
const TEXT_EVERY: f32 = 0.25;
/// GPU spans listed.
const TOP_SPANS: usize = 10;

/// The main schedules timed, in order; a probe runs before the first and
/// after each.
const STAGES: [&str; 6] = [
    "First",
    "PreUpdate",
    "Fixed",
    "Update",
    "PostUpdate",
    "Last",
];

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Probe(usize);

/// One frame's main-thread times (ms).
#[derive(Clone, Copy, Debug, Default)]
struct FrameRecord {
    total: f32,
    stages: [f32; 6],
    /// From the end of Last to the next First: extract, present, waits.
    rest: f32,
    ticks: u32,
    render: f32,
}

#[derive(Resource, Default)]
struct Probes {
    marks: [Option<Instant>; 7],
    last_start: Option<Instant>,
    ticks: u32,
}

#[derive(Resource, Default)]
struct FrameHistory {
    frames: Vec<FrameRecord>,
}

/// The render thread's last schedule time (µs), shared with the render world.
#[derive(Resource, Clone, Default)]
struct RenderTime(Arc<AtomicU64>);

/// When the render schedule started (render world).
#[derive(Resource, Default)]
struct RenderStart(Mutex<Option<Instant>>);

/// Whether the overlay is shown.
#[derive(Resource, Debug, Default)]
pub struct ProfilerOverlay {
    pub on: bool,
}

#[derive(Component)]
struct OverlayRoot;

#[derive(Component)]
struct OverlayText;

#[derive(Component)]
struct GraphBar(usize);

pub(crate) struct ProfilerPlugin;

impl Plugin for ProfilerPlugin {
    fn build(&self, app: &mut App) {
        let (on, shown) = app
            .world()
            .get_resource::<Options>()
            .map_or((false, false), |o| {
                (o.profiler || o.perf_log.is_some(), o.profiler)
            });
        if !on {
            return;
        }
        let render_time = RenderTime::default();
        app.add_plugins(RenderDiagnosticsPlugin)
            .insert_resource(ProfilerOverlay { on: shown })
            .insert_resource(render_time.clone())
            .init_resource::<Probes>()
            .init_resource::<FrameHistory>();
        // A probe schedule before First and after each timed schedule.
        {
            let mut main = app.world_mut().resource_mut::<MainScheduleOrder>();
            main.insert_before(First, Probe(0));
            main.insert_after(First, Probe(1));
            main.insert_after(PreUpdate, Probe(2));
            main.insert_after(RunFixedMainLoop, Probe(3));
            main.insert_after(Update, Probe(4));
            main.insert_after(PostUpdate, Probe(5));
            main.insert_after(Last, Probe(6));
        }
        for i in 0..=6 {
            app.add_systems(
                Probe(i),
                move |probes: ResMut<Probes>,
                      history: ResMut<FrameHistory>,
                      render: Res<RenderTime>| {
                    probe(i, probes, history, render)
                },
            );
        }
        app.add_systems(FixedFirst, |mut p: ResMut<Probes>| p.ticks += 1)
            .add_systems(
                Update,
                (toggle_overlay, show_overlay, update_overlay).chain(),
            )
            .add_systems(Last, log_report);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(render_time)
            .init_resource::<RenderStart>()
            .add_systems(
                Render,
                (
                    render_begin.in_set(RenderSystems::ExtractCommands),
                    render_end.in_set(RenderSystems::PostCleanup),
                ),
            );
    }
}

fn probe(
    i: usize,
    mut probes: ResMut<Probes>,
    mut history: ResMut<FrameHistory>,
    render: Res<RenderTime>,
) {
    let now = Instant::now();
    if i == 0 {
        if let (Some(start), Some(end)) = (probes.last_start, probes.marks[6]) {
            let ms = |a: Instant, b: Instant| (b - a).as_secs_f32() * 1000.0;
            let mut stages = [0.0; 6];
            for (k, stage) in stages.iter_mut().enumerate() {
                if let (Some(a), Some(b)) = (probes.marks[k], probes.marks[k + 1]) {
                    *stage = ms(a, b);
                }
            }
            let record = FrameRecord {
                total: ms(start, now),
                stages,
                rest: ms(end, now),
                ticks: probes.ticks,
                render: render.0.load(Ordering::Relaxed) as f32 / 1000.0,
            };
            if history.frames.len() == HISTORY {
                history.frames.remove(0);
            }
            history.frames.push(record);
        }
        probes.last_start = Some(now);
        probes.marks = [None; 7];
        probes.ticks = 0;
    }
    probes.marks[i] = Some(now);
}

fn render_begin(start: Res<RenderStart>) {
    if let Ok(mut s) = start.0.lock() {
        *s = Some(Instant::now());
    }
}

fn render_end(start: Res<RenderStart>, time: Res<RenderTime>) {
    if let Some(begun) = start.0.lock().ok().and_then(|s| *s) {
        time.0
            .store(begun.elapsed().as_micros() as u64, Ordering::Relaxed);
    }
}

/// The value at quantile `q` (0..=1) of sorted `v`.
pub(crate) fn quantile(v: &[f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let i = ((v.len() - 1) as f32 * q).round() as usize;
    v[i.min(v.len() - 1)]
}

/// The GPU spans by average time (ms), longest first: Bevy's top-level
/// passes and Exarust's spans (`render/<name>/elapsed_gpu`).
fn gpu_spans(store: &DiagnosticsStore) -> Vec<(String, f64)> {
    let mut spans: Vec<(String, f64)> = store
        .iter()
        .filter_map(|d| {
            let path = d.path().as_str();
            let name = path.strip_prefix("render/")?.strip_suffix("/elapsed_gpu")?;
            Some((name.to_string(), d.average()?))
        })
        .collect();
    spans.sort_by(|a, b| b.1.total_cmp(&a.1));
    spans
}

/// The report: frame times over `frames`, the main-thread split, the render
/// thread and the GPU spans.
fn report(frames: &[FrameRecord], store: Option<&DiagnosticsStore>) -> Vec<String> {
    if frames.is_empty() {
        return vec!["profiler: waiting for frames".into()];
    }
    let n = frames.len() as f32;
    let mut totals: Vec<f32> = frames.iter().map(|f| f.total).collect();
    totals.sort_by(f32::total_cmp);
    let mean = totals.iter().sum::<f32>() / n;
    let avg = |f: &dyn Fn(&FrameRecord) -> f32| frames.iter().map(f).sum::<f32>() / n;
    let mut lines = vec![
        format!(
            "frame {mean:.1} ms ({:.0} FPS)  p50 {:.1}  p95 {:.1}  p99 {:.1}  max {:.1}",
            1000.0 / mean.max(1e-3),
            quantile(&totals, 0.5),
            quantile(&totals, 0.95),
            quantile(&totals, 0.99),
            totals.last().copied().unwrap_or(0.0),
        ),
        "main thread (ms)".into(),
    ];
    let main: f32 = (0..6).map(|k| avg(&|f| f.stages[k])).sum();
    for (k, stage) in STAGES.iter().enumerate() {
        let ms = avg(&|f| f.stages[k]);
        let extra = if k == 2 {
            format!("  ({:.1} ticks)", avg(&|f| f.ticks as f32))
        } else {
            String::new()
        };
        lines.push(format!("  {stage:<11}{ms:>6.2}{extra}"));
    }
    lines.push(format!("  {:<11}{main:>6.2}", "schedules"));
    lines.push(format!(
        "  {:<11}{:>6.2}  (extract, present, waits)",
        "rest",
        avg(&|f| f.rest)
    ));
    lines.push(format!("render thread {:.2} ms", avg(&|f| f.render)));
    if let Some(store) = store {
        let spans = gpu_spans(store);
        if spans.is_empty() {
            lines.push("GPU: no timestamps yet".into());
        } else {
            lines.push("GPU (ms, averaged)".into());
            for (name, ms) in spans.iter().take(TOP_SPANS) {
                lines.push(format!("  {name:<28}{ms:>6.2}"));
            }
        }
    }
    lines
}

fn log_report(
    options: Res<Options>,
    time: Res<Time<Real>>,
    loading: Option<Res<LoadingStatus>>,
    store: Option<Res<DiagnosticsStore>>,
    history: Res<FrameHistory>,
    mut last: Local<Option<f32>>,
    mut since_frames: Local<usize>,
) {
    let Some(interval) = options.perf_log else {
        return;
    };
    if loading.is_some() {
        *last = None;
        return;
    }
    let now = time.elapsed_secs();
    let start = *last.get_or_insert(now);
    *since_frames += 1;
    if now - start < interval {
        return;
    }
    *last = Some(now);
    let n = (*since_frames).min(history.frames.len());
    *since_frames = 0;
    let frames = &history.frames[history.frames.len() - n..];
    for line in report(frames, store.as_deref()) {
        info!("perf: {line}");
    }
    // Every diagnostic Bevy keeps, for finding a pass by name.
    if std::env::var_os("EXA_PERF_ALL").is_some()
        && let Some(store) = &store
    {
        for d in store.iter() {
            info!("perf: diagnostic {} = {:?}", d.path().as_str(), d.average());
        }
    }
}

fn toggle_overlay(keys: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<ProfilerOverlay>) {
    if keys.just_pressed(OVERLAY_KEY) {
        overlay.on = !overlay.on;
    }
}

fn show_overlay(
    mut commands: Commands,
    overlay: Res<ProfilerOverlay>,
    mut roots: Query<&mut Visibility, With<OverlayRoot>>,
) {
    let shown = if overlay.on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Ok(mut v) = roots.single_mut() {
        v.set_if_neq(shown);
        return;
    }
    if !overlay.on {
        return;
    }
    commands
        .spawn((
            Name::new("FrameProfiler"),
            OverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(8.0),
                bottom: px(8.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6.0)),
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            GlobalZIndex(15),
        ))
        .with_children(|root| {
            root.spawn((
                OverlayText,
                Text::new("profiler"),
                TextLayout::no_wrap(),
                TextFont::from_font_size(13.0),
                TextColor(Color::srgb(0.85, 0.85, 0.8)),
            ));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexEnd,
                height: px(GRAPH_HEIGHT),
                column_gap: px(1.0),
                border: UiRect::top(px(1.0)),
                ..default()
            })
            .with_children(|graph| {
                for i in 0..GRAPH_BARS {
                    graph.spawn((
                        GraphBar(i),
                        Node {
                            width: px(3.0),
                            height: px(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.8, 0.3)),
                    ));
                }
            });
        });
}

/// The bar colour of a frame: green within 60 FPS, yellow within 30, red
/// beyond.
fn bar_colour(ms: f32) -> Color {
    if ms <= 1000.0 / 59.0 {
        Color::srgb(0.3, 0.8, 0.3)
    } else if ms <= 1000.0 / 29.0 {
        Color::srgb(0.9, 0.8, 0.2)
    } else {
        Color::srgb(0.9, 0.3, 0.2)
    }
}

#[allow(clippy::too_many_arguments)]
fn update_overlay(
    overlay: Res<ProfilerOverlay>,
    time: Res<Time<Real>>,
    store: Option<Res<DiagnosticsStore>>,
    history: Res<FrameHistory>,
    mut text: Query<&mut Text, With<OverlayText>>,
    mut bars: Query<(&GraphBar, &mut Node, &mut BackgroundColor)>,
    mut refreshed: Local<f32>,
) {
    if !overlay.on {
        return;
    }
    let frames = &history.frames;
    let first = frames.len().saturating_sub(GRAPH_BARS);
    for (bar, mut node, mut colour) in &mut bars {
        let ms = frames.get(first + bar.0).map_or(0.0, |f| f.total);
        node.height = px(GRAPH_HEIGHT * (ms / GRAPH_TOP_MS).min(1.0));
        colour.0 = bar_colour(ms);
    }
    let now = time.elapsed_secs();
    if now - *refreshed < TEXT_EVERY {
        return;
    }
    *refreshed = now;
    if let Ok(mut t) = text.single_mut() {
        let mut lines = report(frames, store.as_deref());
        lines.push(format!(
            "graph: last {GRAPH_BARS} frames, top {GRAPH_TOP_MS:.0} ms (F10 hides)"
        ));
        t.0 = lines.join("\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_pick_from_the_sorted_times() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(quantile(&v, 0.0), 1.0);
        assert_eq!(quantile(&v, 0.5), 3.0);
        assert_eq!(quantile(&v, 1.0), 5.0);
        assert_eq!(quantile(&[], 0.5), 0.0);
    }

    #[test]
    fn the_report_splits_the_main_thread() {
        let frame = FrameRecord {
            total: 20.0,
            stages: [1.0, 2.0, 3.0, 4.0, 5.0, 1.0],
            rest: 4.0,
            ticks: 2,
            render: 7.5,
        };
        let lines = report(&[frame; 4], None);
        assert!(
            lines[0].starts_with("frame 20.0 ms (50 FPS)"),
            "{}",
            lines[0]
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Fixed") && l.contains("(2.0 ticks)"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("schedules") && l.contains("16.00"))
        );
        assert!(lines.iter().any(|l| l == "render thread 7.50 ms"));
    }

    #[test]
    fn bars_turn_red_past_thirty_fps() {
        assert_eq!(bar_colour(10.0), Color::srgb(0.3, 0.8, 0.3));
        assert_eq!(bar_colour(30.0), Color::srgb(0.9, 0.8, 0.2));
        assert_eq!(bar_colour(50.0), Color::srgb(0.9, 0.3, 0.2));
    }
}
