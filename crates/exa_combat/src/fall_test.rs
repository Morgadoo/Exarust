//! Falling to death: the scream, the 25 m rule, the timelines and the kill
//! plane, pinned to the original's numbers (`docs/formats/RSQ.md`,
//! Falling to death).
use super::*;

/// The game clock's milliseconds a body tick.
const MS: u64 = 12;
/// Ground a Height Query that meets nothing gives (m).
const NO_GROUND: f32 = -100.0;

/// A body sinking at `cm_per_tick`, its pelvis at 30 m over nothing.
fn over_nothing(cm_per_tick: f32, feet: &[Option<f32>]) -> FallReading<'_> {
    FallReading {
        sinking: cm_per_tick / 100.0,
        pelvis: 30.0,
        ground: Some(NO_GROUND),
        feet,
    }
}

const NO_FEET_GROUND: [Option<f32>; 2] = [Some(NO_GROUND); 2];

/// At 5 cm a tick a body is steady; past it, it screams once and keeps
/// falling; slower again it is steady and screams the next time.
#[test]
fn it_screams_once_past_5_cm_a_tick() {
    let mut fall = FallDeath::default();
    // A floor 2 m under it: it falls but is not lost.
    let feet = [Some(28.0); 2];
    let near = |v| FallReading {
        ground: Some(28.0),
        ..over_nothing(v, &feet)
    };
    assert_eq!(
        fall.check(&near(-5.0), false, false, 0),
        FallStep::default()
    );
    assert_eq!(fall.stage, FallStage::Steady);
    assert_eq!(
        (
            FallStage::Steady.death_stage(),
            FallStage::Falling.death_stage(),
            FallStage::Lost.death_stage(),
            FallStage::Fading.death_stage(),
            FallStage::Gone.death_stage(),
        ),
        (0, 1, 2, 3, 4)
    );
    let step = fall.check(&near(-5.01), false, false, 12);
    assert!(step.scream && !step.lost);
    assert_eq!(fall.stage, FallStage::Falling);
    assert!(!fall.check(&near(-8.0), false, false, 24).scream);
    assert_eq!(
        fall.check(&near(-4.0), false, false, 36),
        FallStep::default()
    );
    assert_eq!(fall.stage, FallStage::Steady);
    assert!(fall.check(&near(-6.0), false, false, 48).scream);
    // A NaN is not below: steady.
    assert_eq!(
        fall.check(&near(f32::NAN), false, false, 60),
        FallStep::default()
    );
    assert_eq!(fall.stage, FallStage::Steady);
}

/// A dead body never screams, but its fall goes on.
#[test]
fn the_dead_do_not_scream() {
    let mut fall = FallDeath::default();
    let step = fall.check(&over_nothing(-6.0, &NO_FEET_GROUND), true, false, 0);
    assert!(!step.scream && step.lost);
}

/// Lost when the pelvis is 25 m or more over the ground under the body and
/// under each foot: the first tick it falls over nothing, it screams and is
/// lost at once, its time stamped and out of the world.
#[test]
fn twenty_five_metres_over_every_ground() {
    let mut fall = FallDeath::default();
    let step = fall.check(&over_nothing(-6.0, &NO_FEET_GROUND), false, false, 1200);
    assert!(step.scream && step.lost);
    assert_eq!(
        fall,
        FallDeath {
            stage: FallStage::Lost,
            since: 1200,
            out: true
        }
    );
    let at = |ground: Option<f32>, feet: &[Option<f32>]| {
        FallReading {
            sinking: -0.06,
            pelvis: 30.0,
            ground,
            feet,
        }
        .lethal()
    };
    // 25 m exactly is enough; 24.99 m is not.
    assert!(at(Some(5.0), &[Some(5.0); 2]));
    assert!(!at(Some(5.01), &[Some(5.0); 2]));
    // One foot over ground 2 m down, or out of reach: not lost.
    assert!(!at(Some(NO_GROUND), &[Some(NO_GROUND), Some(28.0)]));
    assert!(!at(Some(NO_GROUND), &[Some(NO_GROUND), None]));
    // Supported (no ground measured under the body): never lost.
    assert!(!at(None, &NO_FEET_GROUND));
    // A body without feet needs only the ground under it.
    assert!(at(Some(NO_GROUND), &[]));
}

/// Ticks of 12 ms after the one it was lost on until `fall` does what
/// `wanted` picks.
fn ticks_until(fall: &mut FallDeath, player: bool, wanted: fn(&FallStep) -> bool) -> u64 {
    let reading = over_nothing(-8.0, &NO_FEET_GROUND);
    let lost = fall.since;
    (1..1000)
        .find(|n| wanted(&fall.check(&reading, false, player, lost + n * MS)))
        .unwrap()
}

/// The player hears the pit on the 84th tick after it was lost (1,008 ms:
/// more than 1 s) and dies on the 334th (4,008 ms); any other character
/// dies on the 251st (3,012 ms) and hears nothing.
#[test]
fn the_timelines() {
    let lost = |player| {
        let mut fall = FallDeath::default();
        fall.check(&over_nothing(-6.0, &NO_FEET_GROUND), false, player, 1200);
        fall
    };
    let mut player = lost(true);
    assert_eq!(ticks_until(&mut player, true, |s| s.pit), 84);
    assert_eq!(player.stage, FallStage::Fading);
    let mut player = lost(true);
    assert_eq!(ticks_until(&mut player, true, |s| s.dies), 334);
    assert_eq!(player.stage, FallStage::Gone);
    let mut other = lost(false);
    assert_eq!(ticks_until(&mut other, false, |s| s.dies), 251);
    let mut other = lost(false);
    assert!((1..=251).all(|n| {
        !other
            .check(&over_nothing(-8.0, &[]), false, false, 1200 + n * MS)
            .pit
    }));
    // Once gone nothing more happens, however it moves.
    assert_eq!(
        other.check(&over_nothing(0.0, &[]), false, false, 99_999),
        FallStep::default()
    );
    assert_eq!(other.stage, FallStage::Gone);
}

/// Once lost, slowing down (landing on something) does not save it: only
/// the clock decides, while the check runs.
#[test]
fn lost_is_lost() {
    let mut fall = FallDeath::default();
    fall.check(&over_nothing(-6.0, &NO_FEET_GROUND), false, false, 0);
    let landed = FallReading {
        sinking: 0.0,
        pelvis: 1.0,
        ground: None,
        feet: &[Some(0.0); 2],
    };
    assert!(!fall.check(&landed, false, false, 3000).dies);
    assert!(fall.check(&landed, false, false, 3012).dies);
}

/// The kill plane takes a body once; a body already lost to its fall has
/// left the world and is not taken again.
#[test]
fn the_kill_plane_takes_a_body_once() {
    let mut fall = FallDeath::default();
    assert!(fall.fall_out());
    assert!(!fall.fall_out());
    let mut lost = FallDeath::default();
    lost.check(&over_nothing(-6.0, &NO_FEET_GROUND), false, false, 0);
    assert!(!lost.fall_out());
}

/// The scream's draws: the first gives the pitch 0.95–1.05, the second the
/// gain 0.2–0.35.
#[test]
fn the_screams_gain_and_pitch() {
    assert_eq!(scream_voice(0.0, 0.0), (0.2, 0.95));
    assert_eq!(scream_voice(0.5, 0.0), (0.2, 1.0));
    assert_eq!(scream_voice(0.0, 1.0), (0.35, 0.95));
    assert_eq!(scream_voice(1.0, 0.5), (0.275, 1.05));
}
