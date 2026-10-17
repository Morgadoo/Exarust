//! The body overlay (`--debug-bodies`, or F3 in any mode): every simulated
//! character body, and every awake prop body, drawn over its mesh as the
//! solver sees it, and a readout of one character body's joints.
//!
//! - Each part's four particles and the edges of its tetrahedron.
//! - Each link from its point on the part to its point on the parent,
//!   green within its slack, yellow up to 1 cm past it, red beyond (with
//!   its slack drawn as a sphere around the parent's point); a pivot as a
//!   cross in the same colours.
//! - A living body's link muscle anchors, faint: where the clip's joint
//!   turn wants each link's point.
//! - The readout (top left): for the inspected body (the player's, or the
//!   one picked with F5), each joint's turn from its rest, its swing and
//!   twist about the bone axis, how far it is from the clip's turn, and its
//!   link furthest past its slack, the rows coloured as their links are;
//!   the body's sleep state, support, balance, energy and tone; and the
//!   simulation's speed and pause.
//!
//! The parts are drawn between their last two ticks, as their meshes are.
//! Off, nothing runs but the key check. Keys: `docs/DEBUGGING.md`.

use std::fmt::Write as _;

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_physics::body::inspect::{self, Stretch};
use exa_physics::body::{BodyTicks, Ragdoll};
use exa_physics::prop::PropBody;
use exa_render::supersample::SceneCamera;

use crate::player::PlayerBody;
use crate::sim_control::SimControl;

/// Shows and hides the overlay and its readout.
pub const OVERLAY_KEY: KeyCode = KeyCode::F3;
/// Draws every body, or only the inspected one.
pub const ONLY_INSPECTED_KEY: KeyCode = KeyCode::F4;
/// Inspects the body under the cursor, or the next body.
pub const PICK_KEY: KeyCode = KeyCode::F5;

/// Whether the overlay is shown, and of which bodies.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct BodyOverlay {
    pub on: bool,
    /// Only the inspected body is drawn.
    pub only_inspected: bool,
}

/// The body the readout shows (its body model root): the player's unless
/// another is picked.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct InspectedBody(pub Option<Entity>);

/// The overlay's links and marks: drawn over everything, thick.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct BodyGizmos;

/// The overlay's tetrahedra: drawn over everything, thin.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct BodyFrameGizmos;

/// Line widths of the overlay, in pixels of the (supersampled) scene.
const LINE_WIDTH: f32 = 4.0;
const FRAME_WIDTH: f32 = 1.5;
/// Half the size of a particle's cross and a pivot's, metres.
const PARTICLE_MARK: f32 = 0.008;
const PIVOT_MARK: f32 = 0.015;
const ANCHOR_MARK: f32 = 0.01;
/// How near (m) the cursor's ray must pass a part to pick its body.
const PICK_REACH: f32 = 0.3;
/// Rows of the readout: the body, the simulation, the columns, the parts.
const PANEL_HEAD: usize = 3;
const PANEL_ROWS: usize = PANEL_HEAD + 18;
const PANEL_FONT: f32 = 13.0;
/// Seconds between readings of a running simulation, so the numbers can
/// be read.
const PANEL_PERIOD: f32 = 0.1;

const EDGE: Color = Color::srgba(0.75, 0.8, 0.95, 0.4);
const ANCHOR: Color = Color::srgba(0.3, 0.85, 1.0, 0.35);
const TEXT: Color = Color::srgb(0.85, 0.85, 0.85);

/// The colour of a link in the overlay and of its row in the readout.
fn stretch_colour(band: Stretch) -> Color {
    match band {
        Stretch::Within => css::LIME.into(),
        Stretch::Slight => css::YELLOW.into(),
        Stretch::Over => css::RED.into(),
    }
}

/// The readout's text row for a part.
#[derive(Component)]
struct PanelRow(usize);

/// The readout's frame.
#[derive(Component)]
struct Panel;

pub struct BodyOverlayPlugin;

