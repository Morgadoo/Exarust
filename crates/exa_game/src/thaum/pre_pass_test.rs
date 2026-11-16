use bevy::math::Affine3A;
use exa_ai::{DuelPowers, Duellist};
use exa_physics::body::{BodyPreStep, BodySystems, CharacterTicks, PartSetup, Ragdoll};

use super::*;
use crate::effects::blast::CHARGING;
use crate::thaum::cast::{CastPlugin, PowerCaster, advise_duellists, cast_for_duellists};

/// A one-part body of `bone` at `at` (m) with one 0.3 m sphere.
fn body(bone: usize, at: Vec3, radius: f32) -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        spheres: Some(vec![(Vec3::ZERO, radius)]),
        origin: Vec3::ZERO,
        links: Vec::new(),
        foot: None,
        pins: Vec::new(),
    }])
}

/// The target's particles as each stage of the tick saw them: at the
/// first pass's end and at the character ticks.
#[derive(Resource, Default)]
struct Seen {
    pre: Vec<[Vec3; 4]>,
    ticks: Vec<[Vec3; 4]>,
}

#[derive(Resource)]
struct Target(Entity);

fn see_pre(target: Res<Target>, bodies: Query<&Ragdoll>, mut seen: ResMut<Seen>) {
    if let Ok(body) = bodies.get(target.0) {
        seen.pre.push(body.parts[0].positions);
    }
}

fn see_ticks(target: Res<Target>, bodies: Query<&Ragdoll>, mut seen: ResMut<Seen>) {
    if let Ok(body) = bodies.get(target.0) {
        seen.ticks.push(body.parts[0].positions);
    }
}

/// A duellist whose blast is thrown at a body 0.35 m ahead (the casting
/// test's set-up), with probes after the first pass and at the character
/// ticks. Returns the app, the duellist and the target.
fn thrown_blast(disabled: bool) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(CastPlugin)
        .init_resource::<Seen>()
        .configure_sets(
            FixedUpdate,
            (BodyPreStep, CharacterTicks, BodySystems).chain(),
        )
        .add_systems(
            FixedUpdate,
            (
                see_pre.in_set(BodyPreStep).after(super::meet_bodies_first),
                see_ticks.in_set(CharacterTicks),
            ),
        );
    let mut target_body = body(9, Vec3::new(0.0, 0.0, 0.35), 0.3);
    target_body.set_disabled(disabled);
    let target = app.world_mut().spawn(target_body).id();
    app.insert_resource(Target(target));
    let mut caster = PowerCaster::new(&[0x201]);
    caster.body.focus = 1.0;
    let npc = app
        .world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            body(17, Vec3::ZERO, 0.1),
        ))
        .id();
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(0x201);
    app.world_mut()
        .run_system_cached(advise_duellists)
        .expect("advise");
    app.world_mut()
        .run_system_cached(cast_for_duellists)
        .expect("cast");
    let state = |app: &mut App| {
        let mut q = app.world_mut().query::<&LiveBlast>();
        q.iter(app.world()).next().map(|b| b.blast.state)
    };
    let mut ticks = 0;
    while state(&mut app) == Some(CHARGING) {
        app.world_mut().run_schedule(FixedUpdate);
        ticks += 1;
        assert!(ticks < 200);
    }
    assert_eq!(state(&mut app), Some(THROWN));
    (app, npc, target)
}

/// A thrown blast meets the bodies in the sector step's first pass, before
/// any character ticks: every tick the target moves, it has moved by the
/// end of the first pass, and the character ticks see the same particles;
/// the blast's own tick (after the solver) no longer touches a body.
#[test]
fn a_thrown_blast_meets_the_bodies_before_the_character_ticks() {
    let (mut app, _, target) = thrown_blast(false);
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    app.world_mut().resource_mut::<Seen>().pre.clear();
    app.world_mut().resource_mut::<Seen>().ticks.clear();
    let mut ends = Vec::new();
    for _ in 0..20 {
        app.world_mut().run_schedule(FixedUpdate);
        ends.push(app.world().get::<Ragdoll>(target).unwrap().parts[0].positions);
    }
    let seen = app.world().resource::<Seen>();
    assert_eq!(seen.pre.len(), 20);
    assert_eq!(seen.pre, seen.ticks);
    // Nothing after the character ticks moves the target: each tick ends
    // where its first pass left it.
    assert_eq!(seen.pre, ends);
    // The throw's tick holds the chunks in the hand, so the first pass
    // after it meets nothing; the blast reaches the target later.
    assert_eq!(seen.pre[0], start);
    assert!(seen.pre.iter().any(|p| *p != start));
}

/// A disabled body (a vanished one) is not offered to the first pass, so
/// the blast flies through it.
#[test]
fn a_disabled_body_is_not_offered_to_the_first_pass() {
    let (mut app, _, target) = thrown_blast(true);
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(
        app.world().get::<Ragdoll>(target).unwrap().parts[0].positions,
        start
    );
}

/// The caster's own parts are offered (they lie in the blast's cells) but
/// the slot skips them: the caster never moves.
#[test]
fn the_casters_own_parts_are_skipped() {
    let (mut app, npc, _) = thrown_blast(false);
    let start = app.world().get::<Ragdoll>(npc).unwrap().parts[0].positions;
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(
        app.world().get::<Ragdoll>(npc).unwrap().parts[0].positions,
        start
    );
}

/// The sector's body gates: a part whose bound sphere is apart from the
/// object's is not offered; one within reach is, unless its body is
/// disabled.
#[test]
fn the_sector_gates_offer_only_near_enabled_parts() {
    let near = body(9, Vec3::new(0.0, 0.0, 0.35), 0.3);
    let object = ObjectBounds {
        centre: Vec3::new(0.0, 0.0, 30.0),
        radius: 20.0,
    };
    assert!(offered(object, &near, 0));
    let far = body(9, Vec3::new(0.0, 0.0, 3.5), 0.3);
    assert!(!offered(object, &far, 0));
    let mut off = near.clone();
    off.set_disabled(true);
    assert!(!offered(object, &off, 0));
}
