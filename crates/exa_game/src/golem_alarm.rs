//! The alarm zones of the golem guardians, run live: the player stepping
//! into an alarm zone without its pass wakes the statue golems around it.
//!
//! - **Zone check.** Each frame, once the player is more than 3 cm from the
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
//! Not reproduced: the stance byte the wake clears (Exarust's statues never
//! set it), the glow fade of the skin material `go` to 1 (no live
//! skin-material fade: it is logged), the alarm return that sends awake golems home when the player
//! falls, the zone check's other trigger kinds (the any-character routine
//! and the level transitions, which have their own Exarust paths), and the
//! contents of a carried container that is a level-local item (the item
//! catalogue holds only the game database's containers). The checked point
//! starts at the origin [guess: the body is allocated zeroed]; the room
//! lists come from each character's current position ([guess: without a
//! level room grid every character is offered]).

use bevy::prelude::*;
use exa_ai::brain::{Npc, RatedPlayer};
use exa_ai::combatant::{Combatant, Contact};
use exa_ai::golem::{self, AlarmZone, GolemState, QueryBody};
use exa_character::record::CharacterData;
use exa_combat::{Fighter, StateFlags};
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::Possessions;
use exa_physics::body::Ragdoll;
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
                check_alarm_zones.after(crate::rise::restore_revived),
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
            Option<&mut AlarmCheckPoint>,
        ),
        With<RatedPlayer>,
    >,
    mut guardians: Guardians,
) {
    if volumes.0.is_empty() {
        return;
    }
    let Some((player, gt, flags, possessions, point)) = players.iter_mut().next() else {
        return;
    };
    let at = gt.translation() * 100.0;
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
    let body = |entity: Entity, gt: &GlobalTransform, data: &CharacterData| QueryBody {
        at: gt.translation() * 100.0,
        race_word: data.record.profession(),
        character: entity.to_bits(),
    };
    let Some(level) = level else {
        return guardians
            .iter()
            .map(|(entity, gt, data, ..)| body(entity, gt, data))
            .collect();
    };
    let mut offered = Vec::new();
    for room in query_rooms(level, centre) {
        offered.extend(
            guardians
                .iter()
                .filter(|(_, gt, ..)| {
                    exa_ai::rooms::level_room(&level.rooms, gt.translation() * 100.0) == room
                })
                .map(|(entity, gt, data, ..)| body(entity, gt, data)),
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
}
