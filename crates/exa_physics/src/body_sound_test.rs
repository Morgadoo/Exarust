//! Pinned checks of the plain body's impact sound and scrape sources.
use bevy::math::Affine3A;
use bevy::prelude::Entity;

use super::*;

/// A 0.5 impact with the builders' 0.3 mobility: the first sound, its exact
/// volume, pitch and cooldown, and the generator's draw.
#[test]
fn a_plain_impact_sounds_at_the_originals_volume_and_pitch() {
    let mut state = ImpactState::new(true);
    let mut roll = 1;
    let sound = impact_sound(&mut state, 50.0, 0.3, 1000, &mut roll).unwrap();
    assert_eq!(sound.volume, 0.75);
    assert!((sound.pitch - 0.906).abs() < 1e-6, "{}", sound.pitch);
    // The draw is the third of a hundred, and round(0.5 · 170) is 85.
    assert_eq!(roll, 0x0808_8406);
    assert_eq!(sound.cooldown_ms, 1138);
    assert_eq!(state.cooldown_ms, 1138);
    assert_eq!(state.threshold, 100.0);
}

/// Below the 0.1 mobility the volume is scaled too: 0.05 gives
/// `(0.5·0.8 + 0.2)·0.75 = 0.45` and `(0.5·0.7 + 0.3)·1 = 0.65`.
#[test]
fn a_light_part_lowers_its_volume_as_well_as_its_pitch() {
    let mut state = ImpactState::new(true);
    let mut roll = 1;
    let sound = impact_sound(&mut state, 50.0, 0.05, 0, &mut roll).unwrap();
    assert!((sound.volume - 0.45).abs() < 1e-6, "{}", sound.volume);
    assert!((sound.pitch - 0.65).abs() < 1e-6, "{}", sound.pitch);
}

/// A NaN mobility follows the heavy branch, as the unordered compare does.
#[test]
fn a_nan_mobility_takes_the_heavy_branch() {
    let mut state = ImpactState::new(true);
    let mut roll = 1;
    let sound = impact_sound(&mut state, 50.0, f32::NAN, 0, &mut roll).unwrap();
    assert_eq!(sound.volume, 0.75);
    assert!(sound.pitch.is_nan());
}

/// The cooldown gate: a quiet replay waits, a strictly louder one sounds,
/// and one exactly at twice the last strength does not.
#[test]
fn a_repeat_sounds_only_when_it_is_strictly_louder() {
    let mut state = ImpactState::new(true);
    let mut roll = 1;
    let first = impact_sound(&mut state, 10.0, 0.3, 1000, &mut roll).unwrap();
    assert_eq!(state.threshold, 20.0);
    assert_eq!(
        impact_sound(&mut state, 10.0, 0.3, first.cooldown_ms - 1, &mut roll),
        None
    );
    assert_eq!(
        impact_sound(&mut state, 20.0, 0.3, first.cooldown_ms - 1, &mut roll),
        None
    );
    let louder = impact_sound(&mut state, 21.0, 0.3, first.cooldown_ms - 1, &mut roll).unwrap();
    assert!(louder.volume > 0.0);
    // Past the cooldown even a quiet impact sounds again.
    assert!(impact_sound(&mut state, 10.0, 0.3, louder.cooldown_ms + 1, &mut roll).is_some());
}

/// A strength at or under 0.5 is silent, and a part without a collision
/// sound stays silent without advancing the generator.
#[test]
fn a_weak_or_unsounded_impact_is_silent() {
    let mut state = ImpactState::new(true);
    let mut roll = 1;
    assert_eq!(impact_sound(&mut state, 0.5, 0.3, 0, &mut roll), None);
    assert_eq!(roll, 1);
    let mut state = ImpactState::new(false);
    assert_eq!(impact_sound(&mut state, 50.0, 0.3, 0, &mut roll), None);
    assert_eq!(state.cooldown_ms, u32::MAX);
    assert_eq!(roll, 1);
}