impl Plugin for BodyOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BodyOverlay>()
            .init_resource::<InspectedBody>()
            .insert_gizmo_config(BodyGizmos, on_top(LINE_WIDTH))
            .insert_gizmo_config(BodyFrameGizmos, on_top(FRAME_WIDTH))
            .add_systems(Update, overlay_keys)
            .add_systems(
                Update,
                (keep_inspected, pick_body, draw_overlay, update_panel)
                    .chain()
                    .after(overlay_keys)
                    .run_if(|o: Res<BodyOverlay>| o.on),
            )
            .add_systems(Update, show_panel.run_if(resource_changed::<BodyOverlay>));
    }
}

/// Gizmos drawn over the scene, `width` pixels wide.
fn on_top(width: f32) -> GizmoConfig {
    GizmoConfig {
        depth_bias: -1.0,
        line: GizmoLineConfig { width, ..default() },
        ..default()
    }
}

fn overlay_keys(keys: Option<Res<ButtonInput<KeyCode>>>, mut overlay: ResMut<BodyOverlay>) {
    let Some(keys) = keys else { return };
    if keys.just_pressed(OVERLAY_KEY) {
        overlay.on = !overlay.on;
    }
    if keys.just_pressed(ONLY_INSPECTED_KEY) {
        overlay.only_inspected = !overlay.only_inspected;
    }
}

/// Inspects the player's body, or the first body, while none is chosen or
/// the chosen one is gone.
#[allow(clippy::type_complexity)]
fn keep_inspected(
    mut inspected: ResMut<InspectedBody>,
    bodies: Query<(Entity, Has<PlayerBody>), (With<Ragdoll>, Without<PropBody>)>,
) {
    if inspected.0.is_some_and(|e| bodies.contains(e)) {
        return;
    }
    let player = bodies.iter().find(|(_, p)| *p).map(|(e, _)| e);
    let first = bodies.iter().map(|(e, _)| e).min();
    let chosen = player.or(first);
    if inspected.0 != chosen {
        inspected.0 = chosen;
    }
}

/// F5: inspects the body nearest the cursor's ray, or the next body when
/// none is under it.
fn pick_body(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    time: Res<Time<Fixed>>,
    mut inspected: ResMut<InspectedBody>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), SceneCamera>,
    bodies: Query<(Entity, &Ragdoll), Without<PropBody>>,
) {
    if !keys.is_some_and(|k| k.just_pressed(PICK_KEY)) {
        return;
    }
    let alpha = time.overstep_fraction();
    let ray = windows.single().ok().and_then(|w| {
        let cursor = w.cursor_position()?;
        cameras.iter().find_map(|(camera, eye)| {
            // A supersampled camera draws at a multiple of the window's
            // size.
            let scale = camera
                .logical_viewport_size()
                .map_or(Vec2::ONE, |s| s / w.size().max(Vec2::ONE));
            camera.viewport_to_world(eye, cursor * scale).ok()
        })
    });
    let under = ray.and_then(|ray| {
        bodies
            .iter()
            .flat_map(|(e, body)| {
                (0..body.parts.len()).map(move |i| {
                    let centre: Vec3 = body.drawn_particles(i, alpha).iter().sum::<Vec3>() * 0.25;
                    let along = (centre - ray.origin).dot(*ray.direction).max(0.0);
                    (e, centre.distance(ray.origin + *ray.direction * along))
                })
            })
            .filter(|&(_, d)| d < PICK_REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(e, _)| e)
    });
    let current = inspected.0;
    let next = || {
        let mut all: Vec<Entity> = bodies.iter().map(|(e, _)| e).collect();
        all.sort();
        let at = current.and_then(|c| all.iter().position(|&e| e == c));
        at.map_or(all.first().copied(), |k| {
            all.get((k + 1) % all.len()).copied()
        })
    };
    inspected.0 = under.or_else(next);
    info!("inspecting body {:?}", inspected.0);
}

