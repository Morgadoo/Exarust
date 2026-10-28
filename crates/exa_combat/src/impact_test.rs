//! Limb impacts: the sinking speed, the decay of the impact damage taken,
//! and a ball thrown at a character's limb.
use bevy::ecs::system::RunSystemOnce;
use bevy::math::Affine3A;
use exa_physics::Physics;
use exa_physics::body::{BodyPlugin, PartSetup, step_bodies};

use super::*;

/// A slower height change is taken at once, a faster one blended in at
/// 0.005; rising counts as standing still.
#[test]
fn the_sinking_speed_takes_a_slowdown_at_once() {
    let mut book = ImpactBook::default();
    book.sink(100.0);
    assert_eq!(book.sinking, 0.0);
    book.sink(90.0);
    assert_eq!(book.sinking, (-10.0f64 * 0.005) as f32);
    book.sink(91.0);
    assert_eq!(book.sinking, 0.0);
    book.sinking = -4.0;
    book.sink(90.0);
    assert_eq!(book.sinking, -1.0);
    book.sinking = -0.5;
    book.sink(89.0);
    assert_eq!(book.sinking, (-0.5f32 as f64 * 0.995 - 0.005) as f32);
}

/// Only an overhead swing on a striking part drops a hard impact; anything
/// else draws the clock's spread.
#[test]
fn an_overhead_striking_part_drops_its_hard_impact() {
    assert!(overhead_drop(true, true));
    assert!(!overhead_drop(true, false));
    assert!(!overhead_drop(false, true));
    assert!(!overhead_drop(false, false));
}

/// The impact damage taken falls by the double 0.01 while over it.
#[test]
fn the_impact_damage_taken_recovers() {
    let mut book = ImpactBook {
        taken: 0.025,
        ..ImpactBook::default()
    };
    book.recover();
    assert_eq!(book.taken, (0.025f32 as f64 - 0.01) as f32);
    book.taken = 0.01;
    book.recover();
    assert_eq!(book.taken, 0.01);
}

fn ball(id: u32, bone: usize, at: Vec3) -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone,
        entity: Entity::from_raw_u32(id).unwrap(),
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        spheres: None,
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }])
}

/// Ticks the thrower's hand carries the ball before it lets go. The pair
/// impact reads each part's smoothed velocity, which moves a tenth of the
/// way to the part's motion a tick, so a ball only strikes at its full
/// speed once it has moved at that speed for a while: a velocity given on
/// the tick of the throw would arrive with a third of it.
const CARRY_TICKS: usize = 30;

/// A ball carried at `speed` (m/s) along +x for [`CARRY_TICKS`] ticks
/// before it is let go 0.5 m short of x = 0, at a height of 5 m.
fn thrown_ball(speed: f32) -> Ragdoll {
    let carried = speed / exa_physics::body::BODY_HZ * CARRY_TICKS as f32;
    let mut thrown = ball(2, 0, Vec3::new(-0.5 - carried, 5.0, 0.0));
    thrown.set_velocity(Vec3::X * speed);
    thrown
}

/// One tick of the bodies and the limb impacts; while the hand still
/// carries the ball (`tick` under [`CARRY_TICKS`]) it keeps it at `speed`,
/// and the target is held still so that it has not fallen away when the
/// ball is let go.
fn tick(app: &mut App, target: Entity, thrown: Entity, speed: f32, tick: usize) {
    if tick < CARRY_TICKS {
        let mut body = app.world_mut().get_mut::<Ragdoll>(thrown).unwrap();
        body.set_velocity(Vec3::X * speed);
        let mut body = app.world_mut().get_mut::<Ragdoll>(target).unwrap();
        body.set_velocity(Vec3::ZERO);
    }
    app.world_mut().run_system_once(step_bodies).unwrap();
    app.world_mut().run_system_once(strike_limbs).unwrap();
}