/// The first 1 cm move of a scrape: the move is 1 cm², `(√1·2 + 1)·0.05 =
/// 0.15`, the speed a fifth of it, 0.03; its volume is 0.06, its pitch
/// `1.2 · (0.1·0.03 + 0.9)`, and it starts.
#[test]
fn a_scrape_starts_on_its_first_move() {
    let mut source = ScrapeSource::new(0, Vec3::ZERO, 2.0, 1.2, Vec3::ZERO);
    let event = source.step(Vec3::new(0.01, 0.0, 0.0), Vec3::new(1.0, 2.0, 3.0));
    assert!((source.speed - 0.03).abs() < 1e-7, "{}", source.speed);
    assert!((event.volume - 0.06).abs() < 1e-7, "{}", event.volume);
    assert!((event.pitch - 1.0836).abs() < 1e-6, "{}", event.pitch);
    assert!(event.started && event.playing && !event.stopped);
    assert_eq!(event.point, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(source.last, Vec3::new(0.01, 0.0, 0.0));
}

/// The move is squared, not a length: 4 cm is 16 cm², which the curve
/// takes to 1.2 and clamps at one (a length would have given 0.5).
#[test]
fn a_scrape_measures_its_move_squared() {
    let mut source = ScrapeSource::new(0, Vec3::ZERO, 1.0, 1.0, Vec3::ZERO);
    source.step(Vec3::new(0.04, 0.0, 0.0), Vec3::ZERO);
    assert_eq!(source.speed, 0.2);
}

/// A move of at most 0.01 cm² (here 0.05 cm, 0.0025 cm²) skips the curve
/// and feeds its squared move straight into the smoothing.
#[test]
fn a_tiny_move_skips_the_curve() {
    let mut source = ScrapeSource::new(0, Vec3::ZERO, 1.0, 1.0, Vec3::ZERO);
    let event = source.step(Vec3::new(0.0005, 0.0, 0.0), Vec3::ZERO);
    assert!((source.speed - 0.0005).abs() < 1e-8, "{}", source.speed);
    assert!(!event.playing && !event.started);
}

/// A NaN move fails the speed compare and stops a playing source.
#[test]
fn a_nan_speed_stops_the_source() {
    let mut source = ScrapeSource::new(0, Vec3::ZERO, 1.0, 1.0, Vec3::ZERO);
    assert!(source.step(Vec3::new(0.01, 0.0, 0.0), Vec3::ZERO).started);
    let event = source.step(Vec3::NAN, Vec3::ZERO);
    assert!(event.stopped && !event.playing);
}

/// Standing still decays the speed by four fifths a tick: 0.03, 0.024,
/// 0.0192, which is under 0.02 and stops the source.
#[test]
fn a_scrape_stops_once_its_smoothed_speed_falls_under_the_floor() {
    let mut source = ScrapeSource::new(0, Vec3::ZERO, 2.0, 1.2, Vec3::ZERO);
    let at = Vec3::new(0.01, 0.0, 0.0);
    assert!(source.step(at, Vec3::ZERO).started);
    let speeds = [0.024, 0.0192];
    for (i, speed) in speeds.iter().enumerate() {
        let event = source.step(at, Vec3::ZERO);
        assert!(
            (source.speed - speed).abs() < 1e-7,
            "tick {i}: {}",
            source.speed
        );
        if i == speeds.len() - 1 {
            assert!(event.stopped && !event.playing);
        } else {
            assert!(event.playing && !event.stopped);
        }
    }
}

/// A move over one clamps its smoothed distance at one: the speed then
/// rises by a fifth of one.
#[test]
fn a_long_move_clamps_the_smoothed_distance() {
    let mut source = ScrapeSource::new(0, Vec3::ZERO, 1.0, 1.0, Vec3::ZERO);
    let event = source.step(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);
    assert_eq!(source.speed, 0.2);
    assert!((event.volume - 0.2).abs() < 1e-6);
    assert!((event.pitch - 0.92).abs() < 1e-6);
}

/// A body's source starts where the body has its point, so no false move is
/// recorded on the first tick, and the post-step reports it.
#[test]
fn a_bodys_source_starts_where_its_point_is() {
    let body = |world: Affine3A| {
        super::super::Ragdoll::new(&[super::super::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }])
    };
    let mut body = body(Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0)));
    body.add_scrape(Vec3::new(0.5, 0.0, 0.0), 2.0, 1.2);
    assert_eq!(body.scrapes().len(), 1);
    assert_eq!(body.scrapes()[0].last, Vec3::new(1.5, 2.0, 3.0));
    body.post_step();
    let events = body.scrape_events();
    assert_eq!(events.len(), 1);
    assert!(!events[0].playing && !events[0].started);
}
