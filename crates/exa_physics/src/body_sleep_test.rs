//! Sleep of the physical body: the original's countdown, held at 256 while
//! a body lives and counted down by the speed of its fastest part once it
//! is limp, the sleeping and deep-sleep ticks, and what wakes a body.
use super::floor_test::{app, floor};
use super::*;

const NONE: &[[Vec3; 3]] = &[];

/// A lone limp part (a forearm-sized ball of radius 6 cm) with its centre
/// at `at`.
fn ball(at: Vec3) -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone: 11,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }])
}

/// Ticks `body` in empty space, giving it `speed` (m/s, sideways) before
/// every tick, so each tick measures exactly that motion; returns the tick
/// it fell asleep in, if it did within `ticks`.
fn drift(body: &mut Ragdoll, speed: f32, ticks: u32) -> Option<u32> {
    for tick in 1..=ticks {
        body.set_velocity(Vec3::X * speed);
        body.tick(NONE);
        if body.asleep() {
            return Some(tick);
        }
    }
    None
}

/// The bands, strict as the original's: over 40/83 cm in a tick is fast,
/// over 20/83 cm slow. The countdown: fast wakes it to 256; resting takes
/// two a tick, so a body at rest sleeps 128 ticks (1.54 s) after its last
/// fast tick; slow lifts it to 32 from below and takes one a tick above,
/// two at 32, so it is held at 30–32; it never falls below 1.
#[test]
fn the_countdown_follows_the_fastest_part() {
    for (moved, want) in [
        (0.49, Speed::Fast),
        (40.0 / BODY_HZ, Speed::Slow),
        (0.25, Speed::Slow),
        (20.0 / BODY_HZ, Speed::Resting),
        (0.0, Speed::Resting),
    ] {
        assert_eq!(speed_band(moved), want, "{moved} cm in a tick");
    }
    let mut c = WAKE_COUNT;
    let mut ticks = 0;
    while c > SLEEP_COUNT {
        c = count_down(c, Speed::Resting);
        ticks += 1;
    }
    assert_eq!((ticks, c), (128, 1));
    assert_eq!(count_down(3, Speed::Resting), 1);
    assert_eq!(count_down(1, Speed::Resting), 1);
    // Slow: down by one to 32, then 30, 32, 30, ...
    let mut c = WAKE_COUNT;
    let mut seen = Vec::new();
    for _ in 0..300 {
        c = count_down(c, Speed::Slow);
        seen.push(c);
    }
    assert_eq!(seen[223], 32, "one a tick from 256 to 32");
    assert!(
        seen[224..].chunks(2).all(|p| p == [30, 32]),
        "held at 30–32"
    );
    assert_eq!(count_down(1, Speed::Slow), 32);
    assert_eq!(count_down(30, Speed::Fast), 256);
    assert_eq!(count_down(256, Speed::Fast), 256);
}

/// A limp body moving under 20 cm/s falls asleep in the post-step of its
/// 128th tick; one held between 20 and 40 cm/s never does; one over
/// 40 cm/s keeps its countdown at 256 and starts the 128 ticks afresh.
#[test]
fn a_limp_body_sleeps_by_its_speed() {
    let mut body = ball(Vec3::ZERO);
    assert_eq!(body.countdown(), WAKE_COUNT);
    assert_eq!(drift(&mut body, 0.1, 400), Some(128));

    let mut body = ball(Vec3::ZERO);
    assert_eq!(drift(&mut body, 0.3, 2000), None);
    assert!(
        (30..=32).contains(&body.countdown()),
        "{}",
        body.countdown()
    );

    let mut body = ball(Vec3::ZERO);
    assert_eq!(drift(&mut body, 0.1, 100), None);
    assert_eq!(drift(&mut body, 0.6, 50), None);
    assert_eq!(body.countdown(), WAKE_COUNT);
    assert_eq!(drift(&mut body, 0.0, 400), Some(128));
}

/// Falling asleep, a body's velocity is taken away in its first sleeping
/// tick, which leaves it in deep sleep; from then on it is not moved at
/// all, whatever ticks run.
#[test]
fn a_sleeping_body_keeps_its_pose_bit_exact() {
    let mut body = ball(Vec3::new(3.0, 1.0, -2.0));
    drift(&mut body, 0.15, 400).expect("asleep");
    assert_eq!((body.sleep, body.countdown()), (Sleep::Asleep, 0));
    let pose: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    assert_ne!(body.parts[0].previous, pose[0], "still moving as it sleeps");
    body.tick(NONE);
    assert_eq!(body.sleep, Sleep::Deep);
    assert_eq!(body.parts[0].previous, pose[0], "velocity kept");
    for _ in 0..500 {
        body.tick(NONE);
    }
    assert_eq!(body.sleep, Sleep::Deep);
    assert_eq!(body.parts[0].positions, pose[0]);
    assert_eq!(body.parts[0].previous, pose[0]);
}