/// A ball thrown at 8 m/s at a character's pelvis strikes it: the new
/// damage raises the impact damage taken (less the tick's 0.01), half of
/// it comes off yellow
/// health and dazes the body, the game hears of it, and the root's clock
/// is set. A ball at 0.5 m/s does nothing.
#[test]
fn a_thrown_ball_hurts_the_limb_it_strikes() {
    let run = |speed: f32| {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BodyPlugin))
            .insert_resource(Physics::default())
            .insert_resource(BodyTicks(100))
            .add_message::<LimbImpact>()
            .add_message::<ImpactSounded>()
            .add_message::<WeaponSounded>()
            .add_message::<CameraHit>()
            .add_message::<Weakened>()
            .add_message::<SecondWindFired>();
        let target = app
            .world_mut()
            .spawn((Fighter::default(), ball(1, 1, Vec3::new(0.0, 5.0, 0.0))))
            .id();
        let other = app.world_mut().spawn(thrown_ball(speed)).id();
        let yellow = app.world().get::<Fighter>(target).unwrap().health.yellow;
        for i in 0..CARRY_TICKS + 60 {
            tick(&mut app, target, other, speed, i);
            let book = app.world().get::<ImpactBook>(target).unwrap().clone();
            if book.taken > 0.0 {
                let f = app.world().get::<Fighter>(target).unwrap();
                let lost = yellow - f.health.yellow;
                let messages = app.world().resource::<Messages<LimbImpact>>();
                let heard: Vec<LimbImpact> =
                    messages.iter_current_update_messages().copied().collect();
                let sounded = app.world().resource::<Messages<ImpactSounded>>();
                let sounded: Vec<ImpactSounded> =
                    sounded.iter_current_update_messages().copied().collect();
                assert_eq!(sounded.len(), 1);
                assert!(!sounded[0].foley);
                let body = app.world().get::<Ragdoll>(target).unwrap();
                return Some((book, lost, heard, body.parts[0].sound_ready_ms, other));
            }
        }
        None
    };
    let (book, lost, heard, clock, other) = run(8.0).expect("the ball never hurt the pelvis");
    assert_eq!(heard.len(), 1);
    let hit = heard[0];
    assert_eq!((hit.bone, hit.other), (1, other));
    // The tick's upkeep follows the impact.
    assert_eq!(book.taken, (f64::from(hit.amount) - 0.01) as f32);
    // Struck once: the counter rose by 2; the part step takes one off only
    // before the next tick's contacts.
    assert_eq!(book.count_of(0), 2);
    assert!(lost > 0.0, "{lost}");
    assert!(clock > 0, "{clock}");
    assert_eq!(run(0.5), None);
}

/// A weapon body struck while its owner's window is open rings its own set
/// and records the overhead's impact strength; the item's clock is stamped
/// with the weapon sound's cooldown.
#[test]
fn a_struck_weapon_body_rings_and_records_its_strength() {
    use crate::swing::flags;

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BodyPlugin))
        .insert_resource(Physics::default())
        .insert_resource(BodyTicks(100))
        .add_message::<LimbImpact>()
        .add_message::<ImpactSounded>()
        .add_message::<WeaponSounded>()
        .add_message::<CameraHit>()
        .add_message::<Weakened>()
        .add_message::<SecondWindFired>();
    let owner = app
        .world_mut()
        .spawn((
            Fighter::default(),
            ball(1, 1, Vec3::new(0.0, 100.0, 0.0)),
            SwingState {
                flags: flags::WINDOW_OPENED,
                ..SwingState::default()
            },
            Attacking::new(AttackMotion::Overhead, 0x10),
        ))
        .id();
    let mut item_body = ball(3, 0, Vec3::new(0.0, 5.0, 0.0));
    item_body.parts[0].state = PartState::LIMB_FRESH
        .with(PartState::HELD)
        .with(PartState::STRIKING);
    item_body.held_by = Some(Entity::from_raw_u32(1).unwrap());
    item_body.wake();
    let item = app
        .world_mut()
        .spawn((
            item_body,
            ImpactBook::default(),
            Blade::new(owner, 0.9, crate::damage::sword_a()),
        ))
        .id();
    let mut thrown_body = thrown_ball(8.0);
    thrown_body.parts[0].state = PartState(0);
    let thrown = app.world_mut().spawn(thrown_body).id();
    let mut heard = Vec::new();
    for i in 0..CARRY_TICKS + 60 {
        tick(&mut app, item, thrown, 8.0, i);
        let messages = app.world().resource::<Messages<WeaponSounded>>();
        heard = messages
            .iter_current_update_messages()
            .cloned()
            .collect::<Vec<_>>();
        if !heard.is_empty() {
            break;
        }
    }
    let sound = heard.first().expect("the weapon body never rang");
    assert_eq!(sound.target, item);
    assert!(sound.volume > 0.0 && sound.volume <= 1.0);
    let swing = app.world().get::<SwingState>(owner).unwrap();
    assert_eq!(swing.flags & 0x6, 0x6);
    assert!(swing.strike_strength > 0);
    let body = app.world().get::<Ragdoll>(item).unwrap();
    assert!(body.parts[0].sound_ready_ms > 100);
}

