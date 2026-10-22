use bevy::math::Affine3A;
use exa_physics::body::{PartSetup, Ragdoll};

use super::*;
use crate::thaum::displacement::{CONSTANT, VANISH};
use crate::thaum::energy::SHOCK;
use crate::thaum::force::BARRICADE;
use crate::thaum::power::force::SHELL;

/// A Displacement id for a slot the tree fills.
const DISPLACEMENT_ID: u32 = CONSTANT;

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

/// The player in combat mode: outside it the controller asks the hook
/// every tick.
fn caster(app: &mut App, ids: &[u32]) -> Entity {
    let mut player = PlayerBody::new(0.0);
    player.set_combat(true);
    app.world_mut()
        .spawn((player, PowerCaster::new(ids), ragdoll()))
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
    // not past one) and spends the charge's focus, of which the tick's
    // settle takes its first 0.001 share from the pool.
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    let body = cast_body(&app, caster);
    assert!((body.spent - 0.12 * 0.999).abs() < 1e-6, "{}", body.spent);
    assert!(
        (body.focus - (1.0 - 0.12 * 0.001)).abs() < 1e-6,
        "{}",
        body.focus
    );
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
    // The ticks moved a little of the hold into the capacity; after the
    // release exactly that much is still to come back.
    assert!(
        ended.body.reserved < 0.0 && ended.body.reserved > -0.01,
        "{}",
        ended.body.reserved
    );
    assert!(
        (ended.body.capacity - 1.0 - ended.body.reserved).abs() < 1e-6,
        "{} {}",
        ended.body.capacity,
        ended.body.reserved
    );
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

/// The bar cast entry takes the slot's word as it stands: a power the tree
/// added to the record after the bar was built is cast, not refused by a
/// lookup the original does not make.
#[test]
fn a_slot_the_tree_filled_is_cast() {
    let mut app = new_app();
    let caster = caster(&mut app, &[BARRICADE]);
    app.world_mut()
        .get_mut::<PowerCaster>(caster)
        .unwrap()
        .record
        .slots[1] = SHOCK;
    command(&mut app, caster, BarAction::Toggle);
    app.update();
    command(&mut app, caster, BarAction::Pick(1));
    app.update();
    cast(&mut app, caster);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.last_result, code::STARTED);
    assert!(matches!(state.active[0].class, ActiveClass::Shock(_)));
}

/// The body tick passes event 0 to every running power, so the release flag
/// alone ends none; with the cast motion playing it calls the hook and ends
/// the cast, and a charging shock's hook clears it.
#[test]
fn the_release_ends_the_cast_motion_through_its_hook() {
    let mut app = new_app();
    let caster = caster(&mut app, &[SHOCK]);
    cast(&mut app, caster);
    app.world_mut().run_schedule(FixedUpdate);
    let mut state = app.world_mut().get_mut::<PowerCaster>(caster).unwrap();
    assert_eq!(
        state.body.cast_hook,
        Some(super::super::power::energy::SHOCK_HOOK)
    );
    state.body.release = true;
    state.body.condition = 0;
    app.world_mut().run_schedule(FixedUpdate);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert!(state.body.release);
    assert_eq!(count::<LiveArcs>(&mut app), 1);
    app.world_mut()
        .get_mut::<PowerCaster>(caster)
        .unwrap()
        .body
        .condition = casting::CASTING;
    app.world_mut().run_schedule(FixedUpdate);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert!(!state.body.release);
    assert_eq!((state.body.casting, state.body.cast_hook), (0, None));
    assert_eq!(count::<LiveArcs>(&mut app), 0);
}

