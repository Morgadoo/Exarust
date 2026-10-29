//! Tests of the clip clock, starts, captures and the living sampler, pinned
//! to the original's numbers (`docs/formats/RSQ.md`, Motion playback).
//! (Included from clock.rs: `#[path = "clock_test.rs"] mod tests;`)

use exa_formats::rsq::Key;

use super::*;

/// Bones in the test clips: the pelvis (bone 1, index 0) to the thorax
/// (bone 9, index 8).
const BONES: usize = 9;

/// Bone `i`'s turn at key `k`.
fn turn(k: usize, i: usize) -> Quat {
    Quat::from_rotation_y(0.1 * k as f32 + 0.01 * i as f32)
}

/// A clip with keys at `times` (frames) and the root at `roots` (cm).
fn clip(times: &[f32], roots: &[Vec3], looping: bool) -> Clip {
    Clip {
        mode: i32::from(looping),
        keys: times
            .iter()
            .zip(roots)
            .enumerate()
            .map(|(k, (&time, &root))| Key {
                time,
                root: (root * 0.01).to_array(),
                rotations: (0..BONES)
                    .map(|i| {
                        let q = turn(k, i);
                        [q.x, q.y, q.z, q.w]
                    })
                    .collect(),
                flags: vec![0; BONES],
            })
            .collect(),
    }
}

/// Idle's key times and root (cm).
fn idle() -> Clip {
    let root = Vec3::new(0.1, 100.7, 0.5);
    clip(&[20.0, 80.0], &[root, root], true)
}

/// A get-up whose first key is at `first` (frames), rising to `last`'s
/// time from Get Up 2's first root to its last.
fn get_up(first: f32, last: f32) -> Clip {
    clip(
        &[first, last],
        &[Vec3::new(2.1, 13.0, -31.1), Vec3::new(6.1, 81.7, -1.8)],
        false,
    )
}

/// An animator that has sampled `action`'s `clip` once at frame 5.
fn sampled(action: &str, clip: &Clip) -> Animator {
    let mut anim = Animator::new("Default", action);
    anim.time = 5.0;
    anim.sample(clip, false, None).unwrap();
    anim
}

fn close(a: Quat, b: Quat) -> bool {
    a.dot(b).abs() > 1.0 - 1e-6
}

/// Walk and Run 90° apart. At a full run the bone is 63° toward Run, the
/// root stays Walk's, and a full pose hold replaces the overlay.
#[test]
fn the_run_overlays_the_walk_by_seven_tenths() {
    let flat = Quat::IDENTITY;
    let right = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let bone = |q: Quat| -> Vec<[f32; 4]> {
        (0..BONES)
            .map(|i| if i == 0 { q } else { flat })
            .map(|q| [q.x, q.y, q.z, q.w])
            .collect()
    };
    let key = |time: f32, q: Quat, root: Vec3| Key {
        time,
        root: (root * 0.01).to_array(),
        rotations: bone(q),
        flags: vec![0; BONES],
    };
    let walk_root = Vec3::new(1.0, 2.0, 3.0);
    let walk = Clip {
        mode: 1,
        keys: vec![key(10.0, flat, walk_root), key(80.0, flat, walk_root)],
    };
    let run = Clip {
        mode: 1,
        keys: vec![
            key(10.0, right, Vec3::new(9.0, 9.0, 9.0)),
            key(80.0, right, Vec3::new(9.0, 9.0, 9.0)),
        ],
    };
    let mut anim = Animator::new("Default", "Walk");
    anim.time = 45.0;
    let pose = anim
        .sample_layered(&walk, false, None, Some((&run, 0.7)))
        .unwrap();
    assert!(close(pose.rotations[0], flat.slerp(right, 0.7)));
    assert!(close(pose.rotations[1], flat));
    assert!((pose.root - walk_root * 0.01).length() < 1e-5);

    let mut plain = Animator::new("Default", "Walk");
    plain.time = 45.0;
    let alone = plain.sample(&walk, false, None).unwrap();
    assert!(close(alone.rotations[0], flat));

    let held = Quat::from_rotation_x(0.2);
    let hold: Vec<Quat> = (0..BONES).map(|_| held).collect();
    let mut over = Animator::new("Default", "Walk");
    over.time = 45.0;
    let posed = over
        .sample_layered(&walk, false, Some((&hold, 1.0)), Some((&run, 0.7)))
        .unwrap();
    assert!(close(posed.rotations[0], held));
}

#[test]
fn eases_as_the_original() {
    assert_eq!(Ease::Linear.apply(0.3), 0.3);
    assert!((Ease::Smooth.apply(0.5) - 0.5).abs() < 1e-6);
    assert!((Ease::Smooth.apply(0.25) - 0.15625).abs() < 1e-6);
    assert!(Ease::Fast.apply(0.0).abs() < 1e-6);
    assert!((Ease::Fast.apply(0.1) - 0.455_444).abs() < 1e-5);
    assert!((Ease::Fast.apply(0.75) - 0.939_643).abs() < 1e-5);
    // It never passes 1: it ends at 0.999.
    assert!((Ease::Fast.apply(1.0) - 0.998_889).abs() < 1e-5);
}