/// The same ball on a pelvis that strikes in an overhead swing still
/// sounds, but the hard impact is dropped: no damage, no clock and no
/// draw of the clock's spread.
#[test]
fn an_overhead_strikers_hard_impact_sounds_and_is_dropped() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BodyPlugin))
        .insert_resource(Physics::default())
        .insert_resource(BodyTicks(100))
        .add_message::<LimbImpact>()
        .add_message::<ImpactSounded>()
        .add_message::<WeaponSounded>()
        .add_message::<CameraHit>()
        .add_message::<Weakened>()
        .add_message::<SecondWindFired>();
    let mut pelvis = ball(1, 1, Vec3::new(0.0, 5.0, 0.0));
    pelvis.parts[0].state = pelvis.parts[0].state.with(PartState::STRIKING);
    let target = app
        .world_mut()
        .spawn((
            Fighter::default(),
            Attacking::new(AttackMotion::Overhead, 0x10),
            pelvis,
        ))
        .id();
    let thrown = app.world_mut().spawn(thrown_ball(8.0)).id();
    let mut sounded = Vec::new();
    for i in 0..CARRY_TICKS + 60 {
        tick(&mut app, target, thrown, 8.0, i);
        let messages = app.world().resource::<Messages<ImpactSounded>>();
        sounded.extend(messages.iter_current_update_messages().copied());
        if !sounded.is_empty() {
            break;
        }
    }
    assert_eq!(sounded.len(), 1, "the ball never sounded on the pelvis");
    assert!(sounded[0].volume > 0.25, "{}", sounded[0].volume);
    assert_eq!(app.world().get::<ImpactBook>(target).unwrap().taken, 0.0);
    assert_eq!(
        app.world().get::<Ragdoll>(target).unwrap().parts[0].sound_ready_ms,
        0
    );
    let heard = app.world().resource::<Messages<LimbImpact>>();
    assert_eq!(heard.iter_current_update_messages().count(), 0);
}

/// A part's counter rises by 2 to a cap of 40 and falls by 1 down to 1.
#[test]
fn the_recent_impact_counter_rises_and_settles() {
    let mut book = ImpactBook::default();
    assert_eq!(book.count(2), 2);
    assert_eq!(book.count(2), 4);
    for _ in 0..30 {
        book.count(2);
    }
    assert_eq!(book.count_of(2), 40);
    for _ in 0..50 {
        book.settle_counts();
    }
    assert_eq!((book.count_of(2), book.count_of(0)), (1, 0));
    book.counts[2] = 39;
    assert_eq!(book.count(2), 41);
}

/// The limb impact sound's volume is the scaled impact times 0.05; the
/// crowded pitch is half the volume plus 0.6, the plain pitch 0.6 of it
/// plus 0.5. The root's counter picks between them above 2. An impact
/// scaled under 0.5 is silent; down, the impact is only halved, otherwise
/// a third of it (in double).
#[test]
fn a_limb_impact_has_the_originals_volume_and_pitches() {
    // Down: 2 halved is 1, so volume 0.05.
    let down = limb_impact_sound(2.0, true).expect("a half is audible");
    assert_eq!(down.volume, (2.0f64 * 0.5 * 0.05) as f32);
    assert_eq!(down.pitch, (down.volume as f64 * 0.6 + 0.5) as f32);
    assert_eq!(down.crowded_pitch, (down.volume as f64 * 0.5 + 0.6) as f32);
    assert_eq!(down.pitch_for(2), down.pitch);
    assert_eq!(down.pitch_for(3), down.crowded_pitch);
    // Not down: 4 × 0.33 is 1.32, then halved 0.66, so volume 0.033.
    let standing = limb_impact_sound(4.0, false).expect("a third is audible");
    let scaled = (4.0f64 * 0.33) as f32 * 0.5;
    assert_eq!(standing.volume, (f64::from(scaled) * 0.05) as f32);
    // 3 × 0.33 is 0.99, halved 0.495: under the 0.5 floor.
    assert_eq!(limb_impact_sound(3.0, false), None);
    assert_eq!(limb_impact_sound(2.0, true).unwrap().cooldown_ms, 212);
}