/// Outside combat mode the player's controller asks the hook every tick: a
/// shock's answers −1, so its cast ends and the charge clears.
#[test]
fn out_of_combat_mode_the_controller_ends_a_shock() {
    let mut app = new_app();
    let caster = caster(&mut app, &[SHOCK]);
    cast(&mut app, caster);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(cast_body(&app, caster).cast_hook.is_some());
    app.world_mut()
        .get_mut::<PlayerBody>(caster)
        .unwrap()
        .set_combat(false);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    let body = cast_body(&app, caster);
    assert_eq!((body.casting, body.cast_hook), (0, None));
    assert_eq!(body.hook_asked & casting::hook_event::OUT_OF_COMBAT, 0x8);
    assert_eq!(count::<LiveArcs>(&mut app), 0);
}

/// Vanish pressed while it runs tells the running vanish to let go.
#[test]
fn pressing_vanish_again_releases_it() {
    let mut app = new_app();
    let caster = caster(&mut app, &[VANISH]);
    cast(&mut app, caster);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.last_result, code::STARTED);
    assert!(matches!(&state.active[0].class, ActiveClass::Vanish(v) if !v.base.released));
    cast(&mut app, caster);
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.last_result, code::RELEASED);
    assert_eq!(state.active.len(), 1);
    assert!(matches!(&state.active[0].class, ActiveClass::Vanish(v) if v.base.released));
}

/// The screen's Escape closes the bar, and a tree session's record (a slot it
/// moved, a power it learnt) reaches the caster when the tree closes.
#[test]
fn the_tree_screen_and_the_caster_share_one_record() {
    use crate::power_screen::PowerScreen;
    let mut app = new_app();
    app.init_resource::<PowerScreen>();
    let caster = caster(&mut app, &[BARRICADE, SHOCK]);
    command(&mut app, caster, BarAction::Toggle);
    app.update();
    assert!(app.world().resource::<PowerScreen>().bar_open);
    // Escape on the screen closes the bar.
    app.world_mut().resource_mut::<PowerScreen>().bar_open = false;
    app.update();
    assert!(!app.world().get::<PowerCaster>(caster).unwrap().bar_open);
    assert!(!app.world().resource::<PowerScreen>().bar_open);

    // The tree opens, edits the record and closes.
    app.world_mut().resource_mut::<PowerScreen>().open = true;
    app.update();
    {
        let mut screen = app.world_mut().resource_mut::<PowerScreen>();
        screen.record.slots[2] = DISPLACEMENT_ID;
        screen.record.pairs.push(crate::thaum::tree::KnownPower {
            power: DISPLACEMENT_ID,
            flags: crate::thaum::tree::LEARNED,
        });
    }
    app.update();
    assert_eq!(
        app.world().get::<PowerCaster>(caster).unwrap().record.slots[2],
        0,
        "the open tree owns the record"
    );
    app.world_mut().resource_mut::<PowerScreen>().open = false;
    app.update();
    let state = app.world().get::<PowerCaster>(caster).unwrap();
    assert_eq!(state.record.slots[2], DISPLACEMENT_ID);
    assert_eq!(state.known.len(), 3);
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

/// A duellist with a shock and a target 3 m ahead.
fn duellist(app: &mut App) -> Entity {
    let target = app
        .world_mut()
        .spawn(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 3.0)))
        .id();
    let mut caster = PowerCaster::new(&[SHOCK]);
    caster.body.focus = 0.5;
    app.world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            ragdoll(),
        ))
        .id()
}

/// A duellist's slots are answered by their domain's AI evaluator: the
/// shock within five metres ahead advises 3; out of range, nothing.
#[test]
fn a_duellists_slots_are_answered_by_the_ai_evaluators() {
    let mut app = new_app();
    let npc = duellist(&mut app);
    app.update();
    let powers = app.world().get::<DuelPowers>(npc).unwrap();
    assert_eq!(powers.slots[0], SHOCK);
    assert_eq!(powers.answers[0], 3);
    assert_eq!(powers.slots[1..], [0; 9]);
    assert_eq!(powers.pool, 0.5);
    assert!(!powers.casting);
    let target = app.world().get::<Duellist>(npc).unwrap().target;
    *app.world_mut().get_mut::<GlobalTransform>(target).unwrap() =
        GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 6.0));
    app.update();
    assert_eq!(app.world().get::<DuelPowers>(npc).unwrap().answers[0], 0);
}