#[test]
fn follow_ups_blend_by_the_clip_that_ended() {
    // Get up, Get Up 2, Fall Backward, the kick: no blend.
    for a in [0x0E, 0x0F, 0x11, 0x0D] {
        assert_eq!(follow_up_blend(Some(a), 7), 0, "{a:#x}");
    }
    // Switch Weapons and Kneel: 20; Rebalance and Get Up 3: 60.
    assert_eq!(follow_up_blend(Some(0x1F), 7), 20);
    assert_eq!(follow_up_blend(Some(0x20), 7), 20);
    assert_eq!(follow_up_blend(Some(0x21), 7), 60);
    assert_eq!(follow_up_blend(Some(0x2A), 7), 60);
    // An attack: twice its weapon's recovery (fists: 0).
    assert_eq!(follow_up_blend(Some(5), 7), 14);
    assert_eq!(follow_up_blend(Some(12), 7), 14);
    assert_eq!(follow_up_blend(Some(5), 0), 0);
    assert_eq!(follow_up_blend(None, 7), 0);
}

#[test]
fn only_walks_and_footwork_play_in_place() {
    for a in [0, 1, 2, 0x2B] {
        assert!(in_place(Some(a)), "{a:#x}");
    }
    for a in [3, 5, 0x0E, 0x0F, 0x13, 0x1D] {
        assert!(!in_place(Some(a)), "{a:#x}");
    }
    assert!(!in_place(None));
}

/// A plain capture keeps the root and the pelvis's and legs' last targets,
/// takes the upper body's physical turns, and makes them its targets; the
/// clip's first sample is that capture, and its next ones blend from it
/// to the first key by clock / (first key + blend).
#[test]
fn a_start_samples_its_capture_first() {
    let first = idle();
    let mut anim = sampled("Idle", &first);
    let before = anim.pose.clone().unwrap();
    anim.start("Get up", false, Some("Idle".into()), 20, Ease::Linear);
    let mut physical = vec![None; BONES];
    physical[0] = Some(Quat::from_rotation_z(0.7));
    physical[8] = Some(Quat::from_rotation_z(0.5));
    anim.take_capture(&physical);
    let capture = anim.capture.clone().unwrap();
    assert_eq!(capture.root, before.root);
    assert_eq!(capture.rotations[0], before.rotations[0]);
    assert_eq!(capture.rotations[8], Quat::from_rotation_z(0.5));
    assert_eq!(
        anim.pose.as_ref().unwrap().rotations[8],
        capture.rotations[8]
    );
    assert!(anim.pending.is_none());
    let up = get_up(20.0, 80.0);
    let at_start = anim.sample(&up, false, None).unwrap();
    assert_eq!(at_start.root, capture.root);
    for (got, want) in at_start.rotations.iter().zip(&capture.rotations) {
        assert!(close(*got, *want));
    }
    // One tick of a get-up's clock: 0.9 frames, 0.9/40 of the way.
    assert!(!anim.tick(&up, 0.9, 0));
    let next = anim.sample(&up, false, None).unwrap();
    let want = capture.rotations[8].slerp(turn(0, 8), 0.9 / 40.0);
    assert!(close(next.rotations[8], want));
}

/// The clip reaches its first key when its clock reaches the first key's
/// time plus its blend, and its blend ends there (the clock loses it):
/// Get up's 20 + 20 frames at 0.9 a tick take 44 ticks, Get Up 2's 15 + 20
/// take 38, Idle's 20 frames with no blend at 0.6 a tick take 33.
#[test]
fn clips_reach_their_first_key_on_time() {
    for (first, blend, rate, ticks) in
        [(20.0, 20, 0.9, 44), (15.0, 20, 0.9, 38), (20.0, 0, 0.6, 33)]
    {
        let up = get_up(first, 120.0);
        let mut anim = sampled("Idle", &idle());
        anim.start("Get up", false, Some("Idle".into()), blend, Ease::Linear);
        anim.take_capture(&[]);
        let capture = anim.capture.clone().unwrap();
        let mut blended = 0;
        loop {
            anim.tick(&up, rate, 0);
            let pose = anim.sample(&up, false, None).unwrap();
            if !anim.leading {
                break;
            }
            blended += 1;
            let f = blended as f32 * rate / (first + blend as f32);
            let want = capture.root.lerp(Vec3::new(0.021, 0.13, -0.311), f);
            assert!(pose.root.abs_diff_eq(want, 1e-5), "{first}: {}", pose.root);
        }
        assert_eq!(blended, ticks, "first key {first}, blend {blend}");
        assert_eq!(anim.blend, 0.0);
        assert!(anim.capture.is_none() && anim.ease == Ease::Linear);
        let past = (ticks + 1) as f32 * rate - blend as f32;
        assert!((anim.time - past).abs() < 1e-4, "{}", anim.time);
    }
}

