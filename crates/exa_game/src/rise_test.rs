use bevy::math::Affine3A;
use exa_anim::Animator;
use exa_formats::character_stats::CharacterStats;
use exa_physics::body::{BodyStats, PartSetup, RaceMuscles};

use super::*;

/// A pelvis and a thorax (with a support pin) 30 cm above it and 30 cm
/// along +x: the chest lies toward +x.
fn corpse(race: RaceMuscles, stats: BodyStats) -> Ragdoll {
    let part = |bone, parent, at: Vec3, pins: Vec<Vec3>| PartSetup {
        bone,
        entity: Entity::PLACEHOLDER,
        parent,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins,
        spheres: None,
    };
    let mut body = Ragdoll::new(&[
        part(1, None, Vec3::new(2.0, 0.2, 3.0), Vec::new()),
        part(
            2,
            Some(0),
            Vec3::new(2.3, 0.5, 3.0),
            vec![Vec3::new(0.0, 0.1, 0.0)],
        ),
    ])
    .with_drive()
    .with_race(race)
    .with_stats(stats);
    body.go_limp();
    body
}

fn app() -> App {
    let mut app = App::new();
    app.add_systems(Update, (rise_the_dead, restore_revived).chain());
    app
}

fn fighter(red: f32, rules: u32) -> Fighter {
    let mut f = Fighter::of_stats(
        None,
        CharacterStats {
            rules: BodyRules(rules),
            voice_volume: 0.85,
            ..CharacterStats::default()
        },
    );
    f.health.red = red;
    f.health.yellow = 0.0;
    f
}

/// Spawns a dead character: its controller root (whose controller went)
/// and its body under it, keeping what go-limp took.
fn spawn_dead(app: &mut App, red: f32, rules: u32, race: RaceMuscles) -> (Entity, Entity) {
    let stats = BodyStats {
        scale: 0.9,
        ..BodyStats::HUMAN
    };
    let root = app.world_mut().spawn(Transform::default()).id();
    let body = app
        .world_mut()
        .spawn((
            Transform::default(),
            corpse(race, stats),
            Condition::DEAD,
            fighter(red, rules),
            DeathClock {
                wake_at_ms: Some(10_000),
                died_at_ms: 0,
            },
            CharacterVoice::default(),
            Hands::default(),
            exa_combat::collapse::Collapse::default(),
            LimpKept {
                animator: Some(Animator::new("Default", "Fall Backward")),
                character: Some((root, CharacterBody::default())),
            },
            ChildOf(root),
        ))
        .id();
    (root, body)
}

/// Before the rise clock nothing happens; strictly past it the corpse is
/// revived knocked out with the race's own drive, its voice and yellow
/// back, its clips, controller and heading restored, the clock gone.
#[test]
fn a_dead_body_rises_past_its_clock_with_its_race() {
    let mut app = app();
    let race = RaceMuscles {
        stance: 0.7,
        clip_rate: 0.8,
        ..RaceMuscles::HUMAN
    };
    let (root, body) = spawn_dead(&mut app, 0.6, BodyRules::MAY_RISE, race);
    // 10 000 ms is 833.33 ticks of 12 ms: tick 833 is 9 996 ms.
    app.insert_resource(BodyTicks(833));
    app.update();
    assert_eq!(
        *app.world().get::<Condition>(body).unwrap(),
        Condition::DEAD
    );
    app.insert_resource(BodyTicks(834));
    app.update();
    app.update();
    let world = app.world();
    assert_eq!(*world.get::<Condition>(body).unwrap(), Condition::DOWN);
    assert_eq!(world.get::<StateFlags>(body).unwrap().0 & 0xF0, 0x30);
    assert_eq!(world.get::<KnockOut>(body).unwrap().wake_at_ms, 10_008);
    let f = world.get::<Fighter>(body).unwrap();
    assert_eq!(f.health.yellow, 0.3);
    assert_eq!(world.get::<CharacterVoice>(body).unwrap().volume, 0.85);
    let ragdoll = world.get::<Ragdoll>(body).unwrap();
    let d = ragdoll.drive.as_ref().expect("muscles back");
    assert_eq!(d.race, race);
    assert_eq!(d.stats.scale, 0.9);
    assert_eq!((d.tone, d.stance()), (0.0, 0.0));
    assert!(world.get::<DeathClock>(body).is_none());
    assert!(world.get::<exa_combat::collapse::Collapse>(body).is_none());
    assert!(world.get::<LimpKept>(body).is_none());
    assert_eq!(world.get::<Animator>(body).unwrap().action, "Fall Backward");
    assert!(world.get::<TickedClips>(body).is_some());
    assert!(world.get::<CharacterBody>(root).is_some());
    let at = world.get::<Transform>(root).unwrap().translation;
    assert!((at.x - 2.0).abs() < 1e-5 && (at.z - 3.0).abs() < 1e-5);
    // The chest lies toward +x: the body faces +x (a yaw of π/2).
    let facing = world.get::<Transform>(body).unwrap().rotation * Vec3::Z;
    assert!((facing - Vec3::X).length() < 1e-5, "{facing}");
}

