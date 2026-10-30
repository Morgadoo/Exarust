//! Pinned checks of the mechanisms' sound requests, with the original's
//! set names, points, levels and draw order.
use super::*;
use crate::door::{Door, Shape, Tick};

fn record() -> Record {
    Record {
        flags: 0,
        key: 0,
        lock_sound: 1,
        slam_sound: 5,
        hinge_sound: 6,
        unlatch_sound: 0,
    }
}

fn quiet_tick() -> DoorTick {
    DoorTick {
        playing: false,
        volume: 0.003,
        pitch: 0.0,
        drive: None,
        shut: false,
        shut_sound: None,
        unlatched: false,
        unlatch_sound: None,
        knock_sound: None,
        passable: false,
        both_passable: false,
        travel: 0.0,
        opened: None,
        shove: Vec3::ZERO,
    }
}

/// A unit draw sequence for [`Sound::levels`], recording how many it gave.
fn draws(
    values: &[f64],
) -> (
    impl FnMut() -> f64 + '_,
    std::rc::Rc<std::cell::Cell<usize>>,
) {
    let used = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter = used.clone();
    (
        move || {
            let i = counter.get();
            counter.set(i + 1);
            values[i]
        },
        used,
    )
}

/// The unlatch (`opn`) draws its pitch first, `0.8 + 0.3·draw`, then its
/// volume, `0.1 + 0.05·draw`, each formed in double precision.
#[test]
fn the_unlatch_draws_its_pitch_then_its_volume() {
    let opn = Sound {
        volume: 0.1,
        volume_spread: 0.05,
        pitch: 0.8,
        pitch_spread: 0.3,
    };
    let (draw, used) = draws(&[0.5, 0.25]);
    let (volume, pitch) = opn.levels(draw);
    assert_eq!(used.get(), 2);
    assert_eq!(pitch, (0.5_f64 * 0.3 + 0.8) as f32);
    assert_eq!(volume, (0.25_f64 * 0.05 + 0.1) as f32);
    assert_eq!(pitch, 0.95);
    assert_eq!(volume, 0.1125);
    let (draw, _) = draws(&[0.0, 0.0]);
    assert_eq!(opn.levels(draw), (0.1, 0.8));
    // The unlock's `LkO` and the rattle's `LkT`: 0.15 + 0.1·draw, 0.8 +
    // 0.2·draw.
    let (draw, _) = draws(&[0.5, 0.5]);
    assert_eq!(RATTLE.levels(draw), (0.2, 0.9));
}

/// A knock or a slam has no volume share: one draw, for the pitch.
#[test]
fn a_knock_draws_once() {
    let knock = Sound {
        volume: 0.1125,
        volume_spread: 0.0,
        pitch: 0.4,
        pitch_spread: 0.2,
    };
    let (draw, used) = draws(&[0.5, 0.9]);
    let (volume, pitch) = knock.levels(draw);
    assert_eq!(used.get(), 1);
    assert_eq!(volume, 0.1125);
    assert_eq!(pitch, (0.5_f64 * 0.2 + 0.4) as f32);
}

/// The door's sets come from its record's letters: `door` + ('a' +
/// letter) + suffix; the knock prefers the hinged body's own collision
/// set; the creak starts at the body's centre and the one-shots at the
/// handle point.
#[test]
fn a_door_tick_names_its_sets_and_points() {
    let source = Entity::from_raw_u32(7).unwrap();
    let centre = Vec3::new(1.0, 0.0, 0.0);
    let handle = Vec3::new(2.0, 1.0, 0.0);
    let before = Vec3::new(2.0, 1.0, 0.5);
    let knock = Sound {
        volume: 0.5,
        volume_spread: 0.0,
        pitch: 0.4,
        pitch_spread: 0.2,
    };
    let opn = Sound {
        volume: 0.1,
        volume_spread: 0.05,
        pitch: 0.8,
        pitch_spread: 0.3,
    };
    let out = DoorTick {
        playing: true,
        volume: 0.075,
        pitch: 0.722,
        knock_sound: Some(knock),
        shut_sound: Some(knock),
        unlatch_sound: Some(opn),
        opened: Some(opn),
        ..quiet_tick()
    };
    let cues = door_cues(source, &record(), false, &out, centre, handle, before);
    let shot = |set: &str, position, sound, own| MechanismSounded::Shot {
        set: set.to_owned(),
        position,
        sound,
        own_set_of: own,
    };
    assert_eq!(
        cues,
        vec![
            shot("doora_opn", before, opn, None),
            MechanismSounded::Voice {
                source,
                set: "doorg_hng".to_owned(),
                position: centre,
                levels: Some((0.075, 0.722)),
            },
            shot("doorf_slm", handle, knock, Some(source)),
            shot("doorf_slm", handle, knock, None),
            shot("doora_opn", handle, opn, None),
        ]
    );
    // A sliding leaf knocks on its slam set; a quiet tick stops the creak.
    let quiet = DoorTick {
        knock_sound: Some(knock),
        ..quiet_tick()
    };
    let cues = door_cues(source, &record(), true, &quiet, centre, handle, before);
    assert_eq!(
        cues,
        vec![
            MechanismSounded::Voice {
                source,
                set: "doorg_hng".to_owned(),
                position: centre,
                levels: None,
            },
            shot("doorf_slm", handle, knock, None),
        ]
    );
    assert_eq!(
        lock_cue(&record(), UNLOCKED, handle, RATTLE),
        shot("doorb_LkO", handle, RATTLE, None)
    );
    assert_eq!(
        lock_cue(&record(), LOCKED, handle, RATTLE),
        shot("doorb_LkT", handle, RATTLE, None)
    );
}