/// A get-up keeps its root's travel and ends at its last key's root, Get
/// Up 2's (6.1, 81.7, −1.8) cm, not at its first key's place across the
/// ground; the stance it ends into starts from the last root sampled and
/// blends to Idle's over its first 20 frames, smoothly.
#[test]
fn a_get_up_ends_at_its_root_offset() {
    let up = get_up(15.0, 120.0);
    let stance = idle();
    let mut held = Animator::new("Default", "Get Up 2");
    held.looping = false;
    held.leading = false;
    held.time = 120.0;
    let end = held.sample(&up, false, None).unwrap().root;
    assert!(
        end.abs_diff_eq(Vec3::new(0.061, 0.817, -0.018), 1e-6),
        "{end}"
    );
    let mut anim = sampled("Idle", &stance);
    anim.start("Get Up 2", false, Some("Idle".into()), 20, Ease::Linear);
    anim.take_capture(&[]);
    let mut last = None;
    for _ in 0..200 {
        if anim.tick(&up, 0.9, follow_up_blend(Some(0x0F), 0)) {
            break;
        }
        last = anim.sample(&up, false, None);
    }
    // The last sample, under a tick's clock from the end.
    let last = last.unwrap();
    assert!(last.root.distance(end) < 0.007, "{}", last.root);
    assert_eq!((anim.action.as_str(), anim.looping), ("Idle", true));
    assert_eq!((anim.blend, anim.ease), (0.0, Ease::Smooth));
    anim.take_capture(&[]);
    assert_eq!(anim.capture.as_ref().unwrap().root, last.root);
    anim.tick(&stance, 0.6, 0);
    let first = anim.sample(&stance, false, None).unwrap();
    let f = Ease::Smooth.apply(0.6 / 20.0);
    let want = last.root.lerp(Vec3::new(0.001, 1.007, 0.005), f);
    assert!(first.root.abs_diff_eq(want, 1e-6), "{}", first.root);
    assert!((f - 0.002_646).abs() < 1e-6);
}

/// A looping clip wraps by its last key's time, and its first frames blend
/// from its last key back to its first.
#[test]
fn loops_wrap_by_their_last_key() {
    let times: Vec<f32> = (1..=8).map(|k| 10.0 * k as f32).collect();
    let roots: Vec<Vec3> = (0..8).map(|k| Vec3::new(0.0, 100.0, k as f32)).collect();
    let walk = clip(&times, &roots, true);
    let mut anim = Animator::new("Default", "Walk");
    anim.leading = false;
    anim.time = 79.7;
    assert!(!anim.tick(&walk, 0.6, 0));
    assert!((anim.time - 0.3).abs() < 1e-4, "{}", anim.time);
    anim.time = 5.0;
    let pose = anim.sample(&walk, false, None).unwrap();
    assert!(close(pose.rotations[3], turn(7, 3).slerp(turn(0, 3), 0.5)));
    assert!((pose.root.z - 0.035).abs() < 1e-6);
    let still = anim.sample(&walk, true, None).unwrap();
    assert_eq!(still.root.z, 0.0);
    anim.time = 0.3;
    assert!(!anim.tick(&walk, -0.8, 0));
    assert!((anim.time - 79.5).abs() < 1e-4, "{}", anim.time);
}

/// A clip played once without a follow-up holds its last key.
#[test]
fn a_clip_without_a_follow_up_holds_its_last_key() {
    let up = get_up(20.0, 80.0);
    let mut anim = Animator::new("Default", "Get up");
    anim.looping = false;
    anim.time = 79.5;
    assert!(!anim.tick(&up, 0.9, 0));
    assert!(anim.finished && anim.time == 80.0);
    let pose = anim.sample(&up, false, None).unwrap();
    assert!(close(pose.rotations[2], turn(1, 2)));
}

/// A whole capture takes every bone's physical turn, the pelvis's
/// included.
#[test]
fn a_whole_capture_takes_the_lying_body() {
    let mut anim = sampled("Idle", &idle());
    anim.start("Get up", false, Some("Idle".into()), 20, Ease::Linear);
    anim.capture_whole();
    let physical: Vec<Option<Quat>> = (0..BONES)
        .map(|i| Some(Quat::from_rotation_x(0.05 * i as f32 + 0.3)))
        .collect();
    anim.take_capture(&physical);
    let capture = anim.capture.unwrap();
    for (got, want) in capture.rotations.iter().zip(&physical) {
        assert_eq!(Some(*got), *want);
    }
}

/// A start before the clip's first sample has nothing to capture: the
/// clip starts from its first key (from its last when it loops).
#[test]
fn a_first_clip_starts_from_its_keys() {
    let up = get_up(20.0, 80.0);
    let mut anim = Animator::new("Default", "Idle");
    anim.play_once("Get up", "Idle");
    anim.take_capture(&[]);
    assert!(anim.capture.is_none() && anim.pending.is_none());
    anim.tick(&up, 0.9, 0);
    let pose = anim.sample(&up, false, None).unwrap();
    assert!(close(pose.rotations[4], turn(0, 4)));
    let mut looped = Animator::new("Default", "Idle");
    looped.time = 10.0;
    let stance = clip(
        &[20.0, 80.0],
        &[Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 100.0, 0.0)],
        true,
    );
    let pose = looped.sample(&stance, false, None).unwrap();
    assert!(close(pose.rotations[4], turn(1, 4).slerp(turn(0, 4), 0.5)));
    // A blend asked of such a start has nothing to blend from: the clip
    // plays from its clock's zero, from its last key to its first over
    // its first 20 frames (0.6 of a frame a tick), not from a clock set
    // back by the blend.
    let mut blended = Animator::new("Default", "Idle");
    blended.start("Cmbt - Idle", true, None, 40, Ease::Smooth);
    blended.take_capture(&[]);
    assert_eq!(blended.blend, 0.0);
    for tick in 1..=3 {
        blended.tick(&stance, 0.6, 0);
        let pose = blended.sample(&stance, false, None).unwrap();
        let f = 0.6 * tick as f32 / 20.0;
        assert!(close(pose.rotations[4], turn(1, 4).slerp(turn(0, 4), f)));
    }
    assert!((blended.time - 1.8).abs() < 1e-5, "{}", blended.time);
}