/// The power a duellist's combat state chose is cast as the bar's: the
/// shock's class and arcs, and the body then reads casting.
#[test]
fn a_duellists_chosen_power_is_cast() {
    let mut app = new_app();
    let npc = duellist(&mut app);
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(SHOCK);
    app.update();
    let state = app.world().get::<PowerCaster>(npc).unwrap();
    assert_eq!(state.last_result, code::STARTED);
    assert!(matches!(state.active[0].class, ActiveClass::Shock(_)));
    assert_eq!(count::<LiveArcs>(&mut app), 1);
    let powers = app.world().get::<DuelPowers>(npc).unwrap();
    assert_eq!(powers.chosen, None);
    assert!(powers.casting);
}

/// The combat state's stop: with the body's power answering nothing from
/// its evaluator, the hook is asked with 0x10 and a negative answer ends
/// the cast; while the evaluator answers, the hook is not asked.
#[test]
fn a_duellists_stop_asks_the_hook_once_its_power_answers_nothing() {
    let mut app = new_app();
    let npc = duellist(&mut app);
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(SHOCK);
    app.update();
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        cast_body(&app, npc).cast_hook,
        Some(super::super::power::energy::SHOCK_HOOK)
    );
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().probed = true;
    app.update();
    let body = cast_body(&app, npc);
    assert_eq!(body.casting, SHOCK);
    assert_eq!(body.hook_asked & casting::hook_event::RELEASE, 0);
    assert!(app.world().get::<DuelPowers>(npc).unwrap().running);
    let target = app.world().get::<Duellist>(npc).unwrap().target;
    *app.world_mut().get_mut::<GlobalTransform>(target).unwrap() =
        GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 6.0));
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().probed = true;
    app.update();
    let powers = app.world().get::<DuelPowers>(npc).unwrap();
    assert!(!powers.running && !powers.probed);
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));
}

/// A duellist's blast charges in its hand on its cast motion and hook,
/// throws once the charge passes 1 (the hook and power id go), pushes the
/// limb in front of it and leaves the scene when its chunks are spent.
#[test]
fn a_duellists_blast_charges_throws_and_strikes() {
    use crate::effects::blast::{CHARGING, THROWN};
    use crate::thaum::blast_live::{BLAST_HOOK, LiveBlast};
    use crate::thaum::force::BLAST_MOTION;
    let mut app = new_app();
    let target = app
        .world_mut()
        .spawn(Ragdoll::new(&[PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 0.0, 0.35)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            spheres: Some(vec![(Vec3::ZERO, 0.3)]),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        }]))
        .id();
    let mut caster = PowerCaster::new(&[0x201]);
    caster.body.focus = 1.0;
    let npc = app
        .world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            ragdoll(),
        ))
        .id();
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(0x201);
    app.update();
    let body = cast_body(&app, npc);
    assert_eq!(body.casting, 0x201);
    assert_eq!(body.cast_hook, Some(BLAST_HOOK));
    assert_eq!(count::<LiveBlast>(&mut app), 1);
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
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
    // Five sixths of a 12 ms tick a tick: a hundred of them sum to just
    // under 1 in single precision, so the charge passes 1 on tick 101.
    assert_eq!(ticks, 101);
    assert_eq!(state(&mut app), Some(THROWN));
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));
    assert!(body.spent > 0.0 || body.focus < 1.0);
    // The throw's own tick still holds the chunks in the hand (no move, no
    // impulse); they fly from the next.
    for _ in 0..5 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let moved = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    assert_ne!(moved, start);
    for _ in 0..100 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(count::<LiveBlast>(&mut app), 0);
    assert_eq!(BLAST_MOTION, 0x22);
}