/// A blow wakes a deep sleeper to 256 and moves it from rest.
#[test]
fn a_shove_wakes_a_sleeper_and_it_moves() {
    let mut body = ball(Vec3::ZERO);
    drift(&mut body, 0.0, 400).expect("asleep");
    body.tick(NONE);
    assert_eq!(body.sleep, Sleep::Deep);
    let x = body.parts[0].transform().translation.x;
    body.shove(11, Vec3::X * 1.2, 0.0);
    assert_eq!((body.sleep, body.countdown()), (Sleep::Awake, WAKE_COUNT));
    body.tick(NONE);
    let moved = body.parts[0].transform().translation.x - x;
    assert!(
        (moved - 1.2 / BODY_HZ).abs() < 1e-3,
        "moved {:.4} m in its first tick",
        moved
    );
}

/// A blow sent to the game's solver (as a hit sends one) wakes a sleeping
/// body in the next tick.
#[test]
fn a_queued_blow_wakes_a_sleeping_body() {
    let mut body = ball(Vec3::new(0.0, 5.0, 0.0));
    drift(&mut body, 0.0, 400).expect("asleep");
    let mut app = app(floor(Vec3::ZERO), 1.0 / BODY_HZ);
    let entity = app.world_mut().spawn(body).id();
    for _ in 0..20 {
        app.update();
    }
    let lying = app.world().get::<Ragdoll>(entity).unwrap();
    assert_eq!(lying.sleep, Sleep::Deep, "asleep in the air");
    let x = lying.parts[0].transform().translation.x;
    app.world_mut().write_message(Shove {
        body: entity,
        bone: 11,
        velocity: Vec3::X,
        daze: 0.0,
    });
    for _ in 0..5 {
        app.update();
    }
    let woken = app.world().get::<Ragdoll>(entity).unwrap();
    assert!(!woken.asleep());
    assert!(woken.parts[0].transform().translation.x - x > 0.02);
}

/// A living body never sleeps, standing still or not; once it goes limp it
/// is awake with the living countdown and sleeps by its speed.
#[test]
fn living_bodies_never_sleep_until_they_go_limp() {
    let mut body = ball(Vec3::ZERO).with_drive();
    assert_eq!(drift(&mut body, 0.0, 1000), None);
    assert_eq!(body.countdown(), WAKE_COUNT);
    body.go_limp();
    assert_eq!((body.sleep, body.countdown()), (Sleep::Awake, WAKE_COUNT));
    assert_eq!(drift(&mut body, 0.0, 400), Some(128));
}

/// A sleeper keeps its level contacts, unused. An explicit wake keeps them
/// (where it lies they still hold); a touch from another body, as a blow
/// or a contact between bodies, raises the lower countdown to the other's
/// and drops them.
#[test]
fn a_touch_wakes_the_lower_countdown_afresh() {
    let level = super::tests::floor_quad(0.0, 5.0);
    let mut sleeper = ball(Vec3::new(0.0, 0.06, 0.0));
    let mut ticks = 0;
    while !sleeper.asleep() && ticks < 400 {
        sleeper.tick(level.as_slice());
        ticks += 1;
    }
    // On the floor it never moved 20 cm/s, so 128 ticks.
    assert_eq!(ticks, 128);
    sleeper.tick(level.as_slice());
    assert!(!sleeper.parts[0].contacts.is_empty(), "no contact kept");

    let mut woken = sleeper.clone();
    woken.wake_to(WAKE_COUNT);
    assert!(!woken.asleep() && !woken.parts[0].contacts.is_empty());

    let mut living = ball(Vec3::new(1.0, 0.06, 0.0)).with_drive();
    let mut touched = sleeper.clone();
    equalise_sleep(&mut living, &mut touched);
    assert_eq!(
        (touched.sleep, touched.countdown()),
        (Sleep::Awake, WAKE_COUNT)
    );
    assert!(touched.parts[0].contacts.is_empty());
    assert_eq!(living.countdown(), WAKE_COUNT);
    // A body counting down is raised to a higher one, not lowered.
    let mut a = ball(Vec3::ZERO);
    drift(&mut a, 0.0, 50);
    let mut b = sleeper.clone();
    equalise_sleep(&mut a, &mut b);
    assert_eq!(
        (a.countdown(), b.countdown()),
        (WAKE_COUNT - 100, WAKE_COUNT - 100)
    );
    assert!(!b.asleep());
}

/// The summed limb motion the posture stage reads: each part's farthest
/// particle over the last tick, in cm, summed over the parts. Two parts
/// drifting 1 cm a tick sum 2 cm; one particle of the second moved 2 cm
/// up as well makes that part's √5 = 2.236 cm, 3.236 in all.
#[test]
fn exertion_sums_each_parts_farthest_particle() {
    let part = |parent: Option<usize>, at: Vec3| PartSetup {
        bone: if parent.is_some() { 12 } else { 11 },
        entity: Entity::PLACEHOLDER,
        parent,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    };
    let mut body = Ragdoll::new(&[part(None, Vec3::ZERO), part(Some(0), Vec3::Y * 0.3)]);
    assert_eq!(body.exertion(), 0.0);
    body.set_velocity(Vec3::X * 0.01 * BODY_HZ);
    assert!((body.exertion() - 2.0).abs() < 1e-4, "{}", body.exertion());
    body.parts[1].previous[2] -= Vec3::Y * 0.02;
    assert!(
        (body.exertion() - (1.0 + 5f32.sqrt())).abs() < 1e-4,
        "{}",
        body.exertion()
    );
}