/// The pose a body holds pulls every bone's turn by the hold's weight: in
/// full at 1, part way between, not at all at the original's 1.19e-7.
#[test]
fn a_hold_pulls_toward_the_held_pose() {
    let stance = idle();
    let mut anim = sampled("Idle", &stance);
    let mut physical = vec![None; BONES];
    physical[0] = Some(Quat::from_rotation_z(0.9));
    physical[8] = Some(Quat::from_rotation_z(0.4));
    let hold = anim.hold_pose(&physical).unwrap();
    // The pelvis holds its target, the thorax its physical turn.
    assert_eq!(hold[0], anim.pose.as_ref().unwrap().rotations[0]);
    assert_eq!(hold[8], Quat::from_rotation_z(0.4));
    let clip_pose = anim.sample(&stance, false, None).unwrap();
    let full = anim.sample(&stance, false, Some((&hold, 1.0))).unwrap();
    for (got, want) in full.rotations.iter().zip(&hold) {
        assert!(close(*got, *want));
    }
    assert_eq!(full.root, clip_pose.root);
    assert_eq!(anim.held, 1.0);
    let part = anim.sample(&stance, false, Some((&hold, 0.25))).unwrap();
    assert!(close(
        part.rotations[8],
        clip_pose.rotations[8].lerp(hold[8], 0.25)
    ));
    let none = anim.sample(&stance, false, Some((&hold, 1.0e-7))).unwrap();
    assert_eq!(none, clip_pose);
}

/// Progress is the original's (clock − blend) / last key; Exarust's own
/// measure from the first key to the last stays 0 through the blend.
#[test]
fn progress_counts_from_the_clocks_zero() {
    let times = [20.0, 40.0, 60.0, 70.0, 80.0, 95.0, 110.0];
    let roots = [Vec3::new(0.0, 90.0, 0.0); 7];
    let attack = clip(&times, &roots, false);
    let mut anim = Animator::new("Default", "Attack Mid");
    anim.play_once("Attack Mid", "Cmbt - Idle");
    anim.blended(20, Ease::Smooth);
    anim.time = 30.0;
    assert!((anim.clip_progress(&attack) - 10.0 / 110.0).abs() < 1e-6);
    anim.time = 20.0 + 20.0 + 4.5;
    assert!((anim.clip_progress(&attack) - 24.5 / 110.0).abs() < 1e-6);
}

/// Ticks a jumped-in get-up, 0.75 frames on its first tick (the
/// off-balance rate, as its start's tick runs) and 0.9 a tick after, until
/// its clock passes its first key, checking that it holds its first key
/// exactly until then; returns the ticks after the first and the clock
/// after the first.
fn hold_first_key(anim: &mut Animator, rise: &Clip) -> (usize, f32) {
    assert!(!anim.tick(rise, 0.75, 0));
    let pose = anim.sample(rise, false, None).unwrap();
    let rebased = anim.time;
    assert_eq!(anim.blend, 0.0);
    assert!(!anim.leading && anim.capture.is_none());
    for (i, r) in pose.rotations.iter().enumerate() {
        assert!(close(*r, turn(0, i)), "bone {i}");
    }
    let first = rise.keys[0].time;
    let mut ticks = 0;
    while anim.time <= first {
        anim.tick(rise, 0.9, 0);
        let held = anim.sample(rise, false, None).unwrap();
        if anim.time <= first {
            assert!(close(held.rotations[4], turn(0, 4)));
        }
        ticks += 1;
    }
    (ticks, rebased)
}