/// Red under the double 0.01 clears the may-rise rule and the body stays
/// dead; the race's keep-rising mark keeps the rule; the blocking bit and
/// a disabled clock never rise.
#[test]
fn too_little_red_stays_dead() {
    let mut app = app();
    let (_, low) = spawn_dead(&mut app, 0.005, BodyRules::MAY_RISE, RaceMuscles::HUMAN);
    let (_, kept) = spawn_dead(&mut app, 0.005, BodyRules::MAY_RISE, RaceMuscles::HUMAN);
    app.world_mut()
        .get_mut::<Fighter>(kept)
        .unwrap()
        .first_flag_red = true;
    let (_, blocked) = spawn_dead(
        &mut app,
        1.0,
        BodyRules::MAY_RISE | BodyRules::NO_RED_REGENERATION,
        RaceMuscles::HUMAN,
    );
    app.insert_resource(BodyTicks(10_000));
    app.update();
    let world = app.world();
    let rules = |e| world.get::<Fighter>(e).unwrap().stats.rules.0;
    assert_eq!(rules(low) & BodyRules::MAY_RISE, 0);
    assert_eq!(rules(kept) & BodyRules::MAY_RISE, BodyRules::MAY_RISE);
    for e in [low, kept, blocked] {
        assert_eq!(*world.get::<Condition>(e).unwrap(), Condition::DEAD);
        assert!(world.get::<Ragdoll>(e).unwrap().drive.is_none());
    }
}

/// The rule's reading: the clock is strictly passed, a disabled one never.
#[test]
fn the_rise_state_reads_the_rule_and_clock() {
    let may = BodyRules(BodyRules::MAY_RISE);
    let s = rise_state(may, Condition::DEAD, Some(100), 100, 1.0, false, false);
    assert!(s.may_rise && s.dead && !s.due);
    assert!(rise_state(may, Condition::DEAD, Some(100), 101, 1.0, false, false).due);
    assert!(!rise_state(may, Condition::DEAD, None, u32::MAX, 1.0, false, false).due);
    assert_eq!(heading_yaw(Vec2::ZERO), None);
    assert_eq!(heading_yaw(Vec2::Y), Some(0.0));
}

/// The unarmed set-up's test reads the record's out set: a dropped weapon
/// left it empty, a kept shield does not; without a record the hands are
/// read, a shield or an off-hand light filling the off hand.
#[test]
fn the_unarmed_set_up_reads_the_record_out_set() {
    use exa_character::held::{HeldItem, ShieldStyle};
    use exa_items::possession::{Hand, Possessions};
    let sword = Hand { id: 5, flags: 0 };
    let shield = Hand { id: 6, flags: 0 };
    let mut record = Possessions {
        out: [sword, shield],
        ..Possessions::default()
    };
    assert!(!weapon_slots_empty(Some(&record), None));
    record.forget_loose(|id| id == 6);
    assert!(!weapon_slots_empty(Some(&record), None));
    record.forget_loose(|_| false);
    assert!(weapon_slots_empty(Some(&record), None));
    // The record wins over the hands.
    let armed = Hands {
        main: Some(HeldItem::weapon(0x11)),
        ..Hands::default()
    };
    assert!(weapon_slots_empty(Some(&record), Some(&armed)));
    assert!(!weapon_slots_empty(None, Some(&armed)));
    assert!(weapon_slots_empty(None, Some(&armed.let_go())));
    assert!(weapon_slots_empty(None, None));
    let shielded = Hands {
        shield: Some(ShieldStyle::Plain),
        ..Hands::default()
    };
    assert!(!weapon_slots_empty(None, Some(&shielded.let_go())));
    let lit = Hands {
        off_light: true,
        ..Hands::default()
    };
    assert!(!weapon_slots_empty(None, Some(&lit)));
    assert!(weapon_slots_empty(None, Some(&lit.let_go())));
}

