use super::*;
use crate::attack::AttackMotion::{Main, Other, Overhead, Stab};
use exa_formats::weapon_record::{Holder, Lengths, Section};
use exa_formats::weapon_stats::{StatBytes, at};

/// A sword weighing 0.3 with balance 0.6: speed 1.155, modulation 0.4,
/// recoil 0.46, recovery round(7) = 7 (a 14-frame blend into the stance).
fn sword() -> WeaponRecord {
    let mut stats = StatBytes::default();
    stats.set(at::WEIGHT, 30);
    stats.set(at::BALANCE, 60);
    WeaponRecord::of_one_part(0x11, &stats, false, 90.0)
}

/// The master note's Sword A: weight 0.5, balance 0.8 (speed 1.125,
/// modulation 0.2).
fn sword_a() -> Swing {
    let mut stats = StatBytes::default();
    stats.set(at::WEIGHT, 50);
    stats.set(at::BALANCE, 80);
    Swing::of(&WeaponRecord::of_one_part(0x11, &stats, false, 90.0))
}

fn fists() -> Swing {
    Swing::of(&WeaponRecord::build(
        &Section::FISTS,
        false,
        Lengths::hand(8.0, 9.66),
        &Holder::AVERAGE,
    ))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

/// The sword runs 0.87 frames a tick at its start (progress 0), 0.97 at
/// its end, 1.30 mid-swing; its recoil and handling are its record's.
#[test]
fn a_sword_paces_its_swing() {
    let swing = Swing::of(&sword());
    assert!((swing.speed - 1.155).abs() < 1e-5);
    assert!((swing.modulation - 0.4).abs() < 1e-5);
    assert_eq!(swing.recovery, 7);
    assert!((swing.recoil - 0.46).abs() < 1e-5, "{swing:?}");
    assert!((swing.handling - 0.7125).abs() < 1e-5);
    let at = |p: f32| swing.rate(false, p, 1.0, 0.0);
    assert!((at(0.0) - 1.155 * (1.0 - 0.833 * 0.4) * 1.125).abs() < 1e-5);
    assert!((at(0.0) - 0.8664).abs() < 1e-3);
    assert!((at(1.0) - 0.9746).abs() < 1e-3);
    assert!((at(0.5) - 1.2994).abs() < 1e-3);
    // d = 0.25: 1 − 0.833·0.4·0.3125.
    assert!((at(0.25) - 1.155 * 0.895_875 * 1.125).abs() < 1e-5);
}

/// The master note's examples at the human clip base 1.0 (its 1.07, 1.22
/// and 1.56 took the superseded (80/83) base): Sword A 1.055 frames a
/// tick before its first key, 1.107 just after, 1.266 mid-swing; the
/// fists 1.62 throughout, so a 110-frame punch takes 68 ticks (816 ms).
#[test]
fn the_notes_swords_and_fists() {
    let a = sword_a();
    assert!((a.speed - 1.125).abs() < 1e-6 && (a.modulation - 0.2).abs() < 1e-6);
    assert!(close(a.rate(false, 0.0, 1.0, 0.0), 1.054_772));
    assert!(close(a.rate(false, 1e-6, 1.0, 0.0), 1.107_486));
    assert!(close(a.rate(false, 0.5, 1.0, 0.0), 1.265_625));
    let f = fists();
    assert!(close(f.speed, Swing::FISTS.speed) && f.recovery == 0);
    assert!(close(f.recoil, 0.1) && close(f.handling, 0.8));
    for p in [0.0, 0.3, 0.5, 0.9] {
        assert!(close(f.rate(false, p, 1.0, 0.0), 1.62));
    }
    let (mut t, mut ticks) = (0.0f32, 0);
    while t / 110.0 < 1.0 {
        t += f.rate(false, t / 110.0, 1.0, 0.0);
        ticks += 1;
    }
    assert_eq!((ticks, ticks * 12), (68, 816));
}

/// A stab is 1.2 times faster, and slower early: 0.5 + 1.2p of it under
/// 40 % of the clip.
#[test]
fn a_stab_starts_slowly() {
    let even = Swing {
        speed: 1.0,
        modulation: 0.0,
        ..Swing::FISTS
    };
    let stab = |p: f32| even.rate(true, p, 1.0, 0.0);
    assert!((stab(0.1) - 1.125 * 1.2 * 0.62).abs() < 1e-5);
    assert!((stab(0.5) - 1.125 * 1.2).abs() < 1e-5);
    assert_eq!(even.frames(Some(Stab.index()), 0.1, 1.0, 1.0), stab(0.1));
}

/// A feral character swings slower early and faster late: at feral 1 half
/// the rate before the first key, 1.5 times at the clip's end, the same
/// mid-swing; at 0.5 three quarters before the first key.
#[test]
fn feral_swings_ramp_up() {
    let f = Swing::FISTS;
    let plain = |p: f32| f.rate(false, p, 1.0, 0.0);
    assert!(close(f.rate(false, 0.0, 1.0, 1.0), plain(0.0) * 0.5));
    assert!(close(f.rate(false, 1.0, 1.0, 1.0), plain(1.0) * 1.5));
    assert!(close(f.rate(false, 0.5, 1.0, 1.0), plain(0.5)));
    assert!(close(f.rate(false, 0.0, 1.0, 0.5), plain(0.0) * 0.75));
}

/// A tired fighter's swing: stopped under 80 % of its energy, the share
/// squared under all of it (in place of the weapon's rate), and scaled by
/// the clock's base.
#[test]
fn energy_slows_the_swing() {
    let s = Swing::FISTS;
    assert_eq!(s.frames(Some(5), 0.5, 1.0, 0.79), 0.0);
    assert!((s.frames(Some(5), 0.5, 1.0, 0.9) - 0.81).abs() < 1e-6);
    assert!((s.frames(Some(5), 0.5, 1.5, 1.0) - 1.62 * 1.5).abs() < 1e-5);
    assert!(close(paced(0.3, 0.8), 0.64));
}

/// The expected rest of a swing (ms, at 80 ticks a second): a 110-frame
/// punch 849 ms from its start; Sword A's 100-frame swing 1053 ms from
/// its start, 426 ms from 0.6; the 0.3/0.6 sword 802 ms from 0.25;
/// nothing past the end.
#[test]
fn the_expected_swing_time() {
    assert_eq!(Swing::FISTS.expected_ms(0.0, 110.0, 1.0), 849);
    let a = sword_a();
    assert_eq!(a.expected_ms(0.0, 100.0, 1.0), 1053);
    assert_eq!(a.expected_ms(0.6, 100.0, 1.0), 426);
    assert_eq!(Swing::of(&sword()).expected_ms(0.25, 100.0, 1.0), 802);
    assert_eq!(Swing::FISTS.expected_ms(1.2, 110.0, 1.0), 0);
}

/// A released swing drains by max(1 − 0.05·|v|, 0.1)/W17 × 0.03 a tick,
/// at least 0.01: the fists (W17 0.1) still 0.3, at 10 cm a tick 0.15;
/// the arena sword (W17 0.42) still 0.0714, at 10 cm a tick 0.0357, at
/// 20 cm a tick 0.01. Feint may renew it after (2f + 1) ×
/// (round(100·W17) + 100) ms: 110/220/330 ms for the fists at 0, 0.35
/// and 0.8 of the clip, 142/284/426 for the sword (161 in its lead-in at
/// −0.1).
#[test]
fn a_released_swing_stalls_by_its_recoil() {
    let fists = Swing::FISTS;
    let arena = Swing {
        recoil: 0.42,
        ..sword_a()
    };
    assert!(close(fists.release_stall(0.0), 0.3));
    assert!(close(fists.release_stall(100.0), 0.15));
    assert!(close(arena.release_stall(0.0), 0.071_428_57));
    assert!(close(arena.release_stall(100.0), 0.035_714_28));
    assert!(close(arena.release_stall(400.0), 0.01));
    let renew = |s: &Swing, p: f32| s.renew_after_ms(p, Some(Main(0)), false);
    assert_eq!([0.0, 0.35, 0.8].map(|p| renew(&fists, p)), [110, 220, 330]);
    assert_eq!([0.0, 0.35, 0.8].map(|p| renew(&arena, p)), [142, 284, 426]);
    assert_eq!(renew(&arena, -0.1), 161);
    // Out of an attack's clip, or chained past its carried window: 3×.
    assert_eq!(arena.renew_after_ms(0.0, None, false), 426);
    assert_eq!(arena.renew_after_ms(0.0, Some(Other(1)), true), 426);
}

/// Where a released swing stands: a punch released at 12 000 ms at the
/// clip's start stalls 0.3 a tick and is back in its stance 42 ms later;
/// the sword's 175 ms later (1000/(80 × stall)), its renewal at 142 ms.
/// A late, fast sword (20 cm a tick tracked) 1250 ms by its stall, capped
/// at its expected 426 ms.
#[test]
fn a_release_sets_the_stall_and_its_end() {
    let at = |progress: f32, last_key: f32| SwingAt {
        now_ms: 12_000,
        motion: Some(Main(0)),
        progress,
        last_key,
        base: 1.0,
    };
    let mut punch = SwingState::default();
    punch.start(true, false);
    punch.release(&Swing::FISTS, &at(0.0, 110.0));
    assert!(punch.has(flags::STALLING) && punch.has(flags::RELEASED));
    assert!(punch.has(flags::NO_CHAIN));
    assert!(close(punch.stall, 0.3));
    assert_eq!((punch.end_ms, punch.renew_ms), (12_042, 12_110));
    let arena = Swing {
        recoil: 0.42,
        ..sword_a()
    };
    let mut cut = SwingState::default();
    cut.release(&arena, &at(0.0, 100.0));
    assert_eq!((cut.end_ms, cut.renew_ms), (12_175, 12_142));
    let mut late = SwingState::default();
    late.tracker[0].speed2 = 400.0;
    late.release(&arena, &at(0.6, 100.0));
    assert!(close(late.stall, 0.01));
    assert_eq!(late.end_ms, 12_000 + 426);
}

/// A struck attacker's stall: daze 0.3 on a still swing stalls 1.2 a tick
/// (a first stall is not capped), its stance back round(80 × 1/1.2) =
/// 67 ms later; daze 0.2 with the striker at 10 cm a tick and W17 0.42
/// 0.632 (127 ms); a fast swing (30 cm a tick) shrugs off a 0.05 daze; a
/// second stall adds up to 1; a spent swing takes none.
#[test]
fn a_daze_stalls_a_struck_swing() {
    let at = SwingAt {
        now_ms: 5_000,
        motion: Some(Main(1)),
        progress: 0.4,
        last_key: 100.0,
        base: 1.0,
    };
    let arena = Swing {
        recoil: 0.42,
        ..sword_a()
    };
    let mut s = SwingState::default();
    assert!(close(s.daze_stall(&arena, &at, 0.3, 1.0).unwrap(), 1.2));
    assert!(s.has(flags::STALLING) && !s.has(flags::RELEASED));
    assert!(close(s.stall, 1.2));
    assert_eq!(s.end_ms, 5_067);
    let mut s = SwingState::default();
    s.tracker[0].speed2 = 100.0;
    assert!(close(s.daze_stall(&arena, &at, 0.2, 1.0).unwrap(), 0.632));
    assert_eq!(s.end_ms, 5_127);
    // A second blow adds its stall, up to 1.
    s.daze_stall(&arena, &at, 0.2, 1.0);
    assert_eq!(s.stall, 1.0);
    let mut fast = SwingState::default();
    fast.tracker[1].speed2 = 900.0;
    assert_eq!(fast.daze_stall(&arena, &at, 0.05, 1.0), None);
    assert!(!fast.has(flags::STALLING));
    let mut spent = SwingState {
        strength: 0.0,
        ..SwingState::default()
    };
    assert_eq!(spent.daze_stall(&arena, &at, 0.5, 1.0), None);
    // The second factor: 0.25 over the most health, halved by a potion.
    assert_eq!(daze_factor(0.25, false), 1.0);
    assert_eq!(daze_factor(0.25, true), 0.5);
    assert_eq!(daze_factor(0.5, false), 0.5);
}

/// A swing tick of `motion` at `progress` (the clip's last key 100) at
/// 10 000 ms for a fighter of weapon class `class`, its first striker
/// moving at `v` cm a tick.
fn tick_at(motion: AttackMotion, progress: f32, class: u8, v: Vec3) -> SwingTick {
    SwingTick {
        at: SwingAt {
            now_ms: 10_000,
            motion: Some(motion),
            progress,
            last_key: 100.0,
            base: 1.0,
        },
        motion,
        feral: 0.0,
        energy: 1.0,
        weapon_class: class,
        strikers: [Some(v), Some(Vec3::ZERO)],
        aim: Vec3::Z,
    }
}

/// The window opens past 30 % of a side attack: with the fists only the
/// motion's hand (the right on the main side, the left on the other,
/// which becomes the primary striker), with a weapon every striker; past
/// 40 % of a stab, 50 % of an overhead (both claws in a clawed one).
#[test]
fn the_window_opens_by_motion() {
    let swing = Swing::FISTS;
    for (motion, class, at, open, primary) in [
        (Main(0), 0x80, 0.3, [false, false], None),
        (Main(0), 0x80, 0.31, [true, false], Some(Slot::First)),
        (Other(2), 0x80, 0.31, [false, true], Some(Slot::Second)),
        (Main(1), 0x10, 0.31, [true, true], None),
        (Stab, 0x10, 0.39, [false, false], None),
        (Stab, 0x10, 0.41, [true, true], None),
        (Overhead, 0x80, 0.49, [false, false], None),
        (Overhead, 0x80, 0.51, [true, false], Some(Slot::First)),
        (Overhead, 0xC0, 0.51, [true, true], Some(Slot::First)),
    ] {
        let mut s = SwingState::default();
        let mut p = None;
        assert_eq!(
            s.tick(&swing, &tick_at(motion, at, class, Vec3::ZERO), &mut p),
            SwingEvent::Plays
        );
        assert_eq!((s.open, p), (open, primary), "{motion:?} {class:#x} {at}");
        assert_eq!(s.has(flags::WINDOW_OPENED), open[0] || open[1]);
    }
}

/// The overhead's window closes past 90 % with nothing in hand (class 0),
/// past 95 % otherwise, and at once past 85 % once the swing no longer
/// falls 3 cm a tick; the stab's past 80 %, or past 95 % while its thrust
/// drives on at 2.5 cm a tick along the aim. A side attack's stays open.
#[test]
fn the_window_closes_late_in_overheads_and_stabs() {
    let swing = Swing::FISTS;
    let shut = |motion, p: f32, class, v: Vec3, slot_velocity: Vec3| {
        let mut s = SwingState {
            flags: flags::WINDOW_OPENED,
            open: [true; 2],
            ..SwingState::default()
        };
        s.tracker[0].velocity = slot_velocity;
        s.tracker[0].best = slot_velocity.length_squared();
        let mut primary = None;
        s.tick(&swing, &tick_at(motion, p, class, v), &mut primary);
        !s.open[0]
    };
    let falling = Vec3::new(0.0, -10.0, 0.0);
    assert!(!shut(Overhead, 0.89, 0, falling, falling));
    assert!(shut(Overhead, 0.91, 0, falling, falling));
    assert!(!shut(Overhead, 0.86, 0x10, falling, falling));
    assert!(shut(Overhead, 0.96, 0x10, falling, falling));
    assert!(shut(Overhead, 0.86, 0x10, Vec3::ZERO, Vec3::ZERO));
    let thrust = Vec3::Z * 3.0;
    assert!(!shut(Stab, 0.79, 0x10, Vec3::ZERO, Vec3::ZERO));
    assert!(shut(Stab, 0.81, 0x10, Vec3::ZERO, Vec3::ZERO));
    assert!(!shut(Stab, 0.9, 0x10, thrust, thrust));
    assert!(shut(Stab, 0.96, 0x10, thrust, thrust));
    assert!(!shut(Main(0), 0.99, 0x10, Vec3::ZERO, Vec3::ZERO));
}

/// Remise: with the perk (no 0x80000) a side attack held through 80 % of
/// its clip chains (not at 0.8 itself); without it, released, stalling or
/// already chained past its carried window, never; nor a stab or an
/// overhead.
#[test]
fn remise_chains_past_80_percent() {
    let swing = Swing::FISTS;
    let event = |flags: u32, motion, p| {
        let mut s = SwingState {
            flags,
            ..SwingState::default()
        };
        let mut primary = None;
        s.tick(&swing, &tick_at(motion, p, 0x80, Vec3::ZERO), &mut primary)
    };
    assert_eq!(event(0, Main(0), 0.8), SwingEvent::Plays);
    assert_eq!(event(0, Main(0), 0.81), SwingEvent::Chains);
    assert_eq!(event(0, Other(1), 0.81), SwingEvent::Chains);
    for f in [
        flags::NO_CHAIN,
        flags::RELEASED,
        flags::CONTINUED,
        flags::STALLING,
    ] {
        assert_ne!(event(f, Main(0), 0.81), SwingEvent::Chains, "{f:#x}");
    }
    assert_eq!(event(0, Stab, 0.9), SwingEvent::Plays);
    assert_eq!(event(0, Overhead, 0.9), SwingEvent::Plays);
    // A start without Remise sets 0x80000 for a side attack only.
    let mut s = SwingState::default();
    s.start(true, false);
    assert_eq!(s.flags, flags::NO_CHAIN);
    s.start(false, false);
    assert_eq!(s.flags, 0);
    s.start(true, true);
    assert_eq!(s.flags, 0);
}

/// A chained swing keeps its window and trackers up to 25 % of its clip;
/// past it its state becomes 0x20000 alone (no further chain), the window
/// shuts, then opens again past 30 %.
#[test]
fn a_chained_swing_carries_its_window() {
    let swing = Swing::FISTS;
    let mut s = SwingState {
        flags: flags::WINDOW_OPENED | flags::LATE,
        open: [true, false],
        ..SwingState::default()
    };
    s.chain();
    assert!(s.has(flags::CONTINUATION) && s.strength == 1.0);
    let mut primary = Some(Slot::First);
    let v = Vec3::X * 8.0;
    s.tick(&swing, &tick_at(Other(0), 0.1, 0x80, v), &mut primary);
    assert_eq!(s.open, [true, false]);
    assert!(s.tracker[0].speed2 > 0.0, "carried window tracks");
    s.tick(&swing, &tick_at(Other(0), 0.26, 0x80, v), &mut primary);
    assert_eq!((s.flags, s.open), (flags::CONTINUED, [false, false]));
    s.tick(&swing, &tick_at(Other(0), 0.31, 0x80, v), &mut primary);
    assert_eq!(s.open, [false, true]);
    assert_eq!(primary, Some(Slot::Second));
    assert!(s.has(flags::CONTINUED | flags::WINDOW_OPENED));
    // Past 80 % it does not chain again.
    let mut p = None;
    assert_eq!(
        s.tick(&swing, &tick_at(Other(0), 0.85, 0x80, v), &mut p),
        SwingEvent::Plays
    );
}

/// While a swing stalls each tick takes the stall off its strength and
/// the clip runs at rate × P; once its end has passed the stance returns
/// and its state clears. Tired (0.9 of its energy) the rate is 0.81 even
/// while it stalls.
#[test]
fn a_stall_drains_the_swing() {
    let swing = Swing::FISTS;
    let mut s = SwingState {
        flags: flags::STALLING | flags::RELEASED | flags::NO_CHAIN,
        stall: 0.3,
        end_ms: 10_036,
        ..SwingState::default()
    };
    let mut primary = None;
    let mut t = tick_at(Main(0), 0.05, 0x80, Vec3::ZERO);
    let mut rates = Vec::new();
    for k in 0..4u64 {
        t.at.now_ms = 10_000 + 12 * k;
        let e = s.tick(&swing, &t, &mut primary);
        rates.push((s.rate, e));
    }
    let expect = [
        (1.62 * 0.7, SwingEvent::Plays),
        (1.62 * 0.4, SwingEvent::Plays),
    ];
    for (k, (r, e)) in expect.iter().enumerate() {
        assert!(close(rates[k].0, *r) && rates[k].1 == *e, "{k}: {rates:?}");
    }
    // 10 024 ms: P ≈ 0.1; 10 036: not past the end yet; P 0.
    assert!(rates[2].0 < 0.17 && rates[2].1 == SwingEvent::Plays);
    assert_eq!(rates[3], (0.0, SwingEvent::Plays));
    t.at.now_ms = 10_048;
    assert_eq!(s.tick(&swing, &t, &mut primary), SwingEvent::Ends);
    assert_eq!(s.flags, 0);
    let mut tired = SwingState {
        flags: flags::STALLING,
        stall: 0.5,
        end_ms: 20_000,
        ..SwingState::default()
    };
    let mut t = tick_at(Main(0), 0.5, 0x80, Vec3::ZERO);
    t.energy = 0.9;
    tired.tick(&swing, &t, &mut primary);
    assert!(close(tired.rate, 0.81));
}

/// The tracker caps a striker's speed at 50 cm a tick up to 80 % of the
/// clip and moves a quarter of the way to it a tick; after 80 % the speed
/// is capped at the slot's best and the best only shrinks; outside the
/// window region (before 30 %) it is cleared.
#[test]
fn the_tracker_follows_the_striker() {
    let mut t = Tracker::default();
    t.track(Vec3::X * 80.0, 0.5);
    assert!(close(t.velocity.x, 12.5));
    assert!(close(t.speed2, 156.25) && close(t.best, 156.25));
    t.track(Vec3::X * 10.0, 0.6);
    assert!(close(t.velocity.x, 11.875));
    // The latest speed caps the slot's.
    assert!(close(t.speed2, 100.0));
    t.track(Vec3::X * 40.0, 0.85);
    assert!(t.best <= 141.02 && t.speed2 <= t.best);
    let swing = Swing::FISTS;
    let mut s = SwingState::default();
    s.tracker[0] = t;
    let mut primary = None;
    s.tick(&swing, &tick_at(Main(0), 0.2, 0x80, Vec3::X), &mut primary);
    assert_eq!(
        (s.tracker[0].velocity, s.tracker[0].speed2),
        (Vec3::ZERO, 0.0)
    );
}

/// While attacking the body turns at (1 − 0.6·enc)(1 − 0.25·p)·B·W16·P/
/// scale a tick, B = 4π/3 / 83: the fists (W16 0.8) at full strength at
/// the swing's start 0.0404 rad a tick (192°/s), half way 0.0353; half
/// strength or encumbrance 0.5 lower.
#[test]
fn the_attack_turns_the_body_by_the_weapon() {
    let f = Swing::FISTS;
    assert!(close(f.turn_rate(0.0, 1.0, 0.0, 1.0), 0.050_467_35 * 0.8));
    assert!(close(
        f.turn_rate(0.5, 1.0, 0.0, 1.0),
        0.050_467_35 * 0.8 * 0.875
    ));
    assert!(close(f.turn_rate(0.0, 0.5, 0.0, 1.0), 0.050_467_35 * 0.4));
    assert!(close(
        f.turn_rate(0.0, 1.0, 0.5, 1.0),
        0.050_467_35 * 0.8 * 0.7
    ));
    assert!(close(f.turn_rate(0.0, 1.0, 0.0, 2.0), 0.050_467_35 * 0.4));
}

/// A chained attack starts with round(6·W18) + 2 frames of blend: 2 for
/// the fists, 4 for the 0.3/0.6 sword (2.4 rounds to 2), 3 for Sword A
/// (1.2 → 1).
#[test]
fn a_chain_blends_by_the_shaping() {
    assert_eq!(Swing::FISTS.chain_blend(), 2);
    assert_eq!(Swing::of(&sword()).chain_blend(), 4);
    assert_eq!(sword_a().chain_blend(), 3);
}

/// A fighter swings with its primary striker: the right fist before any
/// attack, the left after an other-side attack made it primary (a
/// Harvester's left claw swings slower than its right fist).
#[test]
fn the_primary_striker_paces_the_swing() {
    let mut app = App::new();
    app.add_systems(Update, pace_swings);
    let fighter = app.world_mut().spawn(Attacker::default()).id();
    let hand = |section: &Section| {
        WeaponRecord::build(section, false, Lengths::hand(8.0, 9.66), &Holder::AVERAGE)
    };
    let (fist, claw) = (hand(&Section::FISTS), hand(&Section::HEAVY_CLAWS));
    app.world_mut()
        .spawn(Blade::new(fighter, 0.18, fist).in_slot(Slot::First));
    app.world_mut()
        .spawn(Blade::new(fighter, 0.18, claw).in_slot(Slot::Second));
    app.update();
    let swing = |app: &App| *app.world().get::<Swing>(fighter).unwrap();
    assert_eq!(swing(&app), Swing::of(&fist));
    app.world_mut()
        .get_mut::<Attacker>(fighter)
        .unwrap()
        .primary = Some(Slot::Second);
    app.update();
    assert_eq!(swing(&app), Swing::of(&claw));
    assert_ne!(Swing::of(&claw), Swing::of(&fist));
}

/// A synthetic motion set: the "Default" moveset's attacks and stances,
/// 100 frames each.
fn motions() -> Motions {
    use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
    let key = |time| Key {
        time,
        root: [0.0; 3],
        rotations: Vec::new(),
        flags: Vec::new(),
    };
    let clip = Clip {
        mode: 0,
        keys: vec![key(0.0), key(100.0)],
    };
    let actions = [
        "Attack Mid",
        "Attack Down",
        "Attack Up",
        "Attack Combo 1",
        "Attack Combo 2",
        "Attack Combo 3",
        "Cmbt - Idle",
    ];
    let set = MotionSet {
        bone_count: 0,
        actions: actions
            .iter()
            .map(|n| Action {
                name: (*n).into(),
                params: [0; 2],
            })
            .collect(),
        movesets: vec![Moveset {
            name: "Default".into(),
            parent: String::new(),
            clips: vec![Some(clip); actions.len()],
        }],
    };
    Motions::new(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0)
}

/// An unarmed fighter attacking with Attack Mid at clock `time` (of 100
/// frames), its condition attacking, at body tick 1000 (12 000 ms), the
/// swing routine running once per update on it as the ticked character.
fn attacking(time: f32, state: SwingState) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(BodyTicks(1000))
        .insert_resource(motions())
        .add_systems(Update, run_swings);
    let mut anim = Animator::new("Default", "Cmbt - Idle");
    anim.play_once("Attack Mid", "Cmbt - Idle");
    anim.time = time;
    let fighter = app
        .world_mut()
        .spawn((
            Fighter::default(),
            anim,
            Attacking::new(Main(0), 0x80),
            Condition::ATTACKING,
            GlobalTransform::IDENTITY,
            Swing::FISTS,
        ))
        .id();
    *app.world_mut().get_mut::<SwingState>(fighter).unwrap() = state;
    app.insert_resource(TickedCharacter {
        body: fighter,
        character: None,
    });
    (app, fighter)
}