/// The stagger's jump in, as the original's runs: Get up (keys 20, 40, 55,
/// 70, 80) started with a blend of 50 three keys in sets its clock to 55;
/// after its first tick at 0.75 its first sample finds the capture meant
/// for key 3 while it aims at the first, so it rebases at once (clock
/// 5.75, no blend) and samples the first key exactly, which it holds until
/// its clock passes 20, 16 ticks later at 0.9. Get Up 2 (15, 40, …) two
/// keys in starts at 40 and rebases to −9.25, 27 ticks before its first
/// key.
#[test]
fn a_stagger_jumps_in_and_holds_the_first_key() {
    let at = |n: usize| vec![Vec3::new(-2.1, 51.0, 26.0); n];
    let rise = clip(&[20.0, 40.0, 55.0, 70.0, 80.0], &at(5), false);
    let mut anim = sampled("Idle", &idle());
    anim.start("Get up", false, Some("Idle".into()), 50, Ease::Linear);
    anim.capture_whole();
    anim.jump_in(&rise, 3);
    assert_eq!((anim.time, anim.capture_key), (55.0, 3));
    anim.take_capture(&vec![Some(Quat::from_rotation_x(1.2)); BONES]);
    let (ticks, rebased) = hold_first_key(&mut anim, &rise);
    assert!((rebased - 5.75).abs() < 1e-4, "{rebased}");
    assert_eq!(ticks, 16);

    let times = [15.0, 40.0, 63.0, 82.0, 98.0, 110.0, 120.0];
    let rise = clip(&times, &at(7), false);
    let mut anim = sampled("Idle", &idle());
    anim.start("Get Up 2", false, Some("Idle".into()), 50, Ease::Linear);
    anim.jump_in(&rise, 2);
    assert_eq!(anim.time, 40.0);
    anim.take_capture(&[]);
    let (ticks, rebased) = hold_first_key(&mut anim, &rise);
    assert!((rebased + 9.25).abs() < 1e-4, "{rebased}");
    assert_eq!(ticks, 27);

    // Key 0, or one past the clip's, leaves the start as it was.
    let mut plain = sampled("Idle", &idle());
    plain.start("Get up", false, None, 20, Ease::Linear);
    plain.jump_in(&rise, 0);
    plain.jump_in(&rise, 9);
    assert_eq!((plain.time, plain.capture_key), (0.0, 0));
}

/// A tick keeps the frames it added as the rate and whether its end test
/// fired; a sample writes the progress, (clock − blend) / the last key,
/// and a start resets it: the walk at 79 + 0.6 is 0.995 through, the
/// next tick wraps it (the end test fires, the clock reads 0.2) and a
/// get-up's end into its follow-up fires it too, as does a clip without
/// one finishing.
#[test]
fn the_end_test_rate_and_progress_are_exposed() {
    let times: Vec<f32> = (1..=8).map(|k| 10.0 * k as f32).collect();
    let roots = vec![Vec3::new(0.0, 100.0, 0.0); 8];
    let walk = clip(&times, &roots, true);
    let mut anim = Animator::new("Default", "Walk");
    anim.leading = false;
    anim.time = 79.0;
    assert!(!anim.tick(&walk, 0.6, 0));
    assert!((anim.rate - 0.6).abs() < 1e-6 && !anim.ended);
    anim.sample(&walk, true, None).unwrap();
    assert!(
        (anim.progress - 79.6 / 80.0).abs() < 1e-6,
        "{}",
        anim.progress
    );
    assert!(!anim.tick(&walk, 0.6, 0));
    assert!(anim.ended && (anim.time - 0.2).abs() < 1e-4);
    anim.sample(&walk, true, None).unwrap();
    assert!((anim.progress - 0.2 / 80.0).abs() < 1e-6);
    anim.start("Idle", true, None, 0, Ease::Linear);
    assert_eq!(anim.progress, 0.0);

    let up = get_up(20.0, 80.0);
    anim.start("Get up", false, Some("Idle".into()), 0, Ease::Linear);
    anim.time = 79.5;
    assert!(anim.tick(&up, 0.9, 20));
    assert!(anim.ended && anim.action == "Idle" && (anim.rate - 0.9).abs() < 1e-6);
    let mut once = Animator::new("Default", "Get up");
    once.looping = false;
    once.time = 79.5;
    assert!(!once.tick(&up, 0.9, 0));
    assert!(once.ended && once.finished);
}

