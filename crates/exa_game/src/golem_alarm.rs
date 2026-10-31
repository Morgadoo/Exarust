//! The alarm zones of the golem guardians, run live: the player stepping
//! into an alarm zone without its pass wakes the statue golems around it.
//!
//! - **Zone check.** Each frame, once the player's point (its body frame's
//!   anchor: the root's x and z at the pelvis height,
//!   [`crate::zone_scripts::zone_point`]) is more than 3 cm from the
//!   point its zones were last checked at ([`exa_ai::golem::moved_enough`]),
//!   the level's alarm zones are checked in placement order and the point
//!   moves to the player. A zone the player has just entered
//!   ([`exa_ai::golem::zone_entered`]) ends the check while the player's
//!   state flags carry [`exa_ai::golem::ALARM_EXEMPT`], is crossed freely
//!   when the player carries its pass ([`exa_ai::golem::carries`], over the
//!   inventory and the containers in it), and rings the alarm otherwise.
//! - **Alarm.** The zone arms the alarm (its box centre and pass) unless
//!   another armed zone is kept ([`exa_ai::golem::alarm_trigger`]: kept
//!   unless the player carries that zone's pass); then every character in
//!   the room holding the zone's centre or in a room behind that room's
//!   exits whose race word carries the AI bit and which stands strictly
//!   within 800 cm (x and z) of the centre is offered to the wake callback.
//!   A golem whose statue byte is set ([`exa_ai::brain::Npc::statue`]) is
//!   woken ([`exa_ai::golem::wake`]) unless its body rules carry
//!   [`exa_ai::golem::WAKE_EXEMPT`]: its record's race word loses the
//!   statue bit, `golem_act` plays at it (volume ½, pitch 0.9 plus a fifth
//!   of the engine's draw), the statue byte clears so its ally sense sees,
//!   its body is kept awake for 256 ticks and its place and facing become
//!   its home. Exempt or not, its contacts (the combatant's list and the
//!   brain's integration copy) become the player alone at a concern of 10. A ring that woke any counts one more alarm.
//! - **Reset.** Each level start disarms the alarm and zeroes the count
//!   ([`GolemAlarm::default`]).
//! - **Return.** The story revive of a knocked-out player
//!   ([`crate::flow::Step::AlarmReturn`], before its save) runs the alarm
//!   return while an alarm is armed ([`return_from_alarm`],
//!   [`exa_ai::golem::alarm_return`]): every character in the player's
//!   room or a room behind its exits whose race word carries the AI bit
//!   and which stands strictly within 3000 cm (x and z) of the player is
//!   offered to the homeward callback. An awake guardian (statue byte
//!   clear) that is not dying goes home ([`exa_ai::golem::go_home`]): its
//!   tone and stance factor become 1, its condition 0, it plays Stand 2,
//!   it is put at its home facing its home angle and woken for 32 ticks,
//!   and it falls asleep ([`exa_ai::golem::sleep`]: the race word's statue
//!   bit and the statue byte set, the statue rest on, the condition 0 and,
//!   the first time, the sleep latch and scales). With the statue byte set
//!   its ally sense clears its concern for the player, so its target pass
//!   leaves combat. When one went home, the player is put 200 cm past the
//!   third-axis face of the first `Alarm_` box holding the armed zone's
//!   point, facing back across it with a draw, and asks for motion 0x1d;
//!   its condition is dead and its tone and stance factor 0, which the
//!   revive that follows raises. The alarm stays armed.
//!
//! Evidence: the original's AI manager frame (the 3 cm step and the checked
//! point), its zone check, the inventory search, the alarm trigger, its
//! radius query and wake callback, the golem wake and the alarm reset. See
//! `exa_ai::golem` for the rules.
//!
//! The wake's clear of the pose word and statue rest follows from the
//! cleared statue byte at the next sentry tick ([`crate::golem_body`]); the
//! sleep scales the sentry put on a statue come off with its latch
//! ([`crate::golem_body::undo_sleep_scales`]).
//!
//! Evidence for the return: the story knock-out hook (the dead condition
//! with red health of at least 0.01, five seconds), the story revive that
//! runs the world's revive pass before its save, that pass's first call
//! (the alarm return, only while the alarm's armed byte is set), its 3000
//! cm radius query around the player, the homeward callback, the guardian
//! sleep, the `Alarm_` lookup and the pose reset.
//!
//! Not reproduced: the stance byte the wake clears (Exarust's statues never
//! set it), the glow fade of the skin material `go` (to 1 on a wake, to 0
//! on a sleep: no live skin-material fade, it is logged), the rest of the
//! world's revive pass (every brain's reset and every body's wake), the
//! pose reset's transition bytes, angular words and the parts' reached
//! bit, the body links' re-measure after the move (Exarust's links keep
//! their rest lengths) and the pose reset's frame height: the original
//! puts the frame at the body's own standing-height word, Exarust keeps
//! the character root's height [guess: the floor under it], the zone
//! check's other trigger kinds (the any-character routine
//! and the level exits, run by [`crate::zone_scripts`]), and the
//! contents of a carried container that is a level-local item (the item
//! catalogue holds only the game database's containers). The checked point
//! starts at the origin (the character object is allocated zero-filled and
//! only the AI manager's frame writes it); the room
//! lists come from each character's current position ([guess: without a
//! level room grid every character is offered]).

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_ai::brain::{Npc, RatedPlayer};
use exa_ai::combatant::{Combatant, Contact};
use exa_ai::golem::{self, AlarmNode, AlarmZone, GolemState, QueryBody};
use exa_anim::{Animator, Motions};
use exa_character::record::CharacterData;
use exa_combat::{Condition, Fighter, StateFlags};
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::Possessions;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotion;
use exa_world::AlarmVolume;
use exa_world::room_view::LevelRooms;

