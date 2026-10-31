//! The level's trigger zones, run live: a character stepping into a script
//! zone runs the zone's routine, the player stepping into an exit zone
//! changes level, and the player stopping inside a memory fragment's stay
//! zone runs its routine.
//!
//! - **Zone check.** Each frame, every character in the scene (a body with
//!   a record id whose presence is not dormant, leaving or entering) whose
//!   point (its frame anchor: the root's x and z at the pelvis's height,
//!   [`zone_point`]) is more than 3 cm from the point its zones were last
//!   checked at ([`exa_ai::golem::moved_enough`]) checks the level's zones
//!   in placement order and its point moves to it. A zone it has just
//!   entered ([`exa_ai::golem::zone_entered`]: within the node radius,
//!   inside the box, outside it at the last check) acts by its kind
//!   ([`exa_ai::manager::enter`]): a script zone runs its routine for any
//!   character; an exit zone, for the player, queues the level change to
//!   its level (a `locales.rdb` id) at its entrance, the long transition
//!   for the flagged kind ([`ExitZoneCrossed`], read by the game flow).
//! - **Checked point.** It starts at the origin: the original's character
//!   object is allocated zero-filled and only the AI manager's frame writes
//!   the point. A character that is in a zone at its first check (in the
//!   scene, more than 3 cm from the origin) has entered it, as in the
//!   original: `exanima01`'s first `Magic Lamps` zone holds Sticky (0x26)
//!   at the level start, whose routine runs and does nothing for it (the
//!   topic is the player role's).
//! - **Stay check.** A clip start that leaves Walk or Run leaves a
//!   trailing-foot request; 500 ms on, the character tick clears it and
//!   runs the stay check (`exa_physics::body::locomotion::Locomotor::
//!   stay_check_due`). For the player it runs the routine of every stay
//!   zone ([`exa_world::stay_volumes`]) the player's point is inside
//!   ([`exa_ai::golem::zone_inside`]), whatever the last check: the memory
//!   fragments' `GameEvent`, consumed by [`crate::game_event_live`].
//! - **Routine.** The routine runs through the conversation engine with the
//!   character as its one actor slot and the zone's id as the local
//!   `ZoneID` ([`exa_script::dialogue::Engine::run_zone_script`]); what it
//!   asks for goes out as [`exa_script::ScriptEffect`] messages, read by
//!   their consumers (`CheckpointSave` by [`crate::save_live`], `GameEvent`
//!   by [`crate::game_event_live`]).
//!
//! The shipped story levels place eleven script zones (`exanima01`'s
//! `Checkpoint` zone runs `CheckpointSave(1);`; the others activate
//! narrator topics), eight memory fragment stay zones (two in
//! `exanima01`, `GameEvent(C00001)` and `C00002`) and the `>name` exits
//! between the levels.
//!
//! Evidence: the AI manager's frame (the 3 cm step, the in-scene state, the
//! checked point it alone writes), the character object's zero-filled
//! allocation, the enter-zone check (the radius, the box distance against
//! 0.1 now and at the last check, the volume section's class and kind), the
//! deferred level change it queues (level, entrance, the `0x21` flag), the
//! stay check and its one caller in the character tick, and the routine
//! runner (one actor, the `ZoneID` local, the routine's text after the
//! section's fixed words).
//!
//! Not reproduced: the alarm zones keep their own loop in
//! [`crate::golem_alarm`] (with the same checked-point rule), so an alarm
//! zone's exempt-player return that ends the original's loop does not stop
//! a later script or exit zone (no shipped level has both an alarm zone and
//! an enter script zone); within a pass the script zones run before the
//! exits, which the original interleaves by placement (the exit is
//! deferred, so only an ended pass could tell). The exit's release of an
//! active possession is not reached (Exarust has no live possession). The
//! stay check runs from the request's lapse whatever the body does, where
//! the original reads it in the movement stage's stopped branch (the move
//! branch drops the request in both).

use bevy::prelude::*;
use exa_ai::brain::{CharacterId, RatedPlayer};
use exa_ai::golem;
use exa_ai::presence::{Presence, PresenceState};
use exa_physics::body::locomotion::Locomotor;
use exa_world::{ExitVolume, ScriptVolume};

/// The level's script zones, in placement order. The level start inserts
/// them from the loaded level ([`exa_world::LevelInfo::script_volumes`]).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptVolumes(pub Vec<ScriptVolume>);