/// The installed clips (skipped without the game) loop and lead in as the
/// original's clock plays them: Walk's keys (10…80) first and last differ
/// by up to 19.9° on a joint, and the loop at 0.6 a tick turns no joint
/// more than 1.3° across the wrap (the old wrap by 70 popped the 19.9°);
/// the stances' first and last keys are the same pose (within 0.1°), so
/// their frames before the first key hold it; Get up (first key 20) and
/// Get Up 2 (15)
/// started with a blend of 20 at 0.9 a tick blend from the capture for 44
/// and 38 ticks (the 45th and 39th pass the first key), Idle with no
/// blend at 0.6 for 33.
#[test]
fn the_installed_clips_loop_and_lead_in_as_the_original() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let data = std::sync::Arc::new(exa_assets::GameData::open(&dir).unwrap());
    let motions = crate::Motions::load(&exa_assets::Game(data));
    let clip_of = |action: &str| {
        motions
            .clip(&Animator::new("Default", action), action)
            .unwrap()
            .clone()
    };
    let degrees = |a: &[f32; 4], b: &[f32; 4]| to_quat(*a).angle_between(to_quat(*b)).to_degrees();
    let widest = |a: &Key, b: &Key| {
        a.rotations
            .iter()
            .zip(&b.rotations)
            .map(|(a, b)| degrees(a, b))
            .fold(0.0, f32::max)
    };

    let walk = clip_of("Walk");
    let times: Vec<f32> = walk.keys.iter().map(|k| k.time).collect();
    assert_eq!(times, (1..=8).map(|k| 10.0 * k as f32).collect::<Vec<_>>());
    assert!(walk.loops());
    let pop = widest(&walk.keys[0], &walk.keys[7]);
    assert!((pop - 19.9).abs() < 0.15, "{pop}");
    let mut anim = Animator::new("Default", "Walk");
    anim.leading = false;
    anim.time = 79.7;
    let before = anim.sample(&walk, true, None).unwrap();
    assert!(!anim.tick(&walk, 0.6, 0));
    let after = anim.sample(&walk, true, None).unwrap();
    let turned = before
        .rotations
        .iter()
        .zip(&after.rotations)
        .map(|(a, b)| a.angle_between(*b).to_degrees())
        .fold(0.0, f32::max);
    assert!(turned < 1.3, "{turned}");

    for name in ["Idle", "Defence stance", "Cmbt - Idle"] {
        let stance = clip_of(name);
        assert!(stance.loops(), "{name}");
        let (first, last) = (stance.keys.first().unwrap(), stance.keys.last().unwrap());
        // The same pose within the stored keys' quantisation (Idle's
        // differ by 0.04°).
        assert!(widest(first, last) < 0.1, "{name}: {}", widest(first, last));
        let mut anim = Animator::new("Default", name);
        anim.leading = false;
        anim.time = first.time * 0.5;
        let held = anim.sample(&stance, false, None).unwrap();
        let off = held
            .rotations
            .iter()
            .zip(&first.rotations)
            .map(|(a, b)| a.angle_between(to_quat(*b)).to_degrees())
            .fold(0.0, f32::max);
        assert!(off < 0.1, "{name}: {off}");
    }

    let idle = clip_of("Idle");
    let lead_ins = [
        ("Get up", 20, 0.9, 44),
        ("Get Up 2", 20, 0.9, 38),
        ("Idle", 0, 0.6, 33),
    ];
    for (name, blend, rate, ticks) in lead_ins {
        let rise = clip_of(name);
        let first = rise.keys[0].time;
        assert_eq!(first, if name == "Get Up 2" { 15.0 } else { 20.0 });
        let mut anim = Animator::new("Default", "Idle");
        anim.time = 5.0;
        anim.sample(&idle, false, None).unwrap();
        anim.start(name, name == "Idle", None, blend, Ease::Linear);
        anim.take_capture(&[]);
        let mut blended = 0;
        loop {
            anim.tick(&rise, rate, 0);
            anim.sample(&rise, false, None).unwrap();
            if !anim.leading {
                break;
            }
            blended += 1;
        }
        assert_eq!(blended, ticks, "{name}");
    }
}

/// The standing pace and a get-up's are one double product. A rate of 1.5
/// gains the double 1.5 × 0.6, which narrows to 0.8999999761581421, one
/// float under the single-precision product; half tone and a NaN tone gain
/// nothing.
#[test]
fn the_pace_is_a_double_product() {
    assert_eq!(standing_frames(1.5), 1.5 * 0.6);
    assert_eq!((standing_frames(1.5) as f32).to_bits(), 0x3f66_6666);
    assert_eq!((rising_frames(1.5, 1.0) as f32).to_bits(), 0x3f66_6666);
    assert_eq!((rising_frames(1.1875, 1.0) as f32).to_bits(), 0x3f36_6666);
    assert_ne!(
        (standing_frames(1.5) as f32).to_bits(),
        (1.5f32 * 0.6).to_bits()
    );
    assert_eq!(rising_frames(1.5, 0.5), 0.0);
    assert_eq!(rising_frames(1.5, f32::NAN), 0.0);
}

/// The 0.6-rate clock adds the double product to the widened clock and
/// narrows once; an attack's rate adds in single precision. From 0.0274 a
/// standing human's 0.6 lands on 0.20739999, the single add on 0.20740001;
/// from 0.0548 a get-up at 1.5 × 0.6 × 0.7 = 0.63 lands on 0.68480003.
#[test]
fn the_six_tenths_clock_adds_in_double() {
    let mut anim = Animator::new("Default", "Idle");
    anim.time = 0.0274;
    assert!(!anim.tick_double(&idle(), 0.3 * 0.6, 0));
    assert_eq!(anim.time, 0.207_399_99);
    let mut single = Animator::new("Default", "Idle");
    single.time = 0.0274;
    single.tick(&idle(), (0.3 * 0.6) as f32, 0);
    assert_eq!(single.time, 0.207_400_01);
    let mut up = Animator::new("Default", "Get up");
    up.time = 0.0548;
    up.tick_double(&get_up(20.0, 120.0), 1.5 * 0.6 * 0.7, 0);
    assert_eq!(up.time, 0.684_800_03);
}