/// The level's alarm zones, in placement order. The level start inserts
/// them from the loaded level ([`exa_world::LevelInfo::alarm_volumes`]).
#[derive(Resource, Debug, Clone, Default)]
pub struct AlarmVolumes(pub Vec<AlarmVolume>);

/// The armed alarm and how many rings woke a golem since the level start.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GolemAlarm {
    pub armed: AlarmZone,
    pub rung: u32,
}

impl Default for GolemAlarm {
    /// The alarm reset a level start runs.
    fn default() -> Self {
        Self {
            armed: AlarmZone::reset(),
            rung: 0,
        }
    }
}

/// The point (cm) the player's zones were last checked at.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct AlarmCheckPoint(pub Vec3);

pub struct GolemAlarmPlugin;

impl Plugin for GolemAlarmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AlarmVolumes>()
            .init_resource::<GolemAlarm>()
            .add_systems(
                Update,
                (
                    check_alarm_zones.after(crate::rise::restore_revived),
                    return_from_alarm
                        .after(crate::flow::FlowSystems::Transition)
                        .before(crate::flow::FlowSystems::ApplySteps),
                ),
            );
    }
}

/// The alarm zones the player has just entered without their pass, in
/// order, from the player at `at` last checked at `checked`: the zone
/// check's loop, which `exempt` (the state-flag bit) ends at the first
/// alarm zone entered.
pub fn rung_zones<'a>(
    volumes: &'a [AlarmVolume],
    at: Vec3,
    checked: Vec3,
    exempt: bool,
    carries: &dyn Fn(u32) -> bool,
) -> Vec<&'a AlarmVolume> {
    let mut rung = Vec::new();
    for volume in volumes {
        if !golem::zone_entered(at, checked, &volume.bounds, volume.radius) {
            continue;
        }
        if exempt {
            break;
        }
        if carries(volume.pass) {
            continue;
        }
        rung.push(volume);
    }
    rung
}

/// The rooms the alarm's radius query walks: the room holding `point`
/// and the rooms behind its exits, in link order (a room behind two exits
/// is walked twice). Empty where the point is in no room.
pub fn query_rooms(level: &LevelRooms, point: Vec3) -> Vec<i32> {
    let room = exa_ai::rooms::level_room(&level.rooms, point);
    if room < 0 {
        return Vec::new();
    }
    let mut rooms = vec![room];
    rooms.extend(
        exa_ai::rooms::room_exits(&level.rooms, &level.tracker, room as u16)
            .iter()
            .map(|exit| exit.room - 1),
    );
    rooms
}

type Guardians<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static mut CharacterData,
        Option<&'static mut Npc>,
        Option<&'static mut Combatant>,
        Option<&'static Fighter>,
        Option<&'static mut StateFlags>,
        Option<&'static mut Ragdoll>,
        Option<&'static mut crate::golem_body::StatueRest>,
    ),
    Without<RatedPlayer>,
>;

/// The AI manager's zone check for the player, and the alarm it rings.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn check_alarm_zones(
    mut commands: Commands,
    volumes: Res<AlarmVolumes>,
    mut alarm: ResMut<GolemAlarm>,
    level: Option<Res<LevelRooms>>,
    catalogue: Option<Res<ItemCatalogue>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    mut players: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&StateFlags>,
            Option<&Possessions>,
            Option<&Ragdoll>,
            Option<&mut AlarmCheckPoint>,
        ),
        With<RatedPlayer>,
    >,
    mut guardians: Guardians,
) {
    if volumes.0.is_empty() {
        return;
    }
    let Some((player, gt, flags, possessions, body, point)) = players.iter_mut().next() else {
        return;
    };
    // The body frame's anchor: the root's x and z at the pelvis height.
    let at = crate::zone_scripts::body_zone_point(gt.translation(), body);
    let checked = point.as_deref().map_or(Vec3::ZERO, |p| p.0);
    if !golem::moved_enough(at, checked) {
        return;
    }
    match point {
        Some(mut point) => point.0 = at,
        None => {
            commands.entity(player).insert(AlarmCheckPoint(at));
        }
    }
    let contents = |id: u32| catalogue.as_deref().and_then(|c| c.contents(id));
    let carried: Vec<u32> = possessions.map_or_else(Vec::new, |p| {
        p.carried
            .iter()
            .map(|c| c.id)
            .filter(|&id| id != 0)
            .collect()
    });
    let carries = |item: u32| golem::carries(carried.iter().copied(), item, &contents);
    let exempt = flags.is_some_and(|f| f.has(golem::ALARM_EXEMPT));
    // A zone entered with its pass is crossed freely: logged, as the
    // proof a carried pass let the player through.
    if !exempt {
        for volume in volumes
            .0
            .iter()
            .filter(|v| golem::zone_entered(at, checked, &v.bounds, v.radius) && carries(v.pass))
        {
            let centre = volume.bounds.centre;
            info!(
                "alarm zone at ({:.0}, {:.0}, {:.0}) cm: the player at ({:.0}, {:.0}, {:.0}) cm carries pass {} and crosses it",
                centre.x, centre.y, centre.z, at.x, at.y, at.z, volume.pass
            );
        }
    }
    for volume in rung_zones(&volumes.0, at, checked, exempt, &carries) {
        let centre = volume.bounds.centre;
        let armed = golem::alarm_trigger(&mut alarm.armed, centre, volume.pass, carries);
        let offered = offered_characters(level.as_deref(), centre, &guardians);
        let woken = golem::radius_query(centre, golem::WAKE_RADIUS, offered, |body| {
            let entity = Entity::from_bits(body.character);
            wake_one(
                &mut guardians,
                entity,
                player,
                bank.as_deref_mut(),
                &mut sounds,
            )
        });
        if woken > 0 {
            alarm.rung += 1;
        }
        info!(
            "alarm zone at ({:.0}, {:.0}, {:.0}) cm, pass {}: {} the alarm, {woken} golem(s) woken, {} ring(s)",
            centre.x,
            centre.y,
            centre.z,
            volume.pass,
            if armed { "armed" } else { "kept" },
            alarm.rung
        );
    }
}