/// The rise and the restore run in the simulation's frame start, as the
/// original's AI manager runs them on its simulation thread: through the
/// fixed step, one world and the bridge (inline and on its thread) rise the
/// same corpse alike.
#[test]
fn the_rise_runs_alike_behind_the_bridge() {
    use exa_physics::sim_app::{SimMode, SimPreStep, SimShellPlugin};
    use exa_physics::sim_bridge::{SimBridgeAppExt, SimBridgePlugin};
    let run = |mode: Option<SimMode>| {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, exa_physics::body::BodyPlugin))
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(Time::<Fixed>::from_hz(exa_physics::STEP_HZ))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / exa_physics::STEP_HZ),
            ))
            .add_systems(SimPreStep, (rise_the_dead, restore_revived).chain());
        if let Some(mode) = mode {
            app.add_plugins((
                SimShellPlugin { mode },
                SimBridgePlugin { extra: Vec::new() },
            ))
            .bridge_copy::<Condition>()
            .bridge_copy::<StateFlags>()
            .bridge_copy::<KnockOut>()
            .bridge_copy::<Fighter>()
            .bridge_copy::<DeathClock>()
            .bridge_copy::<CharacterVoice>()
            .bridge_copy::<Hands>()
            .bridge_copy::<exa_combat::collapse::Collapse>()
            .bridge_copy::<LimpKept>()
            .bridge_copy::<CharacterBody>()
            .bridge_copy::<TickedClips>()
            .bridge_copy::<Animator>()
            .bridge_copy::<PlayerBody>()
            .bridge_copy::<exa_ai::brain::Npc>()
            .bridge_copy::<exa_ai::roles::Roles>()
            .bridge_copy::<exa_items::possession::Possessions>()
            .bridge_copy::<exa_character::held::HeldMoveset>();
        }
        let (root, body) = spawn_dead(&mut app, 0.6, BodyRules::MAY_RISE, RaceMuscles::HUMAN);
        app.insert_resource(BodyTicks(834));
        for _ in 0..4 {
            app.update();
        }
        let w = app.world();
        (
            format!("{:?}", w.get::<Condition>(body)),
            w.get::<Ragdoll>(body).unwrap().drive.is_some(),
            w.get::<TickedClips>(body).is_some(),
            w.get::<CharacterBody>(root).is_some(),
            w.get::<LimpKept>(body).is_none(),
            w.get::<DeathClock>(body).is_none(),
        )
    };
    let alone = run(None);
    assert_eq!(
        alone,
        (
            format!("{:?}", Some(&Condition::DOWN)),
            true,
            true,
            true,
            true,
            true
        )
    );
    for mode in [SimMode::Inline, SimMode::Thread] {
        assert_eq!(run(Some(mode)), alone, "{mode:?}");
    }
}

/// Raise Dead's raise of a corpse without decay: the revival leaves the
/// feral level 0.2 and the decayed rules bit, the revive knocks the body
/// out with its muscles back, and its yellow and red become a half and
/// full over the revive's own yellow; the death clock goes. Command Dead's
/// end then lays the risen body down again: it collapses.
#[test]
fn a_raised_corpse_gets_up_and_command_dead_lays_it_down() {
    use crate::mind::casts::{self, MindBody};
    let mut app = app();
    let (_root, body) = spawn_dead(&mut app, 0.0, 0, RaceMuscles::HUMAN);
    app.insert_resource(BodyTicks(100));
    // The cast's own revival values (no decay, the character allows it).
    let revival = casts::revive(&mut MindBody::new(0), 0.0, 1.0);
    app.world_mut()
        .run_system_cached_with(crate::mind::runtime::raise_corpse, (body, Some(revival)))
        .unwrap();
    app.world_mut().flush();
    let world = app.world();
    assert_eq!(*world.get::<Condition>(body).unwrap(), Condition::DOWN);
    let f = world.get::<Fighter>(body).unwrap();
    assert_eq!((f.health.yellow, f.health.red), (0.5, 1.0));
    assert_eq!(f.stats.feral, 0.2);
    assert_ne!(f.stats.rules.0 & casts::DECAYED_RULE, 0);
    assert_eq!(f.stats.max_energy, revival.most_energy);
    assert!(world.get::<DeathClock>(body).is_none());
    assert!(world.get::<Ragdoll>(body).unwrap().drive.is_some());
    // The go-limp undo gives the clips back; then the end lays it down.
    app.update();
    assert!(app.world().get::<Animator>(body).is_some());
    app.world_mut()
        .run_system_cached_with(crate::mind::runtime::lay_down, body)
        .unwrap();
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<Condition>(body).unwrap(),
        Condition::COLLAPSING
    );
    assert!(
        app.world()
            .get::<exa_combat::collapse::Collapse>(body)
            .is_some()
    );
}

/// A raise without a revival (the body had decay) keeps the feral level;
/// a body that is no longer dead does not rise.
#[test]
fn a_raise_without_decay_left_keeps_the_feral_level() {
    let mut app = app();
    let (_root, body) = spawn_dead(&mut app, 0.4, 0, RaceMuscles::HUMAN);
    app.world_mut()
        .get_mut::<Fighter>(body)
        .unwrap()
        .stats
        .feral = 0.6;
    app.world_mut()
        .run_system_cached_with(crate::mind::runtime::raise_corpse, (body, None))
        .unwrap();
    let f = app.world().get::<Fighter>(body).unwrap();
    assert_eq!(f.stats.feral, 0.6);
    assert_eq!((f.health.yellow, f.health.red), (0.5, 1.0));
    assert_eq!(
        *app.world().get::<Condition>(body).unwrap(),
        Condition::DOWN
    );
    // Raised again while down: nothing happens.
    app.world_mut().get_mut::<Fighter>(body).unwrap().health.red = 0.7;
    app.world_mut()
        .run_system_cached_with(crate::mind::runtime::raise_corpse, (body, None))
        .unwrap();
    assert_eq!(app.world().get::<Fighter>(body).unwrap().health.red, 0.7);
}