/// The limp sampler takes the bracket at the raw clock: before the first
/// key (plus the remaining blend) it blends from the last key of a looping
/// clip, or from the first; between keys it blends as the living sampler
/// does; past the last key a looping clip blends from it back to the first
/// without clamping, and one played once blends from the key before the
/// last to the last with a share of 1.
#[test]
fn the_limp_sampler_brackets_at_the_raw_clock() {
    let walk = clip(&[20.0, 80.0], &[Vec3::ZERO, Vec3::ZERO], true);
    let b = ragdoll_bracket(&walk, 10.0, true, 4.0).unwrap();
    assert_eq!((b.src, b.dst), (1, 0));
    assert!((b.f - 10.0 / 24.0).abs() < 1e-6, "{}", b.f);
    let b = ragdoll_bracket(&walk, 30.0, true, 0.0).unwrap();
    assert_eq!((b.src, b.dst), (0, 1));
    assert!((b.f - 10.0 / 60.0).abs() < 1e-6, "{}", b.f);
    // Past the last key, looping: from it to the first, past 1 allowed.
    let b = ragdoll_bracket(&walk, 110.0, true, 0.0).unwrap();
    assert_eq!((b.src, b.dst), (1, 0));
    assert!((b.f - 30.0 / 20.0).abs() < 1e-6, "{}", b.f);
    // Played once past its last key: the key before the last holds it.
    let once = get_up(20.0, 80.0);
    let b = ragdoll_bracket(&once, 90.0, false, 0.0).unwrap();
    assert_eq!((b.src, b.dst, b.f), (0, 1, 1.0));
    // A sample starts from the keys and keeps the root's travel.
    let mut anim = Animator::new("Default", "Walk");
    anim.time = 110.0;
    let (pose, _) = anim.sample_ragdoll(&walk, true).unwrap();
    assert_eq!(pose.rotations.len(), BONES);
}

/// The limp sampler leads in from a capture while it aims at the
/// capture's key, unblended by any ease; aiming at the next key drops the
/// capture and the blend but keeps the clock.
#[test]
fn the_limp_sampler_leads_in_from_its_capture() {
    let walk = clip(&[20.0, 80.0], &[Vec3::ZERO, Vec3::ZERO], true);
    let mut anim = Animator::new("Default", "Walk");
    anim.capture = Some(ClipPose {
        rotations: vec![Quat::from_rotation_y(0.4); BONES],
        root: Vec3::ZERO,
    });
    anim.ease = Ease::Smooth;
    anim.blend = 4.0;
    anim.time = 12.0;
    let (pose, b) = anim.sample_ragdoll(&walk, true).unwrap();
    assert!(b.from_capture);
    // 12 / 24 = 0.5 of the way from the capture, not eased.
    assert!((b.f - 0.5).abs() < 1e-6, "{}", b.f);
    let half = crate::slerp(Quat::from_rotation_y(0.4), turn(0, 0), 0.5);
    assert!(pose.rotations[0].angle_between(half) < 1e-5);
    // Past the first key the lead ends: no capture, no blend, same clock.
    anim.time = 30.0;
    let (_, b) = anim.sample_ragdoll(&walk, true).unwrap();
    assert!(!b.from_capture);
    assert!(anim.capture.is_none() && !anim.leading);
    assert_eq!((anim.blend, anim.time), (0.0, 30.0));
}

/// Every capture notes whether each foot touched; slots never noted read
/// as unset, and a later capture overwrites the slots it names.
#[test]
fn a_capture_notes_its_planted_feet() {
    let mut anim = Animator::new("Default", "Walk");
    assert!(!anim.captured_planted(5));
    anim.note_planted(&[(5, true), (2, false)]);
    assert!(anim.captured_planted(5) && !anim.captured_planted(2));
    anim.note_planted(&[(5, false)]);
    assert!(!anim.captured_planted(5));
    assert!(!anim.captured_planted(40));
}

/// The hold pose keeps the last targets of skin bones 1 to 7 (its pose
/// slots 0 to 6) and takes the physical turn from bone 8, the same bones as
/// the plain capture.
#[test]
fn the_hold_keeps_the_same_bones_as_the_capture() {
    let mut anim = Animator::new("Default", "Idle");
    let mut pose = ClipPose {
        rotations: vec![Quat::IDENTITY; BONES],
        root: Vec3::ZERO,
    };
    pose.rotations[6] = Quat::from_rotation_y(0.2);
    pose.rotations[7] = Quat::from_rotation_y(0.3);
    anim.keep(&pose, 0.0);
    let physical: Vec<Option<Quat>> = (0..BONES)
        .map(|i| Some(Quat::from_rotation_x(0.1 * (i + 1) as f32)))
        .collect();
    let held = anim.hold_pose(&physical).unwrap();
    // Bone 7 (index 6) keeps its target; bone 8 (index 7) is physical.
    assert_eq!(held[5], pose.rotations[5]);
    assert_eq!(held[6], pose.rotations[6]);
    assert_eq!(held[7], physical[7].unwrap());
}