/// A duellist's barrier rises before it, drains its focus, pushes a body
/// in its shell and ends when the release flag calls its hook.
#[test]
fn a_duellists_barrier_rises_pushes_and_ends() {
    use crate::effects::force_dome::Phase;
    use crate::thaum::barrier_live::{BARRIER, BARRIER_HOOK, BARRIER_MOTION, LiveBarrier};
    let mut app = new_app();
    app.insert_resource(BodyTicks(0));
    let target = app
        .world_mut()
        .spawn(Ragdoll::new(&[PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.85)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            spheres: Some(vec![(Vec3::ZERO, 0.1)]),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        }]))
        .id();
    let mut caster = PowerCaster::new(&[BARRIER]);
    caster.body.focus = 1.0;
    let npc = app
        .world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            ragdoll(),
        ))
        .id();
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(BARRIER);
    app.update();
    let body = cast_body(&app, npc);
    assert_eq!(body.casting, BARRIER);
    assert_eq!(body.cast_motion, Some(BARRIER_MOTION));
    assert_eq!(body.cast_hook, Some(BARRIER_HOOK));
    assert_eq!(count::<LiveBarrier>(&mut app), 1);
    let barrier = |app: &mut App| {
        let mut q = app.world_mut().query::<&LiveBarrier>();
        q.iter(app.world())
            .next()
            .map(|b| (b.barrier.dome.phase, b.barrier.dome.ripples.list.len()))
    };
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    let tick = |app: &mut App| {
        app.world_mut().resource_mut::<BodyTicks>().0 += 1;
        app.world_mut().run_schedule(FixedUpdate);
    };
    for _ in 0..30 {
        tick(&mut app);
    }
    let (phase, ripples) = barrier(&mut app).unwrap();
    assert_eq!(phase, Phase::Growing);
    assert!(ripples > 0);
    assert_ne!(
        app.world().get::<Ragdoll>(target).unwrap().parts[0].positions,
        start
    );
    assert!(cast_body(&app, npc).spent > 0.0);
    app.world_mut()
        .get_mut::<PowerCaster>(npc)
        .unwrap()
        .body
        .release = true;
    tick(&mut app);
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));
    assert_eq!(barrier(&mut app).unwrap().0, Phase::Shrinking);
    for _ in 0..200 {
        tick(&mut app);
    }
    assert_eq!(count::<LiveBarrier>(&mut app), 0);
}

/// A duellist's bubble grows where it was cast, pushes a body its shell
/// covers out, stands, and breaks when it is cast again.
#[test]
fn a_duellists_bubble_grows_pushes_and_breaks_when_cast_again() {
    use crate::effects::force_dome::Phase;
    use crate::thaum::bubble_live::{BUBBLE_HOOK, LiveBubble};
    use crate::thaum::power::force::{BUBBLE, BUBBLE_MOTION};
    let mut app = new_app();
    app.insert_resource(BodyTicks(0));
    let target = app
        .world_mut()
        .spawn(Ragdoll::new(&[PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 1.0, 1.1)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            spheres: Some(vec![(Vec3::ZERO, 0.1)]),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        }]))
        .id();
    let mut caster = PowerCaster::new(&[BUBBLE]);
    caster.body.focus = 1.0;
    let npc = app
        .world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            ragdoll(),
        ))
        .id();
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(BUBBLE);
    app.update();
    let body = cast_body(&app, npc);
    assert_eq!(body.casting, BUBBLE);
    assert_eq!(body.cast_motion, Some(BUBBLE_MOTION));
    assert_eq!(body.cast_hook, Some(BUBBLE_HOOK));
    let bubble = |app: &mut App| {
        let mut q = app.world_mut().query::<&LiveBubble>();
        q.iter(app.world())
            .next()
            .map(|b| (b.bubble.dome.phase, b.bubble.dome.at))
    };
    assert_eq!(
        bubble(&mut app),
        Some((Phase::Growing, Vec3::new(0.0, 100.0, 0.0)))
    );
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    let tick = |app: &mut App| {
        app.world_mut().resource_mut::<BodyTicks>().0 += 1;
        app.world_mut().run_schedule(FixedUpdate);
    };
    for _ in 0..110 {
        tick(&mut app);
    }
    assert_eq!(bubble(&mut app).unwrap().0, Phase::Standing);
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));
    assert_ne!(
        app.world().get::<Ragdoll>(target).unwrap().parts[0].positions,
        start
    );
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(BUBBLE);
    app.update();
    tick(&mut app);
    tick(&mut app);
    assert_eq!(bubble(&mut app).unwrap().0, Phase::Shrinking);
    for _ in 0..80 {
        tick(&mut app);
    }
    assert_eq!(count::<LiveBubble>(&mut app), 0);
}

