//! The tick's stages in the order of the original's sector step and body
//! tick (`context/re/body-tick-order.md` §1, `master-body.md` §10): the
//! solver's 20 iterations, each a pair pass before any body's links, the
//! contacts dropped before body–body detection, which comes before the
//! level's, 8 friction passes with the pairs' before the level's, then the
//! post-step; and every stage of one character's tick before the next
//! character's.
use bevy::math::Affine3A;

use super::super::tests::floor_quad;
use super::*;
use crate::Physics;
use crate::body::{BodyPlugin, BodyTicks, PartSetup, Ragdoll, step_bodies};

/// A lone limp part (a forearm-sized ball, 6 cm) whose centre is at
/// height `y` (m).
fn ball(y: f32) -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone: 11,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(Vec3::Y * y),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }])
}

/// A limp body of two parts, the lower one placed 40 cm below where its
/// pivot holds it, so its links move it in the iterations; it starts
/// `clearance` (m) over the floor.
fn pulled_apart(clearance: f32) -> Ragdoll {
    let at = |y: f32| Affine3A::from_translation(Vec3::new(0.0, y, 0.0));
    Ragdoll::new(&[
        PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: at(1.0),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        },
        PartSetup {
            bone: 2,
            entity: Entity::PLACEHOLDER,
            parent: Some(0),
            world: at(0.2 + clearance),
            centre: Vec3::ZERO,
            half: Vec3::new(0.05, 0.2, 0.05),
            origin: Vec3::new(0.0, -0.4, 0.0),
            links: vec![],
            foot: None,
            pins: Vec::new(),
        },
    ])
}

/// What a stage saw when the solver called it: the stage, the iteration
/// it was given, the last body's highest particle, and whether the first
/// body held level contacts and its ticks.
#[derive(Debug, Clone, PartialEq)]
struct Call {
    stage: &'static str,
    iteration: usize,
    top: f32,
    touching: bool,
    ticks: u32,
}

/// A stage that writes down every call.
#[derive(Default)]
struct Record(Vec<Call>);

impl Record {
    fn note(&mut self, stage: &'static str, iteration: usize, bodies: &[&mut Ragdoll]) {
        let (Some(first), Some(last)) = (bodies.first(), bodies.last()) else {
            return;
        };
        let top = last
            .parts
            .iter()
            .flat_map(|p| p.positions)
            .map(|x| x.y)
            .fold(f32::MIN, f32::max);
        let touching = first.parts.iter().any(|p| !p.contacts.is_empty());
        self.0.push(Call {
            stage,
            iteration,
            top,
            touching,
            ticks: first.ticks,
        });
    }

    fn stages(&self) -> Vec<(&'static str, usize)> {
        self.0.iter().map(|c| (c.stage, c.iteration)).collect()
    }
}

impl SolverStage for Record {
    fn integrated(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("integrated", 0, bodies);
    }
    fn pair_pass(&mut self, bodies: &mut [&mut Ragdoll], iteration: usize) {
        self.note("pair_pass", iteration, bodies);
    }
    fn links(&mut self, bodies: &mut [&mut Ragdoll], iteration: usize) {
        self.note("links", iteration, bodies);
    }
    fn iterated(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("iterated", 0, bodies);
    }
    fn sim_objects(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("sim_objects", 0, bodies);
    }
    fn detect(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("detect", 0, bodies);
    }
    fn sim_object_contacts(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("sim_object_contacts", 0, bodies);
    }
    fn friction(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("friction", 0, bodies);
    }
    fn post_step(&mut self, bodies: &mut [&mut Ragdoll]) {
        self.note("post_step", 0, bodies);
    }
}

/// The solver calls its stages in the original's order: after the
/// integration, 20 iterations (numbered 1 to 20) of the pair pass then
/// links between bodies, around every body's own links (which move the
/// pulled-apart body between the two); the pair contacts dropped while the
/// level contacts of a ball on the floor still stand; the sim objects,
/// body–body detection and sim-object contacts with every contact dropped
/// and none found yet; 8 friction passes on the level contacts found this
/// tick; the post-step after every body's.
#[test]
fn the_solver_runs_its_stages_in_the_originals_order() {
    let ground = floor_quad(0.0, 5.0);
    // The ball settles on the floor (awake for 128 ticks more).
    let mut resting = ball(0.05);
    for _ in 0..64 {
        resting.tick(ground.as_slice());
    }
    assert!(!resting.asleep());
    let ticks = resting.ticks;
    let mut pulled = pulled_apart(0.3);
    let mut stages: Vec<Box<dyn SolverStage>> = vec![Box::new(Record::default())];
    solve_with(
        &mut [&mut resting, &mut pulled],
        ground.as_slice(),
        &mut stages,
    );
    let record = (&*stages[0] as &dyn std::any::Any)
        .downcast_ref::<Record>()
        .unwrap();

    let mut want = vec![("integrated", 0)];
    for i in 1..=20 {
        want.extend([("pair_pass", i), ("links", i)]);
    }
    want.extend([
        ("iterated", 0),
        ("sim_objects", 0),
        ("detect", 0),
        ("sim_object_contacts", 0),
    ]);
    want.extend([("friction", 0); 8]);
    want.push(("post_step", 0));
    assert_eq!(record.stages(), want);

    let calls = &record.0;
    let at = |stage: &str, iteration: usize| {
        calls
            .iter()
            .find(|c| c.stage == stage && c.iteration == iteration)
            .unwrap()
    };
    // Every body's links run between a pass's pair pass and its links
    // between bodies, and nothing runs between two passes.
    assert_ne!(at("pair_pass", 1).top, at("links", 1).top);
    for i in 2..=20 {
        assert_eq!(at("links", i - 1).top, at("pair_pass", i).top);
    }
    // The level contacts of the last tick hold through the iterations and
    // are dropped after the pair contacts; the level is tested after the
    // bodies are, before the friction passes.
    assert!(at("pair_pass", 1).touching && at("iterated", 0).touching);
    for stage in ["sim_objects", "detect", "sim_object_contacts"] {
        assert!(!at(stage, 0).touching, "{stage}");
    }
    assert!(
        calls
            .iter()
            .filter(|c| c.stage == "friction")
            .all(|c| c.touching)
    );
    // The post-step comes last, after every body's own.
    assert!(
        calls
            .iter()
            .all(|c| c.stage == "post_step" || c.ticks == ticks)
    );
    assert_eq!(at("post_step", 0).ticks, ticks + 1);
}