/// The shock hold copies the captured pose for bones 1 to 7 unless the body
/// is down, and turns every other bone by a random axis and angle blended
/// half way from the Defence stance's first key.
#[test]
fn the_shock_hold_turns_the_upper_body() {
    let capture: Vec<Quat> = (0..BONES).map(|i| turn(0, i)).collect();
    let defence: Vec<Quat> = (0..BONES).map(|_| Quat::IDENTITY).collect();
    let mut draws = [0.5, 0.0, 0.5, 0.5].into_iter().cycle();
    let held = shock_hold(&defence, &capture, false, &mut || draws.next().unwrap());
    for i in 0..7 {
        assert_eq!(held[i], capture[i]);
    }
    // Axis (0, −0.5, 0) normalised is −Y; angle 0.5; identity blended half
    // way from it is half the angle.
    let want = Quat::from_axis_angle(Vec3::NEG_Y, 0.25);
    assert!(held[7].abs_diff_eq(want, 1e-6), "{}", held[7]);
    // Down, every bone takes the random turn, the lower body included.
    let mut draws = [0.5, 0.0, 0.5, 0.5].into_iter().cycle();
    let down = shock_hold(&defence, &capture, true, &mut || draws.next().unwrap());
    assert!(down[0].abs_diff_eq(want, 1e-6));
}

/// Starting another clip from Walk or Run leaves a trailing-foot request,
/// taken once; a stance or a clip that does not replace locomotion sets
/// nothing, and a restart of Walk does not ask.
#[test]
fn leaving_walk_or_run_asks_for_the_trailing_foot() {
    let mut anim = Animator::new("Default", "Walk");
    anim.start("Attack Mid", false, None, 0, Ease::Smooth);
    assert!(anim.take_left_locomotion());
    assert!(!anim.take_left_locomotion());
    anim.action = "Run".into();
    anim.start("Idle", true, None, 20, Ease::Linear);
    assert!(anim.take_left_locomotion());
    anim.action = "Idle".into();
    anim.start("Defence stance", true, None, 30, Ease::Smooth);
    assert!(!anim.take_left_locomotion());
    anim.action = "Walk".into();
    anim.start("Walk", true, None, 0, Ease::Linear);
    assert!(!anim.take_left_locomotion());
}

/// The pose-only sampler's key pick: the lead-in by time / (first +
/// blend) from the capture while it aims at the capture's key, unclamped
/// and unguarded between the keys, the loop back to key 0 past the last.
#[test]
fn the_placement_bracket_follows_the_pose_only_sampler() {
    let roots = [Vec3::ZERO; 3];
    let held = clip(&[10.0, 20.0, 40.0], &roots, false);
    // Lead-in: t′ = 4 under the first key; f = 9 / (10 + 5), no ease.
    let b = placement_bracket(&held, 9.0, 5.0, false, Some(0)).unwrap();
    assert_eq!(b.progress, 4.0 / 40.0);
    assert_eq!((b.bracket.src, b.bracket.dst), (0, 0));
    assert_eq!(b.bracket.f, 9.0 / 15.0);
    assert!(b.bracket.from_capture && !b.rebase);
    // Between the keys: the capture's key is past, so it rebases.
    let b = placement_bracket(&held, 30.0, 0.0, false, Some(0)).unwrap();
    assert_eq!((b.bracket.src, b.bracket.dst, b.bracket.f), (1, 2, 0.5));
    assert!(!b.bracket.from_capture && b.rebase);
    // Past the end a clip played once holds its last key...
    let b = placement_bracket(&held, 55.0, 0.0, false, None).unwrap();
    assert_eq!((b.bracket.src, b.bracket.dst, b.bracket.f), (2, 2, 0.0));
    // ...a looping one blends back to key 0, unclamped.
    let b = placement_bracket(&held, 65.0, 0.0, true, None).unwrap();
    assert_eq!((b.bracket.src, b.bracket.dst, b.bracket.f), (2, 0, 2.5));
    // The lead-in of a looping clip comes from its last key.
    let b = placement_bracket(&held, 5.0, 0.0, true, None).unwrap();
    assert_eq!((b.bracket.src, b.bracket.dst, b.bracket.f), (2, 0, 0.5));
    // A first key at time 0 gives no share.
    let flat = clip(&[0.0, 20.0], &roots[..2], false);
    assert_eq!(
        placement_bracket(&flat, 0.0, 0.0, false, None)
            .unwrap()
            .bracket
            .f,
        0.0
    );
    // Where both agree, the shared bracket gives the same pick.
    for t in [3.0, 15.0, 39.0] {
        let p = placement_bracket(&held, t, 0.0, false, None)
            .unwrap()
            .bracket;
        let q = bracket(&held, t, false).unwrap();
        assert_eq!((p.src, p.dst, p.f), (q.src, q.dst, q.f), "{t}");
    }
}

/// The root comes from the source key for Walk, Run and Backpedal with
/// muscles on, from the bracket's source otherwise.
#[test]
fn the_placement_root_comes_from_the_key_for_the_gaits() {
    assert!(placement_root_from_key(true, Some(0)));
    assert!(placement_root_from_key(true, Some(2)));
    assert!(!placement_root_from_key(true, Some(3)));
    assert!(!placement_root_from_key(false, Some(1)));
    assert!(!placement_root_from_key(true, None));
}

/// A queued clip takes over only on a tick whose target key changed.
#[test]
fn a_queued_clip_waits_for_a_new_target_key() {
    assert!(switches_to_queued(4, 3, 2, 1));
    assert!(!switches_to_queued(4, 3, 2, 2));
    assert!(!switches_to_queued(3, 3, 2, 1));
}