/// The characters the alarm's radius query offers, in its order: those
/// standing in each of [`query_rooms`] in turn (every character when the
/// level has no room grid), with their race words.
fn offered_characters(
    level: Option<&LevelRooms>,
    centre: Vec3,
    guardians: &Guardians,
) -> Vec<QueryBody> {
    let bodies: Vec<QueryBody> = guardians
        .iter()
        .map(|(entity, gt, data, ..)| QueryBody {
            at: gt.translation() * 100.0,
            race_word: data.record.profession(),
            character: entity.to_bits(),
        })
        .collect();
    offered_in_rooms(level, centre, bodies)
}

/// The bodies a radius query around `centre` visits, in its order: those
/// standing in each of [`query_rooms`] in turn (all of them when the level
/// has no room grid).
pub fn offered_in_rooms(
    level: Option<&LevelRooms>,
    centre: Vec3,
    bodies: Vec<QueryBody>,
) -> Vec<QueryBody> {
    let Some(level) = level else {
        return bodies;
    };
    let mut offered = Vec::new();
    for room in query_rooms(level, centre) {
        offered.extend(
            bodies
                .iter()
                .filter(|body| exa_ai::rooms::level_room(&level.rooms, body.at) == room),
        );
    }
    offered
}

/// The wake callback on one offered character: a golem whose statue byte
/// is set wakes (unless exempt) and keeps the player as its only contact.
/// Returns whether the callback took it.
fn wake_one(
    guardians: &mut Guardians,
    entity: Entity,
    player: Entity,
    bank: Option<&mut exa_audio::SoundBank>,
    sounds: &mut MessageWriter<exa_audio::PointSounded>,
) -> bool {
    let Ok((_, gt, mut data, npc, combatant, fighter, flags, body, rest)) =
        guardians.get_mut(entity)
    else {
        return false;
    };
    let Some(mut npc) = npc else {
        return false;
    };
    if golem::wake_guardian(npc.statue, None).is_none() {
        return false;
    }
    let rules = fighter.map_or(0, |f| f.stats.rules.0);
    let at = gt.translation() * 100.0;
    let latch = flags
        .as_deref()
        .is_some_and(|f| f.has(golem::SLEEP_SCALES_LATCH));
    let state = GolemState {
        race_word: data.record.profession(),
        dormant: npc.statue,
        latch,
        at,
        angle: exa_physics::body::locomotion::heading_of(gt.rotation() * Vec3::Z),
        masses: Vec::new(),
        limb_scales: Vec::new(),
        glow_scales: Vec::new(),
    };
    // The pitch is drawn inside the wake, after its exemption test.
    let roll = if rules & golem::WAKE_EXEMPT == 0 {
        bank.map_or(0.5, |bank| bank.rng().unit_f64())
    } else {
        0.0
    };
    if let Some((change, pitch)) = golem::wake(&state, rules, roll) {
        let race_word = change.race_word;
        data.record.set_profession(race_word);
        npc.statue = change.dormant;
        npc.guard_home = change.home;
        // The wake zeroes the body's pose word and statue byte: its rest
        // ends now, whatever state the brain is in next.
        if let Some(mut rest) = rest {
            rest.0 = false;
        }
        let unlatched = latch && !change.latch;
        if let Some(mut body) = body {
            if let Some(ticks) = change.wake_ticks {
                body.wake_to(ticks as i32);
            }
            // The sleep scales the sentry put on the statue come off: every
            // part takes its contact base back, each held fist fifty times
            // what it had.
            if unlatched {
                crate::golem_body::undo_sleep_scales(&mut body);
            }
        }
        if let Some(mut flags) = flags
            && unlatched
        {
            flags.0 &= !golem::SLEEP_SCALES_LATCH;
        }
        sounds.write(exa_audio::PointSounded {
            name: golem::WAKE_SOUND.to_string(),
            position: at / 100.0,
            volume: golem::WAKE_VOLUME,
            pitch,
            flags: 0,
        });
        info!(
            "{entity:?} wakes: race word {race_word:#010x}, home ({:.0}, {:.0}, {:.0}) cm, golem_act at pitch {pitch:.3}, glow 'go' to {} (not faded live)",
            at.x, at.y, at.z, change.glow
        );
    }
    // The original keeps one contact list; Exarust keeps the combatant's
    // and the brain's integration copy, so both become the player alone.
    if let Some(wake) = golem::wake_guardian(true, Some(player.to_bits())).flatten() {
        let mut contact = Contact::new(player);
        contact.concern = wake.concern;
        npc.contacts = vec![exa_ai::contacts::ContactRecord::of(&contact, wake.target)];
        if let Some(mut combatant) = combatant {
            combatant.contacts = vec![contact];
        }
    }
    true
}

/// The player the alarm return resets.
type ReturnPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static mut Transform,
        Option<&'static ChildOf>,
        Option<&'static mut Ragdoll>,
        Option<&'static mut Condition>,
        Option<&'static mut Locomotion>,
        Option<&'static mut crate::player::PlayerBody>,
    ),
    (With<RatedPlayer>, Without<Npc>),
>;

/// The guardians the alarm return may send home.
type ReturnGuardians<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static mut Transform,
        Option<&'static ChildOf>,
        &'static mut CharacterData,
        &'static mut Npc,
        Option<&'static mut Condition>,
        Option<&'static mut Ragdoll>,
        Option<&'static mut StateFlags>,
        Option<&'static mut crate::golem_body::StatueRest>,
        Option<&'static mut Animator>,
        Option<&'static mut Locomotion>,
    ),
    Without<RatedPlayer>,
>;