/// A stage with no work leaves the tick exactly as the solver without
/// stages leaves it.
#[test]
fn stages_without_work_change_nothing() {
    struct Idle;
    impl SolverStage for Idle {}
    let ground = floor_quad(0.0, 5.0);
    let (mut a, mut b) = (pulled_apart(0.3), pulled_apart(0.3));
    let mut idle: Vec<Box<dyn SolverStage>> = vec![Box::new(Idle), Box::new(Record::default())];
    for _ in 0..120 {
        super::super::solve(&mut [&mut a], ground.as_slice());
        solve_with(&mut [&mut b], ground.as_slice(), &mut idle);
    }
    for (p, q) in a.parts.iter().zip(&b.parts) {
        assert_eq!(p.positions, q.positions);
        assert_eq!(p.previous, q.previous);
    }
}

#[derive(Default)]
struct Other(u32);
impl SolverStage for Other {}

/// Stages are kept in the order they are added and found by their type.
#[test]
fn solver_stages_are_found_by_type() {
    let mut stages = SolverStages::default();
    assert!(stages.get::<Record>().is_none());
    stages.add(Other(3));
    stages.add(Record::default());
    stages.get_mut::<Other>().unwrap().0 += 1;
    assert_eq!(stages.get::<Other>().unwrap().0, 4);
    assert!(stages.get::<Record>().unwrap().0.is_empty());
    assert_eq!(stages.all().len(), 2);
}

/// Each run of a stage: the character it ticked and the stage's index in
/// [`CharacterStage::ORDER`].
#[derive(Resource, Default)]
struct Ran(Vec<(Entity, usize)>);

fn ran<const STAGE: usize>(ticked: Res<TickedCharacter>, mut ran: ResMut<Ran>) {
    ran.0.push((ticked.body, STAGE));
}

/// An app with the body plugin and a system in each stage of the character
/// tick, added last stage first.
fn staged_app() -> App {
    let mut app = App::new();
    app.add_plugins(BodyPlugin).init_resource::<Ran>();
    use CharacterStage::*;
    app.add_systems(
        CharacterTick,
        (
            ran::<8>.in_set(Carry),
            ran::<7>.in_set(Sampler),
            ran::<6>.in_set(Movement),
            ran::<5>.in_set(Attack),
            ran::<4>.in_set(Posture),
            ran::<3>.in_set(Behaviour),
            ran::<2>.in_set(Anchor),
            ran::<1>.in_set(Readiness),
            ran::<0>.in_set(Upkeep),
        ),
    );
    app
}

/// The nine stages of a character's tick run in the original's order, and
/// each character runs all of them before the next one starts (the order
/// given: the player last); the ticked character is gone afterwards.
#[test]
fn each_character_runs_every_stage_before_the_next() {
    assert_eq!(CharacterStage::ORDER.len(), 9);
    let mut app = staged_app();
    let world = app.world_mut();
    let other = world.spawn_empty().id();
    let player = world.spawn_empty().id();
    let tick = |body| TickedCharacter {
        body,
        character: None,
    };
    run_character_ticks(world, [tick(other), tick(player)]);
    let want: Vec<(Entity, usize)> = [other, player]
        .into_iter()
        .flat_map(|e| (0..9).map(move |s| (e, s)))
        .collect();
    assert_eq!(world.resource::<Ran>().0, want);
    assert!(!world.contains_resource::<TickedCharacter>());
}

/// The tick's sets run in the original's order: the pre-step, the
/// character ticks, the solver, the post-step.
#[test]
fn the_tick_runs_its_sets_in_order() {
    #[derive(Resource, Default)]
    struct Sets(Vec<&'static str>);
    fn note(name: &'static str) -> impl FnMut(ResMut<Sets>) {
        move |mut sets: ResMut<Sets>| sets.0.push(name)
    }
    let mut app = App::new();
    app.add_plugins(BodyPlugin)
        .insert_resource(Physics::default())
        .init_resource::<Sets>()
        .add_systems(
            FixedUpdate,
            (
                note("post").in_set(BodyPostStep),
                note("solver").after(step_bodies).in_set(BodySystems),
                note("characters").in_set(CharacterTicks),
                note("pre").in_set(BodyPreStep),
            ),
        );
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world().resource::<Sets>().0,
        ["pre", "characters", "solver", "post"]
    );
    assert_eq!(app.world().resource::<BodyTicks>().0, 1);
}
