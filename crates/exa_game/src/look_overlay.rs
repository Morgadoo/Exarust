//! The navigation overlay's part for the player (`--debug-nav`, F6, with
//! the grid's cells, `exa_world::nav::view`): its behaviour's last line
//! walk on the grid, from its feet. The way asked runs faint, white for the
//! look-ahead, yellow for a soft stop's goal, cyan for a footwork step's
//! target; the walk to where it ended is green when free and orange when
//! something blocked it, with a ring at the end. A goal the body walks to
//! (out of free walk) is marked with a magenta cross.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use exa_physics::body::locomotion::Locomotion;
use exa_world::nav::NavOverlay;

use crate::player::PlayerBody;
use crate::steering::Asked;

pub struct LookOverlayPlugin;

impl Plugin for LookOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw_line_walks.run_if(|o: Option<Res<NavOverlay>>| o.is_some_and(|o| o.on)),
        );
    }
}

/// Lines float this far over the feet (m).
const LIFT: f32 = 0.05;
/// The end's ring and the goal's cross, half sizes (m).
const RING: f32 = 0.08;
const CROSS: f32 = 0.1;

fn flat(at: Vec3) -> Isometry3d {
    Isometry3d::new(at, Quat::from_rotation_x(-FRAC_PI_2))
}

fn draw_line_walks(bodies: Query<(&PlayerBody, &Locomotion)>, mut gizmos: Gizmos) {
    let up = Vec3::Y * LIFT;
    for (pb, req) in &bodies {
        if let Some(l) = pb.line_asked() {
            let asked = match l.asked {
                Asked::LookAhead => Color::WHITE,
                Asked::SoftStop => Color::srgb(1.0, 1.0, 0.0),
                Asked::Step => Color::srgb(0.0, 1.0, 1.0),
            };
            let walked = if l.blocked {
                Color::srgb(1.0, 0.5, 0.0)
            } else {
                Color::srgb(0.0, 1.0, 0.0)
            };
            let end = l.end.with_y(l.from.y) + up;
            gizmos.line(
                l.from + up,
                l.to.with_y(l.from.y) + up,
                asked.with_alpha(0.35),
            );
            gizmos.line(l.from + up, end, walked);
            gizmos.circle(flat(end), RING, walked);
        }
        if req.moving && !req.free_walk {
            let g = req.goal + up;
            let magenta = Color::srgb(1.0, 0.0, 1.0);
            gizmos.line(g - Vec3::X * CROSS, g + Vec3::X * CROSS, magenta);
            gizmos.line(g - Vec3::Z * CROSS, g + Vec3::Z * CROSS, magenta);
        }
    }
}