/// A duellist's wave charges round it on the blast's hook, is thrown with
/// the hook and power id cleared, pushes a body its ring reaches and leaves
/// once its chunks are spent.
#[test]
fn a_duellists_wave_charges_throws_and_strikes() {
    use crate::effects::force_wave::WaveState;
    use crate::thaum::blast_live::BLAST_HOOK;
    use crate::thaum::wave_live::{LiveWave, WAVE, WAVE_MOTION};
    let mut app = new_app();
    let target = app
        .world_mut()
        .spawn(Ragdoll::new(&[PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 0.3, 1.3)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            spheres: Some(vec![(Vec3::ZERO, 0.3)]),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        }]))
        .id();
    let mut caster = PowerCaster::new(&[WAVE]);
    caster.body.focus = 1.0;
    let npc = app
        .world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            ragdoll(),
        ))
        .id();
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(WAVE);
    app.update();
    let body = cast_body(&app, npc);
    assert_eq!(body.casting, WAVE);
    assert_eq!(body.cast_motion, Some(WAVE_MOTION));
    assert_eq!(body.cast_hook, Some(BLAST_HOOK));
    assert_eq!(count::<LiveWave>(&mut app), 1);
    let state = |app: &mut App| {
        let mut q = app.world_mut().query::<&LiveWave>();
        q.iter(app.world()).next().map(|w| w.wave.state)
    };
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    let mut ticks = 0;
    while state(&mut app) == Some(WaveState::Charging) {
        app.world_mut().run_schedule(FixedUpdate);
        ticks += 1;
        assert!(ticks < 100);
    }
    assert_eq!(state(&mut app), Some(WaveState::Thrown));
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));
    for _ in 0..20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_ne!(
        app.world().get::<Ragdoll>(target).unwrap().parts[0].positions,
        start
    );
    for _ in 0..60 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(count::<LiveWave>(&mut app), 0);
}