fn draw_overlay(
    overlay: Res<BodyOverlay>,
    inspected: Res<InspectedBody>,
    time: Res<Time<Fixed>>,
    bodies: Query<(Entity, &Ragdoll, Has<PropBody>)>,
    mut gizmos: Gizmos<BodyGizmos>,
    mut frames: Gizmos<BodyFrameGizmos>,
) {
    let alpha = time.overstep_fraction();
    for (entity, body, prop) in &bodies {
        let chosen = inspected.0 == Some(entity);
        // Of the thousands of props' bodies, only the awake ones.
        if overlay.only_inspected && !chosen || prop && body.asleep() {
            continue;
        }
        for i in 0..body.parts.len() {
            let p = body.drawn_particles(i, alpha);
            for (a, b) in inspect::TETRA_EDGES {
                frames.line(p[a], p[b], EDGE);
            }
            for x in p {
                frames.cross(Isometry3d::from_translation(x), PARTICLE_MARK, EDGE);
            }
        }
        for k in 0..body.links.len() {
            let (Some(report), Some((on_part, on_parent))) =
                (body.link_report(k), body.drawn_link(k, alpha))
            else {
                continue;
            };
            let colour = stretch_colour(report.stretch());
            if report.pivot {
                gizmos.cross(Isometry3d::from_translation(on_parent), PIVOT_MARK, colour);
                if report.stretch() != Stretch::Within {
                    gizmos.line(on_part, on_parent, colour);
                }
            } else {
                gizmos.line(on_part, on_parent, colour);
                if report.stretch() != Stretch::Within {
                    gizmos
                        .sphere(
                            Isometry3d::from_translation(on_parent),
                            report.slack,
                            colour.with_alpha(0.5),
                        )
                        .resolution(24);
                }
            }
            if let Some(anchor) = body.drawn_anchor(k, alpha) {
                gizmos.line(on_part, anchor, ANCHOR);
                gizmos.cross(Isometry3d::from_translation(anchor), ANCHOR_MARK, ANCHOR);
            }
        }
        // A dot over the inspected body's head.
        if chosen && let Some(head) = body.parts.iter().position(|p| p.bone == 10) {
            let top = body.drawn_particles(head, alpha);
            let centre = top.iter().sum::<Vec3>() * 0.25 + Vec3::Y * 0.3;
            gizmos
                .sphere(Isometry3d::from_translation(centre), 0.04, Color::WHITE)
                .resolution(12);
        }
    }
}