/// Each attack tick stamps the game clock as the last attack tick and sets
/// the clip's rate; a fighter not attacking (its condition otherwise, or
/// its attack ended) is left alone.
#[test]
fn the_routine_stamps_and_paces_each_attack_tick() {
    let (mut app, fighter) = attacking(50.0, SwingState::default());
    app.update();
    let w = app.world();
    assert_eq!(
        w.get::<Attacker>(fighter).unwrap().last_tick_ms,
        Some(12_000)
    );
    assert!(close(w.get::<SwingState>(fighter).unwrap().rate, 1.62));
    assert!(w.get::<SwingState>(fighter).unwrap().open[0]);
    app.world_mut().resource_mut::<BodyTicks>().0 = 1001;
    *app.world_mut().get_mut::<Condition>(fighter).unwrap() = Condition::WEAK;
    app.update();
    assert_eq!(
        app.world().get::<Attacker>(fighter).unwrap().last_tick_ms,
        Some(12_000)
    );
}

/// Off its balance (under 0.6, in double precision, so the float 0.6 is
/// on it) an attacking fighter takes the falls branch: its attack's clip
/// runs 1.25 × base (1.875 for a ghoul's 1.5). So does an attack started
/// off balance before its condition became attacking (normal, combat
/// idle, weak, down); a collapse, death, a shock or a get-up keep their
/// own branches.
#[test]
fn off_balance_the_attack_runs_the_falls_rate() {
    assert_eq!(off_balance_rate(0.59, 1.0), Some(1.25));
    assert_eq!(off_balance_rate(0.0, 1.5), Some(1.875));
    assert_eq!(off_balance_rate(0.6, 1.0), None);
    assert_eq!(off_balance_rate(1.0, 1.0), None);
    for c in [
        Condition::ATTACKING,
        Condition::NORMAL,
        Condition::COMBAT_IDLE,
        Condition::WEAK,
        Condition::DOWN,
    ] {
        assert!(falls_off_balance(c), "{c:?}");
    }
    for c in [
        Condition::COLLAPSING,
        Condition::DEAD,
        Condition::SHOCKED,
        Condition::GETTING_UP,
    ] {
        assert!(!falls_off_balance(c), "{c:?}");
    }
}

