use bevy::math::Affine3A;
use exa_physics::body::{PartSetup, Ragdoll};

use super::*;
use crate::thaum::displacement::CONSTANT;
use crate::thaum::energy::SHOCK;
use crate::thaum::force::BARRICADE;
use crate::thaum::power::force::SHELL;

/// A one-part body at the origin with the right hand's bone.
fn ragdoll() -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone: RIGHT_HAND,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::IDENTITY,
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        spheres: Some(vec![(Vec3::ZERO, 0.1)]),
        origin: Vec3::ZERO,
        links: Vec::new(),
        foot: None,
        pins: Vec::new(),
    }])
}

fn new_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(CastPlugin);
    app
}

fn caster(app: &mut App, ids: &[u32]) -> Entity {
    app.world_mut()
        .spawn((PlayerBody::new(0.0), PowerCaster::new(ids), ragdoll()))
        .id()
}

fn count<T: Component>(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&T>();
    query.iter(app.world()).count()
}

fn command(app: &mut App, caster: Entity, action: BarAction) {
    app.world_mut().write_message(BarCommand { caster, action });
}

fn cast(app: &mut App, caster: Entity) {
    command(app, caster, BarAction::Use);
    app.update();
}

fn cast_body(app: &App, caster: Entity) -> CastBody {
    app.world().get::<PowerCaster>(caster).unwrap().body.clone()
}

/// Force: the barricade's cast makes its remote power, reserves its focus
/// and raises its wall entity.
#[test]
fn the_barricade_casts_and_raises_a_wall() {
    let mut app = new_app();
    let caster = caster(&mut app, &[BARRICADE]);
    cast(&mut app, caster);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.last_result, code::STARTED);
    assert_eq!(state.active.len(), 1);
    assert!(matches!(state.active[0].class, ActiveClass::Remote(_)));
    assert!((state.body.reserved - 0.25).abs() < 1e-6);
    assert!((state.body.spent - 0.25).abs() < 1e-6);
    assert!(state.active[0].entity.is_some());
    assert_eq!(count::<LiveBarricade>(&mut app), 1);
}

/// Energy: the shock makes its class, starts cast motion 0x27, spawns the
/// hand arcs and spends its charge's focus over the first ticks.
#[test]
fn the_shock_casts_makes_arcs_and_charges() {
    let mut app = new_app();
    let caster = caster(&mut app, &[SHOCK]);
    cast(&mut app, caster);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.last_result, code::STARTED);
    assert_eq!(state.active.len(), 1);
    assert!(matches!(state.active[0].class, ActiveClass::Shock(_)));
    assert_eq!(
        state.body.cast_motion,
        Some(crate::thaum::energy::SHOCK_MOTION)
    );
    assert_eq!(count::<LiveArcs>(&mut app), 1);
    // The first tick installs the charge; the second adds its rate (1.0,
    // not past one) and spends the charge's focus.
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    let spent = cast_body(&app, caster).spent;
    assert!((spent - 0.12).abs() < 1e-5, "{spent}");
}

/// Displacement: constant displacement makes its class, holds its cost and
/// hangs its wisps on the body.
#[test]
fn constant_displacement_casts_and_hangs_wisps() {
    let mut app = new_app();
    let caster = caster(&mut app, &[CONSTANT]);
    cast(&mut app, caster);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.last_result, code::STARTED);
    assert_eq!(state.active.len(), 1);
    assert!(matches!(state.active[0].class, ActiveClass::Constant(_)));
    assert!((state.body.reserved - 0.35).abs() < 1e-6);
    assert_eq!(count::<LiveWisps>(&mut app), 1);
}

/// A power told to end releases, clears the cast motion and can start
/// again; the shell's second press ends it rather than releasing.
#[test]
fn an_ended_power_clears_its_cast_and_restarts() {
    let mut app = new_app();
    let caster = caster(&mut app, &[SHELL]);
    cast(&mut app, caster);
    assert_eq!(
        app.world().get::<PowerCaster>(caster).unwrap().last_result,
        1
    );
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(cast_body(&app, caster).casting, SHELL);
    // The second press finds the shell running and tells it to end.
    cast(&mut app, caster);
    let told = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(told.last_result, code::ENDED);
    assert_eq!(told.active[0].state, ENDING);
    app.world_mut().run_schedule(FixedUpdate);
    let ended = app.world().get::<PowerCaster>(caster).unwrap();
    assert!(ended.active.is_empty());
    assert_eq!(ended.body.casting, 0);
    assert!((ended.body.reserved - 0.0).abs() < 1e-6);
    // A new press starts it again instead of releasing a stale cast.
    cast(&mut app, caster);
    let again = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(again.last_result, code::STARTED);
    assert_eq!(again.active.len(), 1);
}

/// A cast short of focus, and one with no persistent slot, create nothing.
#[test]
fn a_refused_cast_creates_nothing() {
    let mut app = new_app();
    let poor = caster(&mut app, &[BARRICADE]);
    app.world_mut()
        .get_mut::<PowerCaster>(poor)
        .unwrap()
        .body
        .focus = 0.01;
    cast(&mut app, poor);
    let state = app.world().get::<PowerCaster>(poor).unwrap();
    assert_eq!(state.last_result, code::REFUSED);
    assert!(state.active.is_empty());
    assert_eq!(state.body.reserved, 0.0);
    assert_eq!(count::<LiveBarricade>(&mut app), 0);

    let mut app = new_app();
    let slotted = caster(&mut app, &[SHELL]);
    app.world_mut()
        .get_mut::<PowerCaster>(slotted)
        .unwrap()
        .body
        .slots = 0;
    cast(&mut app, slotted);
    let state = app.world().get::<PowerCaster>(slotted).unwrap();
    assert_eq!(state.last_result, code::REFUSED);
    assert!(state.active.is_empty());
}

/// The plugin's systems run three updates with and without a known power.
#[test]
fn the_cast_plugin_runs_with_and_without_powers() {
    for ids in [Vec::new(), vec![BARRICADE]] {
        let mut app = new_app();
        let caster = caster(&mut app, &ids);
        for _ in 0..3 {
            app.update();
        }
        for _ in 0..3 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert!(app.world().get::<PowerCaster>(caster).is_some());
    }
}

/// The bar opens, a digit picks and Q casts the picked slot; the chosen
/// slot moves with the pick.
#[test]
fn the_bar_opens_picks_and_casts() {
    let mut app = new_app();
    let caster = caster(&mut app, &[BARRICADE, SHOCK]);
    command(&mut app, caster, BarAction::Toggle);
    app.update();
    assert!(app.world().get::<PowerCaster>(caster).unwrap().bar_open);
    // The second slot is not the chosen one, so picking it selects it.
    command(&mut app, caster, BarAction::Pick(1));
    app.update();
    let picked = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(picked.bar.chosen, 2);
    assert_eq!(picked.record.slots[1] & word::CHOSEN, word::CHOSEN);
    command(&mut app, caster, BarAction::Use);
    app.update();
    let after = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(after.last_result, code::STARTED);
    assert!(!after.bar_open, "a cast closes the bar");
    assert!(matches!(after.active[0].class, ActiveClass::Shock(_)));
}

/// `--powers` reads decimal and hexadecimal ids.
#[test]
fn a_power_list_parses_both_bases() {
    assert_eq!(
        parse_power_list("0x201, 0X401, 10601"),
        Some(vec![0x201, 0x401, 10601])
    );
    assert_eq!(parse_power_list(" , "), Some(Vec::new()));
    assert_eq!(parse_power_list("0x20z"), None);
}