/// A pulled hinge's creak reaches the request with the step's levels: a
/// squared handle speed of 4 (speed 2) is volume 0.075·√2 and pitch
/// 0.022·2 + 0.7.
#[test]
fn a_moving_hinge_sends_its_creak_levels() {
    let shape = Shape {
        sliding: false,
        travel_link: 0,
        sign: 1.0,
        radius: 100.0,
        half_swing: 0.5,
        stop_slack: 50.0,
        axis: Vec3::Y,
    };
    let mut door = Door::new(record(), shape, 0, -0.5, 0.0);
    let out = door.step(Tick::new(100, -0.5, 4.0, 10.0), None);
    let source = Entity::from_raw_u32(3).unwrap();
    let cues = door_cues(source, &record(), false, &out, Vec3::ZERO, Vec3::X, Vec3::X);
    let MechanismSounded::Voice { levels, .. } = &cues[0] else {
        panic!("{cues:?}");
    };
    let (volume, pitch) = levels.unwrap();
    assert_eq!(volume, (f64::from(2.0_f32.sqrt()) * 0.075) as f32);
    assert_eq!(pitch, (2.0_f64 * 0.022 + 0.7) as f32);
}

/// A lever sends its creak (`leverb_mov`) every tick and its tick loop's
/// cue: a start when the return is armed, the fade's volume while it
/// plays, a free when it ends.
#[test]
fn a_lever_sends_its_creak_and_its_tick_loop() {
    let source = Entity::from_raw_u32(9).unwrap();
    let centre = Vec3::new(0.0, 1.0, 0.0);
    let shape = lever::Shape {
        radius: 30.0,
        half_swing: 0.5,
        detent0: -0.5,
        step: 1.0,
        axis: Vec3::Y,
    };
    let mut lever = Lever::new(shape, -0.5);
    let mut out = lever.step(-0.5, Vec3::ZERO, 0);
    out.playing = true;
    out.volume = 0.2;
    out.pitch = 0.9;
    out.arm_return = true;
    lever.arm(0, 5_000);
    let cues = lever_cues(source, centre, &out, false, &lever);
    assert_eq!(
        cues,
        vec![
            MechanismSounded::Voice {
                source,
                set: "leverb_mov".to_owned(),
                position: centre,
                levels: Some((0.2, 0.9)),
            },
            MechanismSounded::TickLoop {
                source,
                position: centre,
                cue: TickCue::Start,
            },
        ]
    );
    out.arm_return = false;
    out.playing = false;
    lever.tick_volume = 0.09;
    let cues = lever_cues(source, centre, &out, true, &lever);
    assert_eq!(
        cues[0],
        MechanismSounded::Voice {
            source,
            set: "leverb_mov".to_owned(),
            position: centre,
            levels: None,
        }
    );
    assert_eq!(
        cues[1],
        MechanismSounded::TickLoop {
            source,
            position: centre,
            cue: TickCue::Gain(0.09),
        }
    );
    lever.tick_playing = false;
    lever.tick_volume = 0.0;
    let cues = lever_cues(source, centre, &out, true, &lever);
    assert_eq!(
        cues[1],
        MechanismSounded::TickLoop {
            source,
            position: centre,
            cue: TickCue::Free,
        }
    );
    // Nothing held, nothing to say about the loop.
    assert_eq!(lever_cues(source, centre, &out, false, &lever).len(), 1);
}

/// A push surface grinds on `Stnb_mov` and clicks once on `Stnb_fit`, at
/// volume 0.3 and pitch 0.9 plus 0.2 of a draw.
#[test]
fn a_plate_sends_its_grind_and_its_click() {
    let source = Entity::from_raw_u32(11).unwrap();
    let centre = Vec3::new(3.0, 0.0, 1.0);
    let record = plate::Record {
        kind: 1,
        flags: 0,
        target: 5,
        second: 0,
    };
    let mut surface = plate::Plate::new(&record, 0);
    surface.pushed = true;
    let mut travels = [0.69];
    let out = plate::plate_step(&mut surface, &mut travels);
    assert!(out.click.is_some(), "{out:?}");
    let cues = plate_cues(source, centre, &out, &surface);
    let MechanismSounded::Voice { set, levels, .. } = &cues[0] else {
        panic!("{cues:?}");
    };
    assert_eq!(set, "Stnb_mov");
    assert_eq!(*levels, Some((out.volume, out.pitch)));
    let MechanismSounded::Shot {
        set,
        sound,
        position,
        ..
    } = &cues[1]
    else {
        panic!("{cues:?}");
    };
    assert_eq!((set.as_str(), *position), ("Stnb_fit", centre));
    let (draw, used) = draws(&[0.5]);
    assert_eq!(sound.levels(draw), (0.3, (0.5_f64 * 0.2 + 0.9) as f32));
    assert_eq!(used.get(), 1);
}