/// A duellist's torrent charges on its hook, pours into the body ahead of
/// it while the hook stays, and the release ends it until it leaves.
#[test]
fn a_duellists_torrent_charges_pours_and_ends_on_the_release() {
    use crate::effects::torrent::{CHARGING, ENDING, RUNNING, TORRENT_MOTION};
    use crate::thaum::torrent_live::{LiveTorrent, TORRENT, TORRENT_HOOK};
    let mut app = new_app();
    let target = app
        .world_mut()
        .spawn(Ragdoll::new(&[PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 0.0, 0.6)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            spheres: Some(vec![(Vec3::ZERO, 0.3)]),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        }]))
        .id();
    let mut caster = PowerCaster::new(&[TORRENT]);
    caster.body.focus = 1.0;
    let npc = app
        .world_mut()
        .spawn((
            Duellist::new(target),
            DuelPowers::default(),
            caster,
            GlobalTransform::IDENTITY,
            ragdoll(),
        ))
        .id();
    app.world_mut().get_mut::<DuelPowers>(npc).unwrap().chosen = Some(TORRENT);
    app.update();
    let body = cast_body(&app, npc);
    assert_eq!(body.casting, TORRENT);
    assert_eq!(body.cast_motion, Some(TORRENT_MOTION));
    assert_eq!(body.cast_hook, Some(TORRENT_HOOK));
    assert_eq!(count::<LiveTorrent>(&mut app), 1);
    let state = |app: &mut App| {
        let mut q = app.world_mut().query::<&LiveTorrent>();
        q.iter(app.world()).next().map(|t| t.torrent.state)
    };
    let start = app.world().get::<Ragdoll>(target).unwrap().parts[0].positions;
    let mut ticks = 0;
    while state(&mut app) == Some(CHARGING) {
        app.world_mut().run_schedule(FixedUpdate);
        ticks += 1;
        assert!(ticks < 200);
    }
    assert_eq!(state(&mut app), Some(RUNNING));
    for _ in 0..30 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let body = cast_body(&app, npc);
    assert_eq!(
        (body.casting, body.cast_hook),
        (TORRENT, Some(TORRENT_HOOK))
    );
    assert_ne!(
        app.world().get::<Ragdoll>(target).unwrap().parts[0].positions,
        start
    );
    app.world_mut()
        .get_mut::<PowerCaster>(npc)
        .unwrap()
        .body
        .release = true;
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(state(&mut app), Some(ENDING));
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));
    for _ in 0..80 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(count::<LiveTorrent>(&mut app), 0);
}

/// A record's powers make its caster: the known pairs and the slots.
#[test]
fn a_records_powers_make_its_caster() {
    let mut slots = [0; 10];
    slots[0] = SHOCK;
    let record = exa_formats::characters::Powers {
        word: 0,
        slots,
        known: vec![exa_formats::characters::Power {
            id: SHOCK,
            flags: crate::thaum::tree::LEARNED,
            blob: None,
        }],
    };
    let caster = PowerCaster::of_record(&record);
    assert_eq!(caster.record.slots, slots);
    assert_eq!(caster.known.len(), 1);
    assert_eq!(caster.known[0].id, SHOCK);

    // A character's caster takes the body's technique word from its learnt
    // techniques and its persistent slots from Concentration: Sustain,
    // Continuity II and Abstraction over 15000 points give the sustain and
    // reserve bits and three slots with the focus item.
    use exa_formats::characters::{CONCENTRATION, CharacterRecord};
    let mut character = CharacterRecord::new();
    assert!(PowerCaster::of_character(&character).is_none());
    character.powers = Some(record);
    character.skills[CONCENTRATION].points = 15_000;
    character.skills[CONCENTRATION].techniques = [1, 3, 5, 0, 0, 0];
    let caster = PowerCaster::of_character(&character).unwrap();
    assert_eq!(
        caster.body.techniques,
        casting::SUSTAIN_TECHNIQUE | casting::RESERVE_TECHNIQUE
    );
    assert_eq!(caster.body.slots, 3);
    character.skills[CONCENTRATION].points = 10_000;
    assert_eq!(PowerCaster::of_character(&character).unwrap().body.slots, 3);
    character.skills[CONCENTRATION].points = 5_000;
    assert_eq!(PowerCaster::of_character(&character).unwrap().body.slots, 0);
}

/// A running shell puts its damping on the caster's fighter for the hit
/// formula, and its end takes it off.
#[test]
fn a_shell_reaches_the_hit_formula() {
    let mut app = new_app();
    let caster = caster(&mut app, &[SHELL]);
    cast(&mut app, caster);
    let factors = |app: &App| *app.world().get::<PowerFactors>(caster).unwrap();
    assert_eq!(factors(&app).shell, 0.5);
    assert_eq!(factors(&app).strikes, 0.0);
    cast(&mut app, caster);
    app.world_mut().run_schedule(FixedUpdate);
    app.update();
    assert_eq!(factors(&app).shell, 0.0);
}

