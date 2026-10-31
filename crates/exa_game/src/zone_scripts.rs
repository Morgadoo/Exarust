//! The level's script zones, run live: a character stepping into one runs
//! the zone's routine.
//!
//! - **Zone check.** Each frame, every character (a body with a record id)
//!   whose point (its frame anchor: the root's x and z at the pelvis's
//!   height, [`zone_point`]) is more than 3 cm from the point its zones were
//!   last checked at
//!   ([`exa_ai::golem::moved_enough`]) checks the level's script zones in
//!   placement order and its point moves to it. A zone it has just entered
//!   ([`exa_ai::golem::zone_entered`]: within the node radius, inside the
//!   box, outside it at the last check) runs its routine.
//! - **Routine.** The routine runs through the conversation engine with the
//!   character as its one actor slot and the zone's id as the local
//!   `ZoneID` ([`exa_script::dialogue::Engine::run_zone_script`]); what it
//!   asks for goes out as [`exa_script::ScriptEffect`] messages, read by
//!   their consumers (`CheckpointSave` by [`crate::save_live`], `GameEvent`
//!   by [`crate::game_event_live`]).
//!
//! The shipped story levels place eleven such zones: `exanima01`'s
//! `Checkpoint` zone runs `CheckpointSave(1);` (the story's first
//! checkpoint, on the way to the exit), and the others activate narrator
//! topics (`Magic Lamps`, `Deeper`, `Portals`, `Sewer Smell`, `Scary Lair`).
//!
//! Evidence: the AI manager's frame (the 3 cm step and the checked point)
//! and its zone check (the radius, the box distance against 0.1 now and at
//! the last check, the volume section's class `0x100`, low byte `0x10`
//! running the routine for any character), and the routine runner (one
//! actor, the `ZoneID` local, the routine's text after the section's fixed
//! words).
//!
//! Not reproduced: the zone check's other kinds stay on their own paths
//! (the alarm zones in [`crate::golem_alarm`], the level transitions), so
//! the original's one loop over every trigger is two Exarust loops over the
//! same placement order with the same checked-point rule; an alarm zone's
//! held-player return that ends the original's loop does not stop a later
//! script zone (no shipped level has both kinds of enter zone). The stay
//! zones (class `0x200`, the memory fragments' `GameEvent` routines) are not
//! run here. The checked point starts at the origin, as the alarm check's
//! does [guess: the body is allocated zeroed].

use bevy::prelude::*;
use exa_ai::brain::CharacterId;
use exa_ai::golem;
use exa_world::ScriptVolume;

/// The level's script zones, in placement order. The level start inserts
/// them from the loaded level ([`exa_world::LevelInfo::script_volumes`]).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptVolumes(pub Vec<ScriptVolume>);

/// The point (cm) a character's script zones were last checked at.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ZoneCheckPoint(pub Vec3);

pub struct ZoneScriptsPlugin;

impl Plugin for ZoneScriptsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptVolumes>()
            .add_systems(Update, check_script_zones);
    }
}

/// A character's point for the zone check, in centimetres: the body
/// frame's anchor, whose x and z follow the root and whose height is the
/// pelvis's world height (the root's own height for a body without a
/// pelvis part). The shipped checkpoint zone floats 10 cm off the floor,
/// so the feet would miss it.
pub fn zone_point(root: Vec3, pelvis_height: Option<f32>) -> Vec3 {
    Vec3::new(root.x, pelvis_height.unwrap_or(root.y), root.z) * 100.0
}

/// The script zones a character at `at` (cm), last checked at `checked`,
/// has just entered, in placement order.
pub fn entered_zones(volumes: &[ScriptVolume], at: Vec3, checked: Vec3) -> Vec<&ScriptVolume> {
    volumes
        .iter()
        .filter(|volume| golem::zone_entered(at, checked, &volume.bounds, volume.radius))
        .collect()
}

/// What the zone check reads of a character, and the point it keeps.
type ZoneCharacter<'a> = (
    Entity,
    &'a GlobalTransform,
    &'a CharacterId,
    Option<&'a exa_physics::body::Ragdoll>,
    Option<&'a mut ZoneCheckPoint>,
);

