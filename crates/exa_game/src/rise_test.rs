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
