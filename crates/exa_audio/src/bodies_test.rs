//! Pinned checks of the plain bodies' impact and scrape sound drivers.
use bevy::math::Affine3A;
use exa_physics::body::PartSetup;

use super::*;

fn touch(a: (Entity, usize), b: (Entity, usize), struck: [Option<f32>; 2]) -> Touch {
    Touch {
        a,
        b,
        met: true,
        hits: 1,
        point: Vec3::ZERO,
        push: Vec3::ZERO,
        deepest: 0.0,
        travel: Vec3::ZERO,
        travel2: 0.0,
        struck,
        hit_event: false,
    }
}

fn body(parts: usize) -> Ragdoll {
    let setup: Vec<PartSetup> = (0..parts)
        .map(|i| PartSetup {
            bone: i + 1,
            entity: Entity::PLACEHOLDER,
            parent: i.checked_sub(1),
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
            spheres: None,
        })
        .collect();
    let mut body = Ragdoll::new(&setup);
    for p in &mut body.parts {
        p.pairing.mobility = 0.3;
    }
    body
}

/// A body takes the impacts of the pairs it is on, on either side, in
/// pair order, and none of a pair that did not strike.
#[test]
fn a_body_takes_its_pairs_impacts_in_order() {
    let (me, other) = (
        Entity::from_raw_u32(1).unwrap(),
        Entity::from_raw_u32(2).unwrap(),
    );
    let touches = [
        touch((me, 1), (other, 0), [Some(3.0), Some(4.0)]),
        touch((other, 2), (me, 0), [Some(5.0), Some(6.0)]),
        touch((me, 0), (other, 0), [None, None]),
    ];
    assert_eq!(struck_parts(&touches, me), [(1, 3.0), (0, 6.0)]);
    assert_eq!(struck_parts(&touches, other), [(0, 4.0), (2, 5.0)]);
}

/// Two impacts on one part in a tick: the first sounds and sets the clock,
/// the second (weaker than twice the first) waits; a part without a set
/// stays silent and parks its clock.
#[test]
fn impacts_ring_through_each_parts_clock() {
    let mut b = body(2);
    let mut roll = 1;
    let rung = ring_impacts(
        &mut b,
        &[(0, 50.0), (0, 60.0), (1, 50.0)],
        |p| p == 0,
        1000,
        &mut roll,
    );
    assert_eq!(rung.len(), 1);
    assert_eq!(rung[0].0, 0);
    assert_eq!(rung[0].1.volume, 0.75);
    assert_eq!(b.parts[0].sound_ready_ms, 1138);
    assert_eq!(b.parts[0].last_impact, 100.0);
    assert_eq!(b.parts[1].sound_ready_ms, u32::MAX);
    // An index past the body is skipped.
    assert!(ring_impacts(&mut b, &[(9, 50.0)], |_| true, 2000, &mut roll).is_empty());
}

fn event(playing: bool) -> ScrapeEvent {
    ScrapeEvent {
        started: playing,
        stopped: !playing,
        playing,
        volume: 0.06,
        pitch: 1.0836,
        point: Vec3::new(1.0, 2.0, 3.0),
    }
}

/// A scrape loop starts over the floor, follows the event's volume, pitch
/// and point, and is asked to stop under it.
#[test]
fn a_scrape_loop_starts_follows_and_stops() {
    let mut env = AudioEnvironment::new();
    let mut bank = SoundBank::default();
    let set = bank.insert_test_set(vec![1, 2]);
    let handle = env.new_handle();
    drive_scrape(&mut env, &mut bank, handle, set, &event(true));
    let id = env.handle_voice(handle).unwrap();
    let voice = env.voice(id).unwrap();
    assert_ne!(voice.flags & voice_flags::LOOPING, 0);
    assert_eq!((voice.gain, voice.pitch), (0.06, 1.0836));
    assert_eq!(voice.position, Vec3::new(1.0, 2.0, 3.0));
    // Still playing: the same voice, new values.
    let mut louder = event(true);
    louder.volume = 0.2;
    drive_scrape(&mut env, &mut bank, handle, set, &louder);
    assert_eq!(env.handle_voice(handle), Some(id));
    assert_eq!(env.voice(id).unwrap().gain, 0.2);
    // Under the floor: the stop bit.
    drive_scrape(&mut env, &mut bank, handle, set, &event(false));
    assert_ne!(env.voice(id).unwrap().flags & voice_flags::STOP, 0);
}

/// A source with an empty set never binds a voice.
#[test]
fn a_scrape_without_a_set_stays_silent() {
    let mut env = AudioEnvironment::new();
    let mut bank = SoundBank::default();
    let handle = env.new_handle();
    drive_scrape(
        &mut env,
        &mut bank,
        handle,
        SoundBank::EMPTY_SET,
        &event(true),
    );
    assert!(!env.handle_is_bound(handle));
}

/// The shipped wood and metal collision sets the props' materials name
/// have their numbered variants (skipped without `EXANIMA_DIR`).
#[test]
fn the_shipped_collision_sets_have_variants() {
    use exa_assets::{GameData, PackKind, locate_install};
    let Some(dir) = locate_install(None) else {
        return;
    };
    let game = GameData::open(&dir).unwrap();
    let sounds = game.pack(PackKind::Sound).unwrap();
    for set in ["wdlcol", "mtlcol"] {
        assert!(sounds.get(&format!("{set}01")).is_some(), "{set}");
    }
}