/// The level's stay zones, in placement order
/// ([`exa_world::LevelInfo::stay_volumes`]).
#[derive(Resource, Debug, Clone, Default)]
pub struct StayVolumes(pub Vec<ScriptVolume>);

/// The level's exit zones, in placement order
/// ([`exa_world::LevelInfo::exit_volumes`]).
#[derive(Resource, Debug, Clone, Default)]
pub struct ExitVolumes(pub Vec<ExitVolume>);

/// The point (cm) a character's zones were last checked at.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ZoneCheckPoint(pub Vec3);

/// The player entered an exit zone: the queued level change.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ExitZoneCrossed {
    /// The destination's `locales.rdb` id.
    pub level: u32,
    /// The arrival marker's entrance.
    pub entrance: u32,
    /// The long transition (the zone's kind `0x21`).
    pub flagged: bool,
    /// The destination's name, when the table names it.
    pub destination: Option<String>,
    /// The player's condition word.
    pub condition: u32,
}

pub struct ZoneScriptsPlugin;

impl Plugin for ZoneScriptsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptVolumes>()
            .init_resource::<StayVolumes>()
            .init_resource::<ExitVolumes>()
            .add_message::<ExitZoneCrossed>()
            .add_systems(Update, (check_zones, check_stay_zones));
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

/// The zone point of a body at `root` (m) with this ragdoll.
pub fn body_zone_point(root: Vec3, body: Option<&exa_physics::body::Ragdoll>) -> Vec3 {
    let pelvis = body
        .and_then(|b| b.part_of(1))
        .map(|p| p.transform().translation.y);
    zone_point(root, pelvis)
}

/// The script zones a character at `at` (cm), last checked at `checked`,
/// has just entered, in placement order.
pub fn entered_zones(volumes: &[ScriptVolume], at: Vec3, checked: Vec3) -> Vec<&ScriptVolume> {
    volumes
        .iter()
        .filter(|volume| golem::zone_entered(at, checked, &volume.bounds, volume.radius))
        .collect()
}

/// The exit zones the player at `at` (cm), last checked at `checked`, has
/// just entered, in placement order.
pub fn entered_exits(volumes: &[ExitVolume], at: Vec3, checked: Vec3) -> Vec<&ExitVolume> {
    volumes
        .iter()
        .filter(|volume| golem::zone_entered(at, checked, &volume.bounds, volume.radius))
        .collect()
}

/// The stay zones the player's stay check runs, the player at `at` (cm):
/// every one it is inside, in placement order.
pub fn stayed_zones(volumes: &[ScriptVolume], at: Vec3) -> Vec<&ScriptVolume> {
    volumes
        .iter()
        .filter(|volume| golem::zone_inside(at, &volume.bounds, volume.radius))
        .collect()
}

/// Whether the AI manager's frame checks this character's zones: one in
/// the scene (presence state 1); a character without a presence (the
/// player) always is.
pub fn in_scene(presence: Option<&Presence>) -> bool {
    presence.is_none_or(|p| p.state == PresenceState::Present)
}

/// What the zone check reads of a character, and the point it keeps.
type ZoneCharacter<'a> = (
    Entity,
    &'a GlobalTransform,
    &'a CharacterId,
    Option<&'a exa_physics::body::Ragdoll>,
    Option<&'a Presence>,
    Has<RatedPlayer>,
    Option<&'a exa_combat::Condition>,
    Option<&'a mut ZoneCheckPoint>,
);