/// The character roots (the controllers' entities) the resets move.
type ReturnRoots<'w, 's> =
    Query<'w, 's, &'static mut Transform, (Without<RatedPlayer>, Without<Npc>)>;

/// Where a reset puts a body: its character root at `at` (cm; the root
/// keeps its own height), facing `angle`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reset {
    pub at: Vec3,
    pub angle: f32,
}

/// The body-root move a reset makes: from the root at `from` (m) facing
/// `from_angle` to the root at `to` (m) facing `angle`, applied to every
/// part so the body keeps its pose about the root.
pub fn reset_move(from: Vec3, from_angle: f32, to: Vec3, angle: f32) -> Affine3A {
    Affine3A::from_rotation_translation(Quat::from_rotation_y(angle), to)
        * Affine3A::from_quat(Quat::from_rotation_y(from_angle).inverse())
        * Affine3A::from_translation(-from)
}

/// The pose reset's move of one character: its root goes to the reset's
/// place at its own height, its model faces the reset's angle, every part
/// of its body moves with it and the body is woken for
/// [`golem::POSE_TICKS`] ticks. A standing body is carried with its root
/// by its next aim ([`Ragdoll::carried`]: its parts, feet, targets and
/// frame); any other (a fallen or limp body) is placed at once. Returns
/// the root's new place (m).
fn reset_character(
    reset: Reset,
    model: &GlobalTransform,
    transform: &mut Transform,
    parent: Option<&ChildOf>,
    roots: &mut ReturnRoots,
    body: Option<&mut Ragdoll>,
) -> Vec3 {
    let from_angle = exa_physics::body::locomotion::heading_of(model.rotation() * Vec3::Z);
    let root = parent.and_then(|p| roots.get_mut(p.parent()).ok());
    let (from, to, parent_turn) = match root {
        Some(mut root) => {
            let from = root.translation;
            let to = Vec3::new(reset.at.x / 100.0, from.y, reset.at.z / 100.0);
            root.translation = to;
            (from, to, root.rotation)
        }
        None => {
            let from = transform.translation;
            let to = Vec3::new(reset.at.x / 100.0, from.y, reset.at.z / 100.0);
            transform.translation = to;
            (from, to, Quat::IDENTITY)
        }
    };
    transform.rotation = parent_turn.inverse() * Quat::from_rotation_y(reset.angle);
    if let Some(body) = body {
        if !body.carried() {
            let moved = reset_move(from, from_angle, to, reset.angle);
            let worlds: Vec<Affine3A> = body.parts.iter().map(|p| moved * p.transform()).collect();
            body.place_at(&worlds);
        }
        body.wake_to(golem::POSE_TICKS as i32);
    }
    to
}

/// Turns a locomotion request to face `angle`.
fn face(locomotion: Option<Mut<Locomotion>>, angle: f32) {
    if let Some(mut l) = locomotion {
        l.heading = angle;
        l.look = angle;
        l.cursor = angle;
        l.desired = angle;
    }
}

/// The story revive's alarm return ([`crate::flow::Step::AlarmReturn`]):
/// taken out of the flow's steps before they are applied, so the guardians
/// and the player are reset before the save and the body's revive. When
/// the player was reset, the step becomes its motion request (0x1d).
#[allow(clippy::too_many_arguments)]
pub fn return_from_alarm(
    mut queued: ResMut<crate::flow::FlowSteps>,
    alarm: Res<GolemAlarm>,
    volumes: Res<AlarmVolumes>,
    level: Option<Res<LevelRooms>>,
    motions: Option<Res<Motions>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    mut players: ReturnPlayers,
    mut guardians: ReturnGuardians,
    mut roots: ReturnRoots,
) {
    let Some(at) = queued
        .steps
        .iter()
        .position(|step| *step == crate::flow::Step::AlarmReturn)
    else {
        return;
    };
    queued.steps.remove(at);
    let Some((_, player_gt, ..)) = players.iter().next() else {
        return;
    };
    let player_at = player_gt.translation() * 100.0;
    let bodies: Vec<QueryBody> = guardians
        .iter()
        .map(|(entity, gt, _, _, data, ..)| QueryBody {
            at: gt.translation() * 100.0,
            race_word: data.record.profession(),
            character: entity.to_bits(),
        })
        .collect();
    let offered = offered_in_rooms(level.as_deref(), player_at, bodies);
    let nodes = volumes.0.iter().map(|v| {
        (
            if v.alarm_node {
                golem::ALARM_NODE_PREFIX
            } else {
                ""
            },
            AlarmNode { bounds: v.bounds },
        )
    });
    let stand = motions
        .as_deref()
        .and_then(|m| m.action_name(golem::WAKE_POSE as u32))
        .map(str::to_string);
    let send_home = |body: QueryBody| {
        send_guardian_home(
            &mut guardians,
            &mut roots,
            Entity::from_bits(body.character),
            stand.as_deref(),
        )
    };
    let roll = || {
        bank.as_deref_mut()
            .map_or(0.5, |bank| bank.rng().unit_f64())
    };
    let Some(done) = golem::alarm_return(&alarm.armed, player_at, offered, send_home, nodes, roll)
    else {
        info!("alarm return: no alarm armed");
        return;
    };
    let Some(returned) = done.player else {
        info!(
            "alarm return around ({:.0}, {:.0}, {:.0}) cm: {} guardian(s) sent home, the player stays",
            player_at.x, player_at.y, player_at.z, done.sent_home
        );
        return;
    };
    let post = returned.post;
    for (entity, gt, mut transform, parent, body, condition, locomotion, player_body) in
        &mut players
    {
        let mut body = body;
        let to = reset_character(
            Reset {
                at: post.at,
                angle: post.angle,
            },
            gt,
            &mut transform,
            parent,
            &mut roots,
            body.as_deref_mut(),
        );
        if let Some(drive) = body.as_deref_mut().and_then(|b| b.drive.as_mut()) {
            drive.tone = returned.tones[0];
            drive.set_stance(returned.tones[1]);
        }
        if let Some(mut condition) = condition {
            condition.0 = returned.condition;
        }
        face(locomotion, post.angle);
        if let Some(mut player_body) = player_body {
            player_body.aim = post.angle;
        }
        info!(
            "alarm return: {} guardian(s) sent home; the player {entity:?} is put at ({:.0}, {:.0}, {:.0}) cm (post ({:.0}, {:.0}, {:.0})) facing {:.3} rad, condition {:#x}, motion {:#x}",
            done.sent_home,
            to.x * 100.0,
            to.y * 100.0,
            to.z * 100.0,
            post.at.x,
            post.at.y,
            post.at.z,
            post.angle,
            returned.condition,
            returned.request
        );
    }
    queued.steps.insert(
        at,
        crate::flow::Step::RequestAction(returned.request as u32),
    );
}