/// Remise: held through 80 % (clock 81 of 100) a fighter with the perk
/// chains one of the other side's attacks at once, with a blend of
/// round(6 × 0) + 2 = 2 frames, eased smoothly, ending in the same stance;
/// its swing is a continuation at full strength and its attack the new
/// motion.
#[test]
fn remise_chains_the_other_side() {
    let (mut app, fighter) = attacking(81.0, SwingState::default());
    app.update();
    let w = app.world();
    let anim = w.get::<Animator>(fighter).unwrap();
    assert!(anim.action.starts_with("Attack Combo"), "{}", anim.action);
    assert_eq!(
        (anim.blend, anim.ease, anim.then.as_deref(), anim.looping),
        (2.0, Ease::Smooth, Some("Cmbt - Idle"), false)
    );
    let state = w.get::<SwingState>(fighter).unwrap();
    assert!(state.has(flags::CONTINUATION));
    assert!(matches!(
        w.get::<Attacking>(fighter).unwrap().motion,
        Other(_)
    ));
    // Without the perk (0x80000 from its start) it plays on.
    let mut no_remise = SwingState::default();
    no_remise.start(true, false);
    let (mut app, fighter) = attacking(81.0, no_remise);
    app.update();
    assert_eq!(
        app.world().get::<Animator>(fighter).unwrap().action,
        "Attack Mid"
    );
}

