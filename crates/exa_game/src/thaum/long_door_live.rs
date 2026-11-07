//! The AI's Long Door asking the navigation grid for its place.
//!
//! An AI caster's Long Door (and the Summon's place pick, which shares the
//! query) starts at phase 0x10 with no destination. Each control tick while
//! there, the original asks its pathfinder, when the shared search is free
//! or already its own:
//!
//! - the subject is the caster's current target, or the caster itself
//!   without one;
//! - the goal is the subject's place moved 300 cm back along its forward
//!   vector ([`nav_query_target`]);
//! - the path is searched from the subject's place, and its last point is
//!   the answer when the path has more than one point and that point lies
//!   more than √200 cm from the subject across the floor ([`nav_answer`]);
//!   any other path is impossible and the door ends.
//!
//! A busy search makes the door wait (the original queues the caster by
//! how long it has waited); Exarust's search answers at once, so the door
//! never waits [Exarust]. The walker's size class is the AI's usual 0.
//!
//! Evidence: the Long Door's and the Summon's phase-0x10 control and the
//! shared navigation query, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_world::nav::{NavGrid, find_path};

use super::cast::{ActiveClass, PowerCaster};
use super::power::displacement::{AI_PICK, NavAnswer, nav_answer, nav_query_target};

/// Centimetres a metre.
const CM: f32 = 100.0;

/// The navigation query's answer for a subject at `place` facing
/// `forward` (cm): the path's last point when usable.
pub fn ask_grid(grid: &NavGrid, place: Vec3, forward: Vec3) -> (NavAnswer, Option<Vec3>) {
    let goal = nav_query_target(place, forward);
    let path = find_path(grid, place / CM, goal / CM, 0);
    let found = (path.points.len() >= 2).then(|| path.points[path.points.len() - 1] * CM);
    (nav_answer(place, found), found)
}

/// Each body step, before the casters' walk: an AI's Long Door at phase
/// 0x10 with no destination asks the grid.
pub(crate) fn pick_long_doors(
    mut casters: Query<(Entity, &mut PowerCaster, Option<&exa_ai::Duellist>)>,
    places: Query<(
        &GlobalTransform,
        Option<&exa_physics::body::locomotion::Locomotor>,
    )>,
    children: Query<&Children>,
    grid: Option<Res<NavGrid>>,
) {
    let Some(grid) = grid else {
        return;
    };
    for (entity, mut caster, duellist) in &mut casters {
        let waiting = caster.active.iter().any(|a| {
            matches!(&a.class, ActiveClass::LongDoor(door)
                if door.base.phase == AI_PICK && door.destination.is_none()
                    && door.nav == NavAnswer::Waiting)
        });
        if !waiting {
            continue;
        }
        // The subject's place and its body's forward vector (the
        // locomotion's yaw: the model faces +Z; a body without one, its
        // transform's).
        let subject = duellist
            .map(|d| d.target)
            .filter(|&t| places.contains(t))
            .unwrap_or(entity);
        let Ok((at, _)) = places.get(subject) else {
            continue;
        };
        let place = at.translation() * CM;
        // A character's root carries its place; its body (a child) the
        // locomotion and the turned model.
        let body = std::iter::once(subject)
            .chain(children.iter_descendants(subject))
            .filter_map(|e| places.get(e).ok())
            .find(|(_, locomotor)| locomotor.is_some());
        let forward = match body {
            Some((_, Some(l))) => Vec3::new(l.yaw.sin(), 0.0, l.yaw.cos()),
            _ => at.rotation() * Vec3::Z,
        };
        let (answer, found) = ask_grid(&grid, place, forward);
        for entry in &mut caster.active {
            if let ActiveClass::LongDoor(door) = &mut entry.class
                && door.base.phase == AI_PICK
                && door.destination.is_none()
            {
                door.nav = answer;
                if answer == NavAnswer::Found {
                    door.destination = found.map(|p| p.to_array());
                }
            }
        }
        info!(
            "long door {entity:?}: the grid answers {answer:?} for subject {subject:?} at {place:?} cm facing {forward:?} (found {found:?})"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::nav::{Layer, NavCell};

    #[test]
    fn an_open_floor_answers_a_place_behind_the_subject() {
        let mut grid = NavGrid::empty(80, 80, 0.0);
        for z in 0..80 {
            for x in 0..80 {
                let floor = NavCell {
                    floor: 100,
                    ..NavCell::default()
                };
                grid.set_cell(x, z, Layer::Lower, floor);
            }
        }
        // A floor at 10 m (shipped floors lie far above voxel 15).
        let place = Vec3::new(15.0, 1000.0, 15.0);
        let (answer, found) = ask_grid(&grid, place, Vec3::Z);
        assert_eq!(answer, NavAnswer::Found);
        let found = found.unwrap();
        // Behind the subject, more than √200 cm away.
        assert!(found.z < place.z - 14.0, "{found:?}");
    }

    #[test]
    fn no_grid_floor_is_impossible() {
        let grid = NavGrid::empty(4, 4, 0.0);
        let (answer, _) = ask_grid(&grid, Vec3::new(15.0, 0.0, 15.0), Vec3::Z);
        assert_eq!(answer, NavAnswer::Impossible);
    }
}
