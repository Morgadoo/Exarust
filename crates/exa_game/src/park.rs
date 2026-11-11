//! The parked characters: a dormant character nothing draws is taken out
//! of the simulation until the renderer draws it again.
//!
//! - **Drawn.** Each frame, before the fixed loop, every dormant or parked
//!   character's body learns whether the renderer drew it in the frame
//!   before: whether any of its collision spheres was inside the frustum
//!   of an active 3D camera drawing the world ([`InView`]). The original's
//!   body tick reads the low bits of its node's render-visibility word,
//!   which its scene culling sets every frame for a node in view; a node no
//!   frame has drawn yet reads as not drawn. Exarust's first frame has no
//!   value yet and counts as drawn. The test is geometric (the bodies'
//!   ticks and the camera's place), not Bevy's mesh culling, whose result
//!   depends on when each mesh finished loading and would make lockstep
//!   runs differ.
//! - **Park.** The body tick's dormant branch (a lying sleeper or a resting
//!   statue, the pose word the AI's sleep and the sentry set) parks a body
//!   that was not drawn unless it already is
//!   ([`Ragdoll::park_unseen`], in the posture stage). A parked body is out
//!   of the solver (no integration, no pair, link or level stage) and only
//!   counts its countdown down ([`Ragdoll::park`]).
//! - **Wake.** The AI manager's frame (the simulation loop's pre-step hook,
//!   before its rise) unparks every parked character in the scene that was
//!   drawn ([`wake_parked`]); the golem wake unparks the golem it wakes
//!   (`crate::golem_alarm`).
//!
//! Evidence: the original's character body tick (its dormant branch, after
//! the daze block: with the pose word's dormant bits, an undrawn node sets
//! the root's link iterations to 5, raises the pose word's hidden bit and
//! marks the node tree unless the body's own mark is set; a drawn one sets
//! 5 and clears the hidden bit), the dynamics unit's mark and unmark tree
//! walks (state bit 4 of every body under the node), the sector step's
//! state masks (bit 4 keeps a body from the integration and from every
//! pair and contact pass; such a body rolls its particle sets and counts
//! down to its deep bit), the AI manager's frame (a character of the scene
//! whose own mark is set and whose node was drawn or is in the sun's
//! shadow is unmarked) and the golem wake (unmark after its wake).
//!
//! Not reproduced: the pose word's hidden bit (its readers only change
//! what a parked body's tick computes, which the solver then leaves out),
//! the sun's shadow bit the manager's frame also accepts (a body only the
//! sun's shadow pass saw is unmarked there and marked again by its body
//! tick in the same step, before the solver, so it stays out; Exarust
//! leaves it parked), the scene culling's own test (its node bounds and
//! zones; Exarust tests the body's spheres against the frustum) and the
//! marks of other
//! bodies under the character's node (Exarust's character is one body;
//! held items are bodies of their own, linked by their grips).
//! `EXA_NO_PARK=1` turns the drawn test off, so nothing is parked.
//!
//! The note makes the simulation read the camera, as the original's body
//! tick reads its renderer's word. Exarust's chase camera is not repeatable
//! tick for tick between runs (its wall probe's tilt differs by a few
//! thousandths of a radian), so a parked body at the edge of the view can
//! be unparked a tick apart in two lockstep runs of the same scene;
//! `EXA_NO_PARK=1` gives back a simulation that does not read the camera.

use bevy::camera::primitives::{Frustum, Sphere};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use exa_physics::body::{InView, OutOfScene, Ragdoll};

/// Parks the dormant characters nothing draws.
pub struct ParkPlugin;

impl Plugin for ParkPlugin {
    fn build(&self, app: &mut App) {
        let enabled = std::env::var_os("EXA_NO_PARK").is_none_or(|v| v.is_empty() || v == "0");
        if enabled {
            app.add_systems(First, note_in_view.run_if(any_with_component::<Camera>));
        }
        app.add_systems(
            exa_physics::sim_app::SimPreStep,
            wake_parked.before(crate::rise::rise_the_dead),
        );
        bridge_park_types(app);
    }
}