/// Force Strikes takes its hook with its cast motion, so a second cast is
/// refused until the motion is replaced; a refused motion makes no strikes.
#[test]
fn strikes_take_their_hook_with_the_motion() {
    use crate::thaum::casting::CastHook;
    use crate::thaum::force::{STRIKES, STRIKES_HOOK, STRIKES_MOTION};
    let armed_hands = || HeldMoveset {
        weapon_class: 0x10,
        ..HeldMoveset::default()
    };
    let mut app = new_app();
    let armed = caster(&mut app, &[STRIKES]);
    app.world_mut().entity_mut(armed).insert(armed_hands());
    {
        let mut caster = app.world_mut().get_mut::<PowerCaster>(armed).unwrap();
        caster.body.focus = 1.0;
    }
    cast(&mut app, armed);
    let body = cast_body(&app, armed);
    assert_eq!(body.cast_motion, Some(STRIKES_MOTION));
    assert_eq!(
        (body.casting, body.cast_hook),
        (STRIKES, Some(STRIKES_HOOK))
    );
    let active = |app: &App, who| app.world().get::<PowerCaster>(who).unwrap().active.len();
    assert_eq!(active(&app, armed), 1);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(cast_body(&app, armed).cast_hook, Some(STRIKES_HOOK));

    let busy = caster(&mut app, &[STRIKES]);
    app.world_mut().entity_mut(busy).insert(armed_hands());
    {
        let mut caster = app.world_mut().get_mut::<PowerCaster>(busy).unwrap();
        caster.body.focus = 1.0;
        caster.body.cast_hook = Some(CastHook { keeps: 0x20 });
    }
    cast(&mut app, busy);
    assert_eq!(app.world().get::<PowerCaster>(busy).unwrap().last_result, 1);
    assert_eq!(active(&app, busy), 0);
    assert_eq!(app.world().get::<PowerFactors>(busy).unwrap().strikes, 0.0);
}

/// A caster whose animator leaves the action its hook saw has the hook
/// asked with event 4, except leaving the walk, the run or an attack.
#[test]
fn leaving_the_cast_action_calls_the_hook() {
    use crate::thaum::casting::CastHook;
    use exa_formats::rsq::{Action, MotionSet};
    let mut app = new_app();
    let action = |name: &str| Action {
        name: name.into(),
        params: [0, 0],
    };
    let set = MotionSet {
        bone_count: 0,
        actions: ["Walk", "Run", "Idle", "Force Raise", "Attack Mid"]
            .into_iter()
            .map(action)
            .collect(),
        movesets: Vec::new(),
    };
    app.insert_resource(Motions::new(
        std::sync::Arc::new(set),
        std::sync::Arc::default(),
        0.0,
    ));
    let mut caster = PowerCaster::new(&[SHELL]);
    caster.body.casting = SHELL;
    caster.body.cast_hook = Some(CastHook { keeps: 0x29 });
    caster.seen_action = Some("Force Raise".into());
    let npc = app
        .world_mut()
        .spawn((caster, Animator::new("Default", "Force Raise")))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        cast_body(&app, npc).cast_hook,
        Some(CastHook { keeps: 0x29 })
    );
    app.world_mut().get_mut::<Animator>(npc).unwrap().action = "Idle".into();
    app.world_mut().run_schedule(FixedUpdate);
    let body = cast_body(&app, npc);
    assert_eq!((body.casting, body.cast_hook), (0, None));

    let mut caster = PowerCaster::new(&[SHELL]);
    caster.body.casting = SHELL;
    caster.body.cast_hook = Some(CastHook::NEVER_KEEPS);
    caster.seen_action = Some("Walk".into());
    let walker = app
        .world_mut()
        .spawn((caster, Animator::new("Default", "Idle")))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(cast_body(&app, walker).casting, SHELL);
}