/// The AI manager's zone check for the script zones, for every character.
pub fn check_script_zones(
    mut commands: Commands,
    volumes: Res<ScriptVolumes>,
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    time: Option<Res<Time<Virtual>>>,
    mut characters: Query<ZoneCharacter>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    if volumes.0.is_empty() {
        return;
    }
    let mut work = Vec::new();
    for (entity, gt, id, body, point) in &mut characters {
        let pelvis = body
            .and_then(|b| b.part_of(1))
            .map(|p| p.transform().translation.y);
        let at = zone_point(gt.translation(), pelvis);
        let checked = point.as_deref().map_or(Vec3::ZERO, |p| p.0);
        if !golem::moved_enough(at, checked) {
            continue;
        }
        match point {
            Some(mut point) => point.0 = at,
            None => {
                commands.entity(entity).insert(ZoneCheckPoint(at));
            }
        }
        for volume in entered_zones(&volumes.0, at, checked) {
            info!(
                "zone script: character {:#x} entered zone {} at {:?} cm: {}",
                id.0,
                volume.zone_id,
                volume.bounds.centre,
                String::from_utf8_lossy(&volume.script)
            );
            work.push((id.0, volume.zone_id, volume.script.clone()));
        }
    }
    if work.is_empty() {
        return;
    }
    let (Some(scripts), Some(mut conversations)) = (scripts, conversations) else {
        return;
    };
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    let ((), out) = conversations.with_scene(&scripts, now, |engine, scene| {
        for (actor, zone, text) in work {
            engine.run_zone_script(scene, actor, zone, &text);
        }
    });
    for effect in out {
        effects.write(exa_script::ScriptEffect(effect));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_physics::intersect::Obb;

    fn zone(centre: Vec3, script: &[u8]) -> ScriptVolume {
        let half = Vec3::new(80.0, 123.0, 50.0);
        ScriptVolume {
            bounds: Obb::new(centre, half, [Vec3::X, Vec3::Y, Vec3::Z]),
            radius: half.length(),
            zone_id: 0,
            script: script.to_vec(),
        }
    }

    /// A zone is entered once: from outside to inside its box, and not
    /// again while the character stays in it.
    #[test]
    fn a_zone_runs_when_entered_not_while_inside() {
        let zones = [zone(Vec3::new(1000.0, 100.0, 0.0), b"CheckpointSave(1);")];
        let outside = Vec3::new(1000.0, 100.0, 200.0);
        let inside = Vec3::new(1000.0, 100.0, 10.0);
        assert!(entered_zones(&zones, outside, Vec3::ZERO).is_empty());
        assert_eq!(entered_zones(&zones, inside, outside).len(), 1);
        assert!(entered_zones(&zones, inside + Vec3::X * 5.0, inside).is_empty());
        // The first check from the zeroed point counts as coming in.
        assert_eq!(entered_zones(&zones, inside, Vec3::ZERO).len(), 1);
    }

    /// The point is the root's x and z at the pelvis height, in cm.
    #[test]
    fn the_zone_point_is_the_frame_anchor() {
        let at = zone_point(Vec3::new(62.85, 0.05, 49.05), Some(1.05));
        assert!((at - Vec3::new(6285.0, 105.0, 4905.0)).length() < 1e-2);
        assert_eq!(
            zone_point(Vec3::new(1.0, 0.5, 2.0), None),
            Vec3::new(100.0, 50.0, 200.0)
        );
    }

    /// Two zones entered at once run in placement order.
    #[test]
    fn zones_run_in_placement_order() {
        let a = zone(Vec3::ZERO + Vec3::X * 500.0, b"A");
        let b = zone(Vec3::ZERO + Vec3::X * 520.0, b"B");
        let zones = [a, b];
        let got: Vec<&[u8]> = entered_zones(&zones, Vec3::X * 510.0, Vec3::X * 5000.0)
            .iter()
            .map(|z| z.script.as_slice())
            .collect();
        assert_eq!(got, [&b"A"[..], b"B"]);
    }
}