/// The data the manager frame's wake touches, for the simulation's bridge.
fn bridge_park_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_snapshot::<Ragdoll>()
        .bridge_copy::<OutOfScene>()
        .bridge_copy::<InView>();
}

/// Whether any of the body's collision spheres, where its last tick left
/// them, is inside `frustum`.
fn in_frustum(body: &Ragdoll, frustum: &Frustum) -> bool {
    body.parts.iter().any(|p| {
        let t = p.transform();
        p.spheres.iter().any(|&(centre, radius)| {
            let sphere = Sphere {
                center: t.transform_point3(centre).into(),
                radius,
            };
            frustum.intersects_sphere(&sphere, true)
        })
    })
}

/// The frame's note of which dormant or parked bodies the renderer drew in
/// the last frame: whether any of the body's spheres is inside the frustum
/// of an active 3D camera that draws the world's layer, as the last frame
/// left the frustum. Written only when it changes.
pub fn note_in_view(
    mut commands: Commands,
    bodies: Query<(Entity, &Ragdoll, Option<&InView>)>,
    cameras: Query<(&Camera, &Frustum, Option<&RenderLayers>), With<Camera3d>>,
) {
    let world = RenderLayers::default();
    let frusta: Vec<&Frustum> = cameras
        .iter()
        .filter(|(camera, _, layers)| {
            camera.is_active && layers.is_none_or(|l| l.intersects(&world))
        })
        .map(|(_, frustum, _)| frustum)
        .collect();
    for (entity, body, was) in &bodies {
        let watched = body.parked() || body.drive.as_ref().is_some_and(|d| d.dormant);
        if !watched {
            continue;
        }
        let now = frusta.iter().any(|f| in_frustum(body, f));
        if was.map(|w| w.0) != Some(now) {
            debug!("body {entity:?} drawn: {now}");
            commands.entity(entity).try_insert(InView(now));
        }
    }
}

/// The AI manager frame's wake: every parked character body in the scene
/// that was drawn is unparked (the original's unmark down its node tree),
/// before the frame's rise.
pub fn wake_parked(
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut bodies: Query<(Entity, &mut Ragdoll, Option<&InView>), Without<OutOfScene>>,
) {
    for (entity, mut body, seen) in &mut bodies {
        if body.parked() && seen.is_none_or(|s| s.0) {
            body.unpark();
            info!(
                "parked body {entity:?} drawn: unparked at tick {}",
                ticks.as_ref().map_or(0, |t| t.0)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dormant_body() -> Ragdoll {
        let mut body = Ragdoll::new(&[]).with_drive();
        body.drive.as_mut().unwrap().dormant = true;
        body
    }

    /// An undrawn dormant body is parked once; a drawn one is not.
    #[test]
    fn the_body_tick_parks_an_undrawn_sleeper() {
        let mut body = dormant_body();
        assert!(!body.park_unseen(true));
        assert!(!body.parked());
        assert!(body.park_unseen(false));
        assert!(body.parked());
        // Its own mark is set: no second mark.
        assert!(!body.park_unseen(false));
        // Awake bodies are never parked.
        let mut awake = Ragdoll::new(&[]).with_drive();
        assert!(!awake.park_unseen(false));
    }

    /// The manager frame unparks a drawn body in the scene, and leaves an
    /// undrawn one and one out of the scene parked.
    #[test]
    fn the_manager_frame_wakes_the_drawn() {
        let mut app = App::new();
        app.add_systems(Update, wake_parked);
        let parked = || {
            let mut body = dormant_body();
            body.park();
            body
        };
        let drawn = app.world_mut().spawn((parked(), InView(true))).id();
        let hidden = app.world_mut().spawn((parked(), InView(false))).id();
        let away = app
            .world_mut()
            .spawn((parked(), InView(true), OutOfScene))
            .id();
        let unknown = app.world_mut().spawn(parked()).id();
        app.update();
        let is_parked = |app: &App, e| app.world().get::<Ragdoll>(e).unwrap().parked();
        assert!(!is_parked(&app, drawn));
        assert!(is_parked(&app, hidden));
        assert!(is_parked(&app, away));
        assert!(!is_parked(&app, unknown));
    }
}