/// A released swing whose stall is over returns to its stance (as it is
/// now: the guard if the fighter was dazed meanwhile) with twice its
/// recovery as blend (the fists' 0), eased smoothly, and the fighter stops
/// attacking.
#[test]
fn a_stall_ends_in_the_stance() {
    let state = SwingState {
        flags: flags::STALLING | flags::RELEASED,
        stall: 0.3,
        end_ms: 11_988,
        open: [true, false],
        ..SwingState::default()
    };
    let (mut app, fighter) = attacking(20.0, state);
    app.update();
    let w = app.world();
    let anim = w.get::<Animator>(fighter).unwrap();
    assert_eq!(
        (anim.action.as_str(), anim.looping, anim.blend, anim.ease),
        ("Cmbt - Idle", true, 0.0, Ease::Smooth)
    );
    assert!(w.get::<Attacking>(fighter).is_none());
    let s = w.get::<SwingState>(fighter).unwrap();
    assert_eq!((s.flags, s.open), (0, [false; 2]));
    // Dazed during the swing, its stance is now the guard: the guard returns.
    let (mut app, fighter) = attacking(20.0, state);
    app.world_mut()
        .entity_mut(fighter)
        .insert(crate::stance::Stance::Defence);
    app.update();
    let anim = app.world().get::<Animator>(fighter).unwrap();
    assert_eq!(anim.action, "Defence stance");
}