/// Runs zone routines (actor, zone id, text) and sends what they ask for.
fn run_routines(
    work: Vec<(u32, u32, Vec<u8>)>,
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    time: Option<Res<Time<Virtual>>>,
    effects: &mut MessageWriter<exa_script::ScriptEffect>,
) {
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

/// The AI manager's zone check for the script and exit zones, for every
/// character in the scene.
#[allow(clippy::too_many_arguments)]
pub fn check_zones(
    mut commands: Commands,
    volumes: Res<ScriptVolumes>,
    exits: Res<ExitVolumes>,
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    time: Option<Res<Time<Virtual>>>,
    mut characters: Query<ZoneCharacter>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
    mut crossed: MessageWriter<ExitZoneCrossed>,
) {
    if volumes.0.is_empty() && exits.0.is_empty() {
        return;
    }
    let mut work = Vec::new();
    for (entity, gt, id, body, presence, player, condition, point) in &mut characters {
        if !in_scene(presence) {
            continue;
        }
        let at = body_zone_point(gt.translation(), body);
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
        if !player {
            continue;
        }
        for volume in entered_exits(&exits.0, at, checked) {
            info!(
                "exit zone: the player at {at:?} cm entered {} at {:?} cm: level {} ({:?}) entrance {}{}",
                volume.name,
                volume.bounds.centre,
                volume.exit.level,
                volume.destination,
                volume.exit.entrance,
                if volume.exit.flagged {
                    ", long transition"
                } else {
                    ""
                }
            );
            crossed.write(ExitZoneCrossed {
                level: volume.exit.level,
                entrance: volume.exit.entrance,
                flagged: volume.exit.flagged,
                destination: volume.destination.clone(),
                condition: condition.map_or(0, |c| c.0),
            });
        }
    }
    run_routines(work, scripts, conversations, time, &mut effects);
}

/// What the stay check reads of a character.
type StayCharacter<'a> = (
    &'a GlobalTransform,
    &'a CharacterId,
    Option<&'a exa_physics::body::Ragdoll>,
    Has<RatedPlayer>,
    &'a mut Locomotor,
);

/// The stay check the character tick runs once a trailing-foot request has
/// lapsed: for the player, every stay zone it is inside runs its routine.
pub fn check_stay_zones(
    volumes: Res<StayVolumes>,
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    time: Option<Res<Time<Virtual>>>,
    mut characters: Query<StayCharacter>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let mut work = Vec::new();
    for (gt, id, body, player, mut locomotor) in &mut characters {
        if !locomotor.stay_check_due {
            continue;
        }
        locomotor.stay_check_due = false;
        if !player {
            continue;
        }
        let at = body_zone_point(gt.translation(), body);
        let stayed = stayed_zones(&volumes.0, at);
        debug!(
            "stay check: the player at {at:?} cm is in {} stay zone(s)",
            stayed.len()
        );
        for volume in stayed {
            info!(
                "stay zone: the player stopped at {at:?} cm in zone {} at {:?} cm: {}",
                volume.zone_id,
                volume.bounds.centre,
                String::from_utf8_lossy(&volume.script)
            );
            work.push((id.0, volume.zone_id, volume.script.clone()));
        }
    }
    run_routines(work, scripts, conversations, time, &mut effects);
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

    /// A stay zone runs for a player inside it whether or not it was
    /// inside at the last check; one outside it does not.
    #[test]
    fn a_stay_zone_runs_while_inside() {
        let zones = [zone(
            Vec3::new(-3220.0, 100.0, 6120.0),
            b"GameEvent(C00001);",
        )];
        assert_eq!(
            stayed_zones(&zones, Vec3::new(-3220.0, 100.0, 6100.0)).len(),
            1
        );
        assert!(stayed_zones(&zones, Vec3::new(-3220.0, 100.0, 6400.0)).is_empty());
    }

    /// An exit zone is entered as a script zone is: from outside, once.
    #[test]
    fn an_exit_zone_is_entered_once() {
        let half = Vec3::new(225.0, 150.0, 225.0);
        let exits = [ExitVolume {
            bounds: Obb::new(
                Vec3::new(-900.0, 100.0, -9980.0),
                half,
                [Vec3::X, Vec3::Y, Vec3::Z],
            ),
            radius: half.length(),
            exit: exa_formats::objdb::LevelExit {
                level: 3,
                entrance: 0,
                flagged: false,
            },
            destination: Some("exanima02".into()),
            name: ">exanima02".into(),
        }];
        let inside = Vec3::new(-900.0, 100.0, -9900.0);
        let outside = Vec3::new(-900.0, 100.0, -9500.0);
        assert_eq!(entered_exits(&exits, inside, outside).len(), 1);
        assert!(entered_exits(&exits, inside, inside + Vec3::X * 4.0).is_empty());
        assert!(entered_exits(&exits, outside, Vec3::ZERO).is_empty());
    }

    /// Only a character in the scene checks its zones; the player has no
    /// presence and always does.
    #[test]
    fn only_characters_in_the_scene_check() {
        assert!(in_scene(None));
        let mut presence = Presence::default();
        assert!(in_scene(Some(&presence)));
        for state in [
            PresenceState::Dormant,
            PresenceState::Leaving,
            PresenceState::Entering,
        ] {
            presence.state = state;
            assert!(!in_scene(Some(&presence)));
        }
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