fn show_panel(
    mut commands: Commands,
    overlay: Res<BodyOverlay>,
    mut panels: Query<&mut Visibility, With<Panel>>,
) {
    let shown = if overlay.on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Ok(mut v) = panels.single_mut() {
        v.set_if_neq(shown);
        return;
    }
    if !overlay.on {
        return;
    }
    let font = TextFont::from_font_size(PANEL_FONT);
    commands
        .spawn((
            Name::new("JointReadout"),
            Panel,
            Node {
                position_type: PositionType::Absolute,
                left: px(8.0),
                top: px(8.0),
                padding: UiRect::all(px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GlobalZIndex(10),
        ))
        .with_children(|frame| {
            frame
                .spawn((
                    Text::default(),
                    // A table: rows never wrap.
                    TextLayout::no_wrap(),
                    font.clone(),
                    TextColor(TEXT),
                ))
                .with_children(|text| {
                    for row in 0..PANEL_ROWS {
                        text.spawn((
                            TextSpan::default(),
                            font.clone(),
                            TextColor(TEXT),
                            PanelRow(row),
                        ));
                    }
                });
        });
}

/// One part's row of the readout: its joint's turn from its rest (total,
/// swing, twist and the swing's axis), how far that is from the clip's
/// turn, how much of its range its slack links use, how open its pivot is
/// (mm), and how far its worst link is past its slack, coloured by that
/// link.
fn part_row(body: &Ragdoll, i: usize) -> (String, Color) {
    let part = &body.parts[i];
    let mut line = format!("{:>2} {:<11}", part.bone, inspect::bone_name(part.bone));
    match body.joint(i) {
        Some(j) => {
            let a = j.angles;
            let _ = write!(
                line,
                " {:>6.1} {:>6.1} {:>6.1} [{:+.1} {:+.1} {:+.1}]",
                a.total.to_degrees(),
                a.swing.to_degrees(),
                a.twist.to_degrees(),
                a.swing_axis.x,
                a.swing_axis.y,
                a.swing_axis.z
            );
            match j.anchor_error {
                Some(e) => {
                    let _ = write!(line, " {:>6.1}", e.to_degrees());
                }
                None => line.push_str("      -"),
            }
        }
        None => {
            let _ = write!(line, "{:>45}", "-");
        }
    }
    match body.range_used(i) {
        Some((used, l)) => {
            let _ = write!(line, " {:>5.0}% l{}", used * 100.0, l.rank);
        }
        None => line.push_str("         -"),
    }
    match body.origin_pivot(i) {
        Some(p) => {
            let _ = write!(line, " {:>5.1}", p.gap * 1000.0);
        }
        None => line.push_str("     -"),
    }
    let colour = match body.worst_link(i) {
        Some(w) => {
            let _ = write!(line, " {:>+7.1} cm", w.excess() * 100.0);
            match w.stretch() {
                Stretch::Within => TEXT,
                band => stretch_colour(band),
            }
        }
        None => TEXT,
    };
    line.push('\n');
    (line, colour)
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_panel(
    inspected: Res<InspectedBody>,
    clock: Res<BodyTicks>,
    virt: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    control: Option<Res<SimControl>>,
    bodies: Query<(&Ragdoll, Has<PlayerBody>), Without<PropBody>>,
    mut rows: Query<(&PanelRow, &mut TextSpan, &mut TextColor)>,
    mut read_at: Local<Option<f32>>,
) {
    // Paused, the rows only change when a tick is stepped.
    let now = real.elapsed_secs();
    if !virt.is_paused() && read_at.is_some_and(|at| now - at < PANEL_PERIOD) {
        return;
    }
    *read_at = Some(now);
    let body = inspected.0.and_then(|e| Some((e, bodies.get(e).ok()?)));
    let mut text = vec![(String::new(), TEXT); PANEL_ROWS];
    match body {
        Some((e, (body, player))) => {
            // The id is the traces' `body` column.
            let mut head = format!(
                "body {e} (id {}){}  {}  {} ({})",
                e.to_bits(),
                if player { " player" } else { "" },
                if body.drive.is_some() {
                    "living"
                } else {
                    "limp"
                },
                body.sleep.name(),
                body.countdown()
            );
            if let Some(d) = &body.drive {
                let _ = write!(
                    head,
                    "  support {:.2}  energy {:.2}  tone {:.2}",
                    d.support(),
                    d.energy,
                    d.tone
                );
            }
            if let Some(d) = &body.drive {
                let b = d.balance();
                let _ = write!(head, "  balance {:.2} ({})", b.value, b.counter());
            }
            head.push('\n');
            text[0].0 = head;
            let mut order: Vec<usize> = (0..body.parts.len()).collect();
            order.sort_by_key(|&i| body.parts[i].bone);
            for (row, &i) in order.iter().take(PANEL_ROWS - PANEL_HEAD).enumerate() {
                text[PANEL_HEAD + row] = part_row(body, i);
            }
        }
        None => text[0].0 = "no simulated body\n".into(),
    }
    let mut sim = format!("tick {}  sim x{:.2}", clock.0, virt.relative_speed());
    if virt.is_paused() {
        sim.push_str("  PAUSED (P resume, . step)");
        if let Some(reason) = control.as_ref().and_then(|c| c.reason.as_ref()) {
            let _ = write!(sim, " at {reason}");
        }
    }
    sim.push('\n');
    text[1].0 = sim;
    text[2].0 = format!(
        "{:<14} {:>6} {:>6} {:>6} {:<16} {:>6} {:>9} {:>5} {:>10}\n",
        "bone", "total", "swing", "twist", "swing axis", "clip", "range", "pivot", "past slack"
    );
    for (row, mut span, mut colour) in &mut rows {
        let Some((line, c)) = text.get(row.0) else {
            continue;
        };
        if span.0 != *line {
            span.0.clone_from(line);
        }
        colour.set_if_neq(TextColor(*c));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_coloured_by_their_stretch() {
        let colour = |excess: f32| stretch_colour(inspect::stretch(excess));
        assert_eq!(colour(-0.02), Color::from(css::LIME));
        assert_eq!(colour(0.0005), Color::from(css::LIME));
        assert_eq!(colour(0.005), Color::from(css::YELLOW));
        assert_eq!(colour(0.05), Color::from(css::RED));
    }
}