/// The installed motion set's attack clips, which the swing clock's
/// numbers rest on (skipped without the game): the fists' punches (the
/// "Default" moveset's Attack Mid and Attack Combo 1) end at 110 frames,
/// so a held punch plays 68 ticks (816 ms), passes 0.8 of its clip, where
/// Remise chains it, after 55, and is expected to take 849 ms from its
/// start; a one-handed Attack Mid ends at 100 frames, a greatsword's at
/// 110.
#[test]
fn the_installed_attack_clips() {
    use exa_anim::MovesetConfig as C;
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let rsq = game
        .pack(exa_rpk::PackKind::Resource)
        .and_then(|p| p.get("humanoid.rsq"))
        .unwrap();
    let seq = exa_formats::rsq::Sequences::parse(rsq).unwrap();
    let motions = Motions::new(
        std::sync::Arc::new(seq.motions),
        std::sync::Arc::default(),
        0.0,
    );
    let last_key = |config: C, action: &str| {
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.config = config;
        anim.play_once(action, "Cmbt - Idle");
        let clip = motions.clip(&anim, action).unwrap();
        clip.keys.last().unwrap().time
    };
    assert_eq!(last_key(C::NONE, "Attack Mid"), 110.0);
    assert_eq!(last_key(C::NONE, "Attack Combo 1"), 110.0);
    assert_eq!(last_key(C::ONE_HANDED, "Attack Mid"), 100.0);
    assert_eq!(last_key(C::GREATSWORD, "Attack Mid"), 110.0);
    let punch = last_key(C::NONE, "Attack Mid");
    let fists = Swing::FISTS;
    let (mut t, mut ticks, mut chain_at) = (0.0f32, 0, None);
    while t / punch < 1.0 {
        if chain_at.is_none() && t / punch > CHAIN_AT {
            chain_at = Some(ticks);
        }
        t += fists.rate(false, t / punch, 1.0, 0.0);
        ticks += 1;
    }
    assert_eq!((ticks, ticks * 12, chain_at), (68, 816, Some(55)));
    assert_eq!(fists.expected_ms(0.0, punch, 1.0), 849);
}

