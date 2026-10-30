//! Pinned checks of the mechanisms' voice, one-shot and tick-loop drivers.
use super::*;

/// A creaking body's voice restarts on a variant while it is not playing,
/// only takes new levels while it plays (its point stays), and a quiet
/// tick zeroes, stops and frees it.
#[test]
fn a_creak_voice_restarts_follows_and_frees() {
    let mut env = AudioEnvironment::new();
    let mut bank = SoundBank::default();
    let a = bank.insert_test_buffer(1.0, 1000, false);
    let b = bank.insert_test_buffer(1.0, 1000, false);
    let set = bank.insert_test_set(vec![a, b]);
    let mut held = None;
    let at = Vec3::new(1.0, 2.0, 3.0);
    // The hinge's creak at a squared handle speed of 1: speed 1, volume
    // 0.075, pitch 0.722.
    let step = drive_voice(
        &mut env,
        &mut bank,
        &mut held,
        set,
        at,
        Some((0.075, 0.722)),
    );
    assert_eq!(step, VoiceStep::Restarted);
    let id = held.unwrap();
    let voice = *env.voice(id).unwrap();
    assert_eq!(
        voice.flags,
        voice_flags::ALLOCATED | voice_flags::RESTART,
        "a restart, not a loop"
    );
    assert_eq!(
        (voice.gain, voice.pitch, voice.position),
        (0.075, 0.722, at)
    );
    // Not yet playing (the mixer has not run): restarted again.
    let step = drive_voice(&mut env, &mut bank, &mut held, set, at, Some((0.08, 0.73)));
    assert_eq!(step, VoiceStep::Restarted);
    // Playing: the levels change and the point stays.
    env.voice_mut(id).unwrap().flags |= voice_flags::ACTIVE;
    let moved = Vec3::new(4.0, 5.0, 6.0);
    let step = drive_voice(
        &mut env,
        &mut bank,
        &mut held,
        set,
        moved,
        Some((0.1, 0.75)),
    );
    assert_eq!(step, VoiceStep::Levelled);
    let voice = *env.voice(id).unwrap();
    assert_eq!((voice.gain, voice.pitch, voice.position), (0.1, 0.75, at));
    assert_eq!(held, Some(id));
    // Quiet: freed.
    let step = drive_voice(&mut env, &mut bank, &mut held, set, at, None);
    assert_eq!(step, VoiceStep::Stopped);
    assert_eq!(held, None);
    assert_eq!(env.voice(id).unwrap().flags, 0);
    // Quiet with nothing held does nothing.
    let step = drive_voice(&mut env, &mut bank, &mut held, set, at, None);
    assert_eq!(step, VoiceStep::Nothing);
}

/// The return's tick loop starts looping at 0.1 and pitch 1, follows the
/// fade (0.1 less 0.01 a tick) and is freed at its end; a second start
/// lets the first voice go.
#[test]
fn the_tick_loop_starts_fades_and_frees() {
    let mut env = AudioEnvironment::new();
    let mut bank = SoundBank::default();
    let buffer = bank.insert_test_buffer(10.0, 10_000, false);
    let mut held = None;
    let at = Vec3::new(0.5, 1.0, -2.0);
    drive_tick_loop(
        &mut env,
        &mut bank,
        &mut held,
        Some(buffer),
        at,
        TickCue::Start,
    );
    let id = held.unwrap();
    let voice = *env.voice(id).unwrap();
    assert_ne!(voice.flags & voice_flags::LOOPING, 0);
    assert_ne!(voice.flags & voice_flags::RESTART, 0);
    assert_eq!((voice.gain, voice.pitch), (0.1, 1.0));
    assert_eq!((voice.buffer, voice.position), (buffer, at));
    drive_tick_loop(
        &mut env,
        &mut bank,
        &mut held,
        Some(buffer),
        at,
        TickCue::Gain(0.09),
    );
    assert_eq!(env.voice(id).unwrap().gain, 0.09);
    drive_tick_loop(
        &mut env,
        &mut bank,
        &mut held,
        Some(buffer),
        at,
        TickCue::Start,
    );
    let again = held.unwrap();
    assert_eq!(env.voice(again).unwrap().gain, 0.1);
    if again != id {
        assert_eq!(env.voice(id).unwrap().flags, 0);
    }
    drive_tick_loop(
        &mut env,
        &mut bank,
        &mut held,
        Some(buffer),
        at,
        TickCue::Free,
    );
    assert_eq!(held, None);
    assert_eq!(env.voice(again).unwrap().flags, 0);
    // Without the file nothing starts.
    drive_tick_loop(&mut env, &mut bank, &mut held, None, at, TickCue::Start);
    assert_eq!(held, None);
}