/// The homeward callback on one offered character: an awake guardian that
/// is not dying goes home and falls asleep. Returns whether it went.
fn send_guardian_home(
    guardians: &mut ReturnGuardians,
    roots: &mut ReturnRoots,
    entity: Entity,
    stand: Option<&str>,
) -> bool {
    let Ok((
        _,
        gt,
        mut transform,
        parent,
        mut data,
        mut npc,
        condition,
        body,
        flags,
        rest,
        animator,
        locomotion,
    )) = guardians.get_mut(entity)
    else {
        return false;
    };
    let latch = flags
        .as_deref()
        .is_some_and(|f| f.has(golem::SLEEP_SCALES_LATCH));
    let at = gt.translation() * 100.0;
    let state = GolemState {
        race_word: data.record.profession(),
        dormant: npc.statue,
        latch,
        at,
        angle: exa_physics::body::locomotion::heading_of(gt.rotation() * Vec3::Z),
        masses: Vec::new(),
        limb_scales: Vec::new(),
        glow_scales: Vec::new(),
    };
    let word = condition.as_deref().map_or(0, |c| c.0);
    let home = npc.guard_home.unwrap_or((Vec3::ZERO, 0.0));
    let Some(gone) = golem::go_home(&state, word, home) else {
        return false;
    };
    npc.guard_home = Some(gone.home);
    let mut body = body;
    if let Some(drive) = body.as_deref_mut().and_then(|b| b.drive.as_mut()) {
        drive.tone = gone.tones[0];
        drive.set_stance(gone.tones[1]);
    }
    if let (Some(mut anim), Some(stand)) = (animator, stand) {
        anim.play_loop(stand);
    }
    let reset = Reset {
        at: gone.home.0,
        angle: gone.home.1,
    };
    let to = reset_character(
        reset,
        gt,
        &mut transform,
        parent,
        roots,
        body.as_deref_mut(),
    );
    face(locomotion, reset.angle);
    {
        let brain = &mut *npc;
        exa_ai::motion::snap_to_body(&mut brain.gaze, &mut brain.turn, reset.angle);
    }
    // The sleep.
    let sleep = gone.sleep;
    data.record.set_profession(sleep.race_word);
    npc.statue = sleep.dormant;
    if let Some(mut condition) = condition {
        condition.0 = sleep.condition.unwrap_or(gone.condition);
    }
    if let Some(mut rest) = rest {
        rest.0 = true;
    }
    if sleep.latch && !latch {
        if let Some(mut flags) = flags {
            flags.0 |= golem::SLEEP_SCALES_LATCH;
        }
        if let Some(body) = body.as_deref_mut() {
            crate::golem_body::apply_sleep_scales(body);
        }
    }
    info!(
        "{entity:?} goes home to ({:.0}, {:.0}, {:.0}) cm facing {:.3} rad (root at ({:.0}, {:.0}, {:.0}) cm) and sleeps: race word {:#010x}, Stand 2, glow 'go' to {} (not faded live)",
        gone.home.0.x,
        gone.home.0.y,
        gone.home.0.z,
        gone.home.1,
        to.x * 100.0,
        to.y * 100.0,
        to.z * 100.0,
        sleep.race_word,
        sleep.glow
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ai::personality::{AI_FLAG, STATUE_FLAG};
    use exa_formats::character_stats::{BodyRules, CharacterStats};
    use exa_formats::characters::CharacterRecord;
    use exa_items::possession::Carried;
    use exa_physics::intersect::Obb;

    /// The exanima06 gate passed by token 474: a box 340 cm along world z,
    /// 300 along x, standing on the floor at (10.5, 0, 69) m.
    fn gate() -> AlarmVolume {
        AlarmVolume {
            bounds: Obb::new(
                Vec3::new(1050.0, 100.0, 6900.0),
                Vec3::new(170.0, 100.0, 150.0),
                [Vec3::Z, Vec3::Y, -Vec3::X],
            ),
            radius: (170.0f32 * 170.0 + 150.0 * 150.0 + 100.0 * 100.0).sqrt(),
            pass: 474,
            alarm_node: true,
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<exa_audio::PointSounded>()
            .insert_resource(AlarmVolumes(vec![gate()]))
            .init_resource::<GolemAlarm>()
            .add_systems(Update, check_alarm_zones);
        app
    }

    fn player(app: &mut App, at_m: Vec3, carried: &[u32]) -> Entity {
        let possessions = Possessions {
            carried: carried
                .iter()
                .map(|&id| Carried {
                    id,
                    ..Carried::default()
                })
                .collect(),
            ..Possessions::default()
        };
        app.world_mut()
            .spawn((
                RatedPlayer,
                GlobalTransform::from_translation(at_m),
                StateFlags::default(),
                possessions,
            ))
            .id()
    }

    /// A Golem Guard (race byte 16) at `at_m`, a statue when `statue`.
    fn golem(app: &mut App, at_m: Vec3, statue: bool, rules: u32) -> Entity {
        let record = CharacterRecord {
            race: 16,
            profession_rest: if statue { STATUE_FLAG } else { 0 },
            ..CharacterRecord::default()
        };
        let mut roll = 0;
        let npc = Npc::of_record(&record, &mut roll);
        assert_eq!(npc.statue, statue);
        let fighter = Fighter::of_stats(
            None,
            CharacterStats {
                rules: BodyRules(rules),
                ..CharacterStats::default()
            },
        );
        app.world_mut()
            .spawn((
                GlobalTransform::from_translation(at_m),
                CharacterData::of_record(record),
                npc,
                Combatant::default(),
                fighter,
            ))
            .id()
    }

    fn walk(app: &mut App, player: Entity, at_m: Vec3) {
        app.world_mut()
            .entity_mut(player)
            .insert(GlobalTransform::from_translation(at_m));
        app.update();
    }

    /// Walking into the gate without its token wakes the statue golems
    /// within 800 cm of its centre: their statue bit and byte clear, their
    /// home is where they stand and their only contact is the player at a
    /// concern of 10. A golem 9 m off, and one that is no statue, are left.
    #[test]
    fn entering_the_gate_wakes_the_statues_around_it() {
        let mut app = app();
        let player = player(&mut app, Vec3::new(10.5, 0.0, 66.0), &[]);
        let near = golem(&mut app, Vec3::new(9.0, 0.0, 75.0), true, 0);
        let far = golem(&mut app, Vec3::new(10.5, 0.0, 78.0), true, 0);
        let awake = golem(&mut app, Vec3::new(9.0, 0.0, 63.0), false, 0);
        app.update();
        // Outside the box: nothing yet.
        assert_eq!(app.world().resource::<GolemAlarm>().rung, 0);
        assert!(app.world().get::<Npc>(near).unwrap().statue);
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.5));
        let alarm = *app.world().resource::<GolemAlarm>();
        assert_eq!(alarm.rung, 1);
        assert!(alarm.armed.armed);
        assert_eq!(
            (alarm.armed.at, alarm.armed.pass),
            (gate().bounds.centre, 474)
        );
        let npc = app.world().get::<Npc>(near).unwrap();
        assert!(!npc.statue, "the ally sense's gate opens");
        let home = npc.guard_home.expect("a home").0;
        assert!((home - Vec3::new(900.0, 0.0, 7500.0)).length() < 1e-3);
        let word = app.world().get::<CharacterData>(near).unwrap().profession();
        assert_eq!(word & STATUE_FLAG, 0);
        assert_ne!(word & AI_FLAG, 0);
        let contacts = &app.world().get::<Combatant>(near).unwrap().contacts;
        assert_eq!(contacts.len(), 1);
        assert_eq!((contacts[0].character, contacts[0].concern), (player, 10.0));
        let brain = &app.world().get::<Npc>(near).unwrap().contacts;
        assert_eq!(brain.len(), 1);
        assert_eq!((brain[0].id, brain[0].concern), (player.to_bits(), 10.0));
        // 900 cm off: out of the query.
        assert!(app.world().get::<Npc>(far).unwrap().statue);
        // Not a statue: the callback refuses it and its contacts stay.
        assert!(
            app.world()
                .get::<Combatant>(awake)
                .unwrap()
                .contacts
                .is_empty()
        );
        assert!(app.world().get::<Npc>(awake).unwrap().guard_home.is_none());
        // Inside already: walking on does not ring again.
        walk(&mut app, player, Vec3::new(10.5, 0.0, 68.5));
        assert_eq!(app.world().resource::<GolemAlarm>().rung, 1);
    }

    /// A resting statue the sentry scaled wakes standing: its rest ends at
    /// once, its latch (the blow-turning bit) clears and its parts take their
    /// contact bases back, the fist fifty times its own.
    #[test]
    fn a_woken_statue_stands_and_loses_its_sleep_scales() {
        use exa_physics::body::PartSetup;
        let mut app = app();
        let player = player(&mut app, Vec3::new(10.5, 0.0, 66.0), &[]);
        let near = golem(&mut app, Vec3::new(9.0, 0.0, 75.0), true, 0);
        let part = |bone: usize, parent: Option<usize>| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let mut body = Ragdoll::new(&[part(1, None), part(17, Some(0))]).with_drive();
        body.set_fist(17, true);
        crate::golem_body::apply_sleep_scales(&mut body);
        let (pelvis, hand) = (body.contact_base(0).unwrap(), body.contact_base(1).unwrap());
        app.world_mut().entity_mut(near).insert((
            body,
            StateFlags(golem::SLEEP_SCALES_LATCH),
            crate::golem_body::StatueRest(true),
        ));
        app.update();
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.5));
        let w = app.world();
        assert!(!w.get::<Npc>(near).unwrap().statue);
        assert_eq!(
            w.get::<crate::golem_body::StatueRest>(near),
            Some(&crate::golem_body::StatueRest(false))
        );
        assert!(
            !w.get::<StateFlags>(near)
                .unwrap()
                .has(StateFlags::TURNS_BLOWS)
        );
        let body = w.get::<Ragdoll>(near).unwrap();
        assert_eq!(body.parts[0].pairing.mobility, pelvis);
        assert_eq!(body.parts[1].pairing.mobility, hand * 50.0);
    }

    /// The token lets its carrier through, also inside a carried
    /// container; another token does not.
    #[test]
    fn the_token_lets_the_player_through() {
        let mut app = app();
        let player = player(&mut app, Vec3::new(10.5, 0.0, 66.0), &[474]);
        let near = golem(&mut app, Vec3::new(9.0, 0.0, 75.0), true, 0);
        app.update();
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.5));
        assert!(app.world().get::<Npc>(near).unwrap().statue);
        assert!(!app.world().resource::<GolemAlarm>().armed.armed);
        // Back out and in again carrying token 475 instead.
        app.world_mut()
            .get_mut::<Possessions>(player)
            .unwrap()
            .carried[0]
            .id = 475;
        walk(&mut app, player, Vec3::new(10.5, 0.0, 66.0));
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.5));
        assert!(!app.world().get::<Npc>(near).unwrap().statue);
    }

    /// Under 3 cm from the last checked point the zones are not checked, so
    /// a small step over the face does not count as entering until the
    /// player has moved on.
    #[test]
    fn the_check_waits_for_three_centimetres() {
        let mut app = app();
        let player = player(&mut app, Vec3::new(10.5, 0.0, 67.29), &[]);
        let near = golem(&mut app, Vec3::new(9.0, 0.0, 75.0), true, 0);
        app.update();
        // 2 cm on: into the box, but not 3 cm from the checked point.
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.31));
        assert!(app.world().get::<Npc>(near).unwrap().statue);
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.33));
        assert!(!app.world().get::<Npc>(near).unwrap().statue);
    }

    /// The state-flag exemption ends the check at the zone; a golem whose
    /// body rules carry the wake exemption is accepted but not woken: it
    /// keeps its statue byte and still takes the player as its contact.
    #[test]
    fn exemptions_end_the_check_or_skip_the_wake() {
        let mut app = app();
        let player = player(&mut app, Vec3::new(10.5, 0.0, 66.0), &[]);
        let near = golem(&mut app, Vec3::new(9.0, 0.0, 75.0), true, 0);
        app.world_mut().get_mut::<StateFlags>(player).unwrap().0 = golem::ALARM_EXEMPT;
        app.update();
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.5));
        assert!(app.world().get::<Npc>(near).unwrap().statue);
        assert_eq!(app.world().resource::<GolemAlarm>().rung, 0);

        let mut app = self::app();
        let player = self::player(&mut app, Vec3::new(10.5, 0.0, 66.0), &[]);
        let exempt = golem(
            &mut app,
            Vec3::new(9.0, 0.0, 75.0),
            true,
            golem::WAKE_EXEMPT,
        );
        app.update();
        walk(&mut app, player, Vec3::new(10.5, 0.0, 67.5));
        assert!(app.world().get::<Npc>(exempt).unwrap().statue);
        let contacts = &app.world().get::<Combatant>(exempt).unwrap().contacts;
        assert_eq!((contacts.len(), contacts[0].concern), (1, 10.0));
        // The callback took it, so the ring counts.
        assert_eq!(app.world().resource::<GolemAlarm>().rung, 1);
    }

    /// A second zone with another token keeps the armed one unless the
    /// player carries the armed zone's token.
    #[test]
    fn a_second_zone_keeps_the_armed_one() {
        let other = AlarmVolume {
            bounds: Obb::axis_aligned(Vec3::new(0.0, 100.0, 0.0), Vec3::new(170.0, 100.0, 150.0)),
            pass: 476,
            ..gate()
        };
        let volumes = [gate(), other];
        let none = |_: u32| false;
        let rung = rung_zones(
            &volumes,
            Vec3::new(1050.0, 0.0, 6750.0),
            Vec3::ZERO,
            false,
            &none,
        );
        assert_eq!(rung.len(), 1);
        let mut alarm = GolemAlarm::default();
        assert!(golem::alarm_trigger(
            &mut alarm.armed,
            rung[0].bounds.centre,
            474,
            none
        ));
        assert!(!golem::alarm_trigger(
            &mut alarm.armed,
            Vec3::ZERO,
            476,
            none
        ));
        assert_eq!(alarm.armed.pass, 474);
        assert!(golem::alarm_trigger(
            &mut alarm.armed,
            Vec3::ZERO,
            476,
            |p| p == 474
        ));
        assert_eq!(alarm.armed.pass, 476);
    }

    /// The story revive's alarm return app: the gate armed, the steps the
    /// revive queued.
    fn return_app() -> App {
        let mut app = App::new();
        app.insert_resource(AlarmVolumes(vec![gate()]))
            .insert_resource(GolemAlarm {
                armed: AlarmZone {
                    armed: true,
                    at: gate().bounds.centre,
                    pass: 474,
                },
                rung: 1,
            })
            .insert_resource(crate::flow::FlowSteps {
                steps: vec![
                    crate::flow::Step::ClosePrelude,
                    crate::flow::Step::AlarmReturn,
                    crate::flow::Step::Save {
                        drop_on_low_health: false,
                    },
                ],
            })
            .add_systems(Update, return_from_alarm);
        app
    }

    /// A character model at `at_m` facing `yaw`, under a root carrying its
    /// place (as the play spawn builds one).
    fn character(app: &mut App, at_m: Vec3, yaw: f32) -> (Entity, Entity) {
        let root = app
            .world_mut()
            .spawn(Transform::from_translation(at_m))
            .id();
        let turn = Quat::from_rotation_y(yaw);
        let model = app
            .world_mut()
            .spawn((
                Transform::from_rotation(turn),
                GlobalTransform::from(Transform::from_translation(at_m).with_rotation(turn)),
                ChildOf(root),
            ))
            .id();
        (root, model)
    }

    /// A woken golem: statue byte clear, its home where it was woken.
    fn woken(app: &mut App, at_m: Vec3, home_cm: Vec3, home_angle: f32) -> (Entity, Entity) {
        let (root, model) = character(app, at_m, 1.0);
        let record = CharacterRecord {
            race: 16,
            profession_rest: STATUE_FLAG,
            ..CharacterRecord::default()
        };
        let mut roll = 0;
        let mut npc = Npc::of_record(&record, &mut roll);
        npc.statue = false;
        npc.guard_home = Some((home_cm, home_angle));
        let mut data = CharacterData::of_record(record);
        let word = data.record.profession() & !STATUE_FLAG;
        data.record.set_profession(word);
        app.world_mut().entity_mut(model).insert((
            data,
            npc,
            Condition::COMBAT_IDLE,
            StateFlags::default(),
            crate::golem_body::StatueRest(false),
        ));
        (root, model)
    }

    /// The knocked-out player at the gate's south face: the woken golems
    /// within 3000 cm go home (put at their homes, facing their home
    /// angles) and sleep (statue byte and bit, condition 0, statue rest);
    /// a golem 31 m off stays awake; the player is put 200 cm past the
    /// box's third-axis face, at x = 700 cm and z = 6900 cm on its own
    /// floor, facing +x across the gate, its condition dead; the step
    /// becomes its motion request 0x1d, before the save.
    #[test]
    fn the_alarm_return_sends_the_golems_home_and_puts_the_player_outside() {
        let mut app = return_app();
        let (player_root, player) = character(&mut app, Vec3::new(10.5, 0.0, 67.0), 0.0);
        app.world_mut()
            .entity_mut(player)
            .insert((RatedPlayer, Condition::DEAD));
        let home = Vec3::new(900.0, 0.0, 7500.0);
        let (golem_root, golem) = woken(&mut app, Vec3::new(10.0, 0.0, 66.0), home, 2.0);
        let far_home = Vec3::new(1050.0, 0.0, 9900.0);
        let (_, far) = woken(&mut app, Vec3::new(10.5, 0.0, 98.0), far_home, 0.0);
        app.update();
        let w = app.world();
        let npc = w.get::<Npc>(golem).unwrap();
        assert!(npc.statue, "asleep");
        assert_eq!(npc.guard_home, Some((home, 2.0)));
        let word = w.get::<CharacterData>(golem).unwrap().profession();
        assert_ne!(word & STATUE_FLAG, 0);
        assert_eq!(w.get::<Condition>(golem).unwrap().0, 0);
        assert!(w.get::<crate::golem_body::StatueRest>(golem).unwrap().0);
        assert!(
            w.get::<StateFlags>(golem)
                .unwrap()
                .has(golem::SLEEP_SCALES_LATCH)
        );
        let root = w.get::<Transform>(golem_root).unwrap().translation;
        assert!((root - Vec3::new(9.0, 0.0, 75.0)).length() < 1e-5);
        let facing = w.get::<Transform>(golem).unwrap().rotation * Vec3::Z;
        assert!((facing - Vec3::new(2.0f32.sin(), 0.0, 2.0f32.cos())).length() < 1e-5);
        // 3100 cm off: out of the query, still awake.
        assert!(!w.get::<Npc>(far).unwrap().statue);
        // The player.
        let at = w.get::<Transform>(player_root).unwrap().translation;
        assert!((at - Vec3::new(7.0, 0.0, 69.0)).length() < 1e-4, "{at}");
        let facing = w.get::<Transform>(player).unwrap().rotation * Vec3::Z;
        assert!((facing - Vec3::X).length() < 1e-3, "{facing}");
        assert_eq!(w.get::<Condition>(player).unwrap().0, 0xA00);
        assert_eq!(
            w.resource::<crate::flow::FlowSteps>().steps,
            vec![
                crate::flow::Step::ClosePrelude,
                crate::flow::Step::RequestAction(0x1d),
                crate::flow::Step::Save {
                    drop_on_low_health: false
                },
            ]
        );
        // The alarm stays armed.
        assert!(w.resource::<GolemAlarm>().armed.armed);
    }

    /// With every golem asleep (or none near) the return sends none home
    /// and leaves the player where it fell; disarmed it does nothing. The
    /// step is taken either way.
    #[test]
    fn the_alarm_return_leaves_the_player_without_a_golem_sent_home() {
        let mut app = return_app();
        let (player_root, player) = character(&mut app, Vec3::new(10.5, 0.0, 67.0), 0.0);
        app.world_mut().entity_mut(player).insert(RatedPlayer);
        let (_, golem) = woken(
            &mut app,
            Vec3::new(10.0, 0.0, 66.0),
            Vec3::new(900.0, 0.0, 7500.0),
            2.0,
        );
        app.world_mut().get_mut::<Npc>(golem).unwrap().statue = true;
        app.update();
        let w = app.world();
        let at = w.get::<Transform>(player_root).unwrap().translation;
        assert_eq!(at, Vec3::new(10.5, 0.0, 67.0));
        assert_eq!(w.resource::<crate::flow::FlowSteps>().steps.len(), 2);

        let mut app = return_app();
        app.insert_resource(GolemAlarm::default());
        let (player_root, player) = character(&mut app, Vec3::new(10.5, 0.0, 67.0), 0.0);
        app.world_mut().entity_mut(player).insert(RatedPlayer);
        let (_, golem) = woken(
            &mut app,
            Vec3::new(10.0, 0.0, 66.0),
            Vec3::new(900.0, 0.0, 7500.0),
            2.0,
        );
        app.update();
        let w = app.world();
        assert!(!w.get::<Npc>(golem).unwrap().statue);
        let at = w.get::<Transform>(player_root).unwrap().translation;
        assert_eq!(at, Vec3::new(10.5, 0.0, 67.0));
        assert_eq!(w.resource::<crate::flow::FlowSteps>().steps.len(), 2);
    }

    /// The reset's move keeps a part's place about the root: a point 1 m
    /// ahead of a root at the origin facing +z ends 1 m ahead of the new
    /// root facing +x.
    #[test]
    fn the_reset_move_carries_the_pose_about_the_root() {
        let moved = reset_move(
            Vec3::ZERO,
            0.0,
            Vec3::new(7.0, 0.0, 69.0),
            std::f32::consts::FRAC_PI_2,
        );
        let ahead = moved.transform_point3(Vec3::Z);
        assert!((ahead - Vec3::new(8.0, 0.0, 69.0)).length() < 1e-5);
    }
}
