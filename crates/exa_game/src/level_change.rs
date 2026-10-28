//! The flow's bridge to an in-process level change.
//!
//! [`crate::flow::Step::LoadLevel`] and `Step::TeardownWorld` used to be
//! logged and left to the host. They now become a
//! [`exa_world::level_lifecycle::LevelChangeRequest`]: the world's teardown
//! runs in `exa_world`, and the binary's viewer loads the destination through
//! the same [`exa_world::LevelLoad`] stages as a start-up load. A
//! [`PlayerPlacement`] carries the flow's `Step::PlacePlayer` across the load,
//! so the player (whose entity, record and possessions survive the change) is
//! put at the original's spawn after the level is in.

use bevy::prelude::*;
use exa_world::level_lifecycle::LevelChangeRequest;

/// Where the player is put after a level change, in metres and radians.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlayerPlacement {
    pub position: Vec3,
    pub yaw: f32,
}

impl PlayerPlacement {
    /// The flow's placement: its position is in centimetres (the engine's
    /// unit) and its rotation is the three rows of the spawn frame.
    pub fn of_flow(placement: &crate::flow::Placement) -> Self {
        let rows = placement.rotation;
        Self {
            position: placement.position * 0.01,
            yaw: rows[2][0].atan2(rows[0][0]),
        }
    }
}

/// Queues the flow's level change. With no level it is a plain teardown.
pub fn request(
    writer: &mut MessageWriter<LevelChangeRequest>,
    level: Option<&str>,
    spawn: u32,
    teardown: bool,
) {
    let request = LevelChangeRequest {
        level: level.map(str::to_string),
        spawn,
        teardown,
    };
    writer.write(request);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{Placement, yaw};

    #[test]
    fn the_flow_placement_becomes_metres_and_a_yaw() {
        let placement = Placement {
            rotation: yaw(150.0),
            position: Vec3::new(8100.0, 30.0, 3920.0),
        };
        let at = PlayerPlacement::of_flow(&placement);
        assert!((at.position - Vec3::new(81.0, 0.3, 39.2)).length() < 1e-5);
        assert!((at.yaw - 150.0f32.to_radians()).abs() < 1e-5);
    }
}