fn blocked() -> RecoilStrike {
    RecoilStrike {
        class_word: 0,
        flags: flags::WINDOW_OPENED | flags::RELEASED,
        clock: 0.0,
        counter: false,
        drive: 1.0,
        now_ms: 1000,
    }
}

fn wound() -> FollowStrike {
    FollowStrike {
        class_word: 0,
        flags: flags::WINDOW_OPENED,
        clock: 0.0,
        counter: false,
        gain: 1.0,
        recoil: 1.0,
        speed: 1.0,
        zone: 0.25,
        edge: false,
        now_ms: 1000,
    }
}

#[test]
fn a_striker_recoils_and_follows_through() {
    let hit = recoil(&blocked());
    assert_eq!(hit.strength.to_bits(), 0x3f00_0000);
    assert_eq!(hit.stall.to_bits(), 0x3f00_0000);
    assert_eq!(hit.flags, 0x27);
    let rest = hit.rest.unwrap();
    assert_eq!(rest.end_ms, 1240);
    assert_eq!(rest.blend, 40);
    assert_eq!(rest.stiffness.to_bits(), 0x3e4c_cccd);
    assert_eq!(rest.held.to_bits(), 0x3f80_0000);
    let short = recoil(&RecoilStrike {
        drive: 13.0 * 0.001,
        ..blocked()
    });
    assert_eq!(short.rest.unwrap().end_ms, 1200);
    let continued = recoil(&RecoilStrike {
        class_word: 0x90,
        flags: flags::CONTINUATION | flags::RELEASED,
        ..blocked()
    });
    assert_eq!(continued.strength.to_bits(), 0x3f00_0000);
    assert_eq!(continued.stall.to_bits(), 0x3f00_0000);
    assert_eq!(continued.flags, flags::CONTINUATION);
    assert!(continued.rest.is_none());
    let late = recoil(&RecoilStrike {
        class_word: 0x90,
        flags: flags::RELEASED,
        clock: 0.4,
        counter: true,
        ..blocked()
    });
    assert_eq!(late.flags, 0);
    assert!(late.rest.is_none());
    let open = recoil(&RecoilStrike {
        class_word: 0x90,
        clock: 0.4,
        counter: false,
        ..blocked()
    });
    assert_eq!(open.flags, 0x27);
    assert_eq!(open.rest.unwrap().end_ms, 1240);
    let earlier = recoil(&RecoilStrike {
        class_word: 0x90,
        clock: f32::from_bits(0x3ecc_cccc),
        counter: true,
        ..blocked()
    });
    assert_eq!(earlier.rest.unwrap().end_ms, 1240);

    let body = follow_through(&wound()).unwrap();
    assert_eq!(body.strength.to_bits(), 0x3d99_999a);
    assert_eq!(body.stall.to_bits(), 0x3d3d_70a4);
    assert_eq!(body.flags, 0x7);
    assert_eq!(body.end_ms, 1060);
    let empty = follow_through(&FollowStrike {
        zone: 0.0,
        ..wound()
    })
    .unwrap();
    assert_eq!(empty.strength.to_bits(), 0);
    assert_eq!(empty.stall.to_bits(), 0x3d4c_cccd);
    let floor = follow_through(&FollowStrike {
        zone: f32::from_bits(0x33ff_ffd7),
        ..wound()
    })
    .unwrap();
    assert_eq!(floor.strength.to_bits(), 0);
    let edge = follow_through(&FollowStrike {
        edge: true,
        ..wound()
    })
    .unwrap();
    assert_eq!(edge.strength.to_bits(), 0x3eb9_999a);
    assert_eq!(edge.stall.to_bits(), 0x3d02_8f5c);
    let capped = follow_through(&FollowStrike {
        gain: 100.0,
        zone: 1.0,
        ..wound()
    })
    .unwrap();
    assert_eq!(capped.strength.to_bits(), 0x3f80_0000);
    assert_eq!(capped.stall.to_bits(), 0);
    assert_eq!(capped.end_ms, 7000);
    let dropped = follow_through(&FollowStrike {
        gain: -1.0,
        ..wound()
    })
    .unwrap();
    assert_eq!(dropped.strength.to_bits(), 0);
    assert_eq!(dropped.end_ms, 1000);
    let glancing = follow_through(&FollowStrike {
        speed: 9.0 * 0.001,
        zone: 0.0,
        ..wound()
    })
    .unwrap();
    assert_eq!(glancing.end_ms, 1000);
    assert!(
        follow_through(&FollowStrike {
            class_word: 0x90,
            flags: flags::CONTINUATION,
            ..wound()
        })
        .is_none()
    );
}
