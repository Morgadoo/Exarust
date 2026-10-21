//! Tests of the combat state.
//! (Included from combat.rs: `#[path = "combat_test.rs"] mod tests;`)

use super::*;

const NOVICE: Traits = Traits {
    skill: 0.0,
    blend: 1.0,
};
const MASTER: Traits = Traits {
    skill: 1.0,
    blend: 1.0,
};

/// A field that scores every place the same, but for the strike, which
/// is 1 within `strike_within` cm of the foe at `foe`.
struct Flat {
    foe: Vec3,
    approach: f32,
    strike_within: f32,
    chased: bool,
    blocked: bool,
}

impl Flat {
    fn new(foe: Vec3) -> Self {
        Self {
            foe,
            approach: 0.0,
            strike_within: 0.0,
            chased: false,
            blocked: false,
        }
    }
}

impl Field for Flat {
    fn blocked(&mut self, _from: Vec3, _to: Vec3) -> bool {
        self.blocked
    }
    fn score(&mut self, _contact: usize, level: i32, point: Vec3) -> PlaceScore {
        let to = self.foe - point;
        let d = (to.x * to.x + to.z * to.z).sqrt();
        PlaceScore {
            approach: self.approach,
            strike: if level == 3 && d <= self.strike_within {
                1.0
            } else {
                0.0
            },
            bearing: yaw(to),
            distance_cm: d,
            ..PlaceScore::default()
        }
    }
    fn chase(&mut self) -> bool {
        self.chased = true;
        true
    }
}

fn foe_contact(level: i32) -> Contact {
    Contact {
        level,
        is_foe: true,
        character: true,
        ..Contact::default()
    }
}

/// The next value of the engine's generator from `seed`.
fn next(seed: u32) -> u32 {
    seed.wrapping_mul(0x0808_8405).wrapping_add(1)
}

#[test]
fn angles_wrap_in_double_and_yaws_use_the_quick_arccosine() {
    // The f32 π is past the double π, so it wraps to −π.
    assert_eq!(wrap(PI_F32), PI_F32 - TWO_PI);
    assert_eq!(wrap(3.14159), 3.14159);
    assert_eq!(wrap(-4.0), -4.0 + TWO_PI);
    assert_eq!(quick_acos(0.0), HALF_PI);
    assert_eq!(quick_acos(1.0), 0.0);
    assert_eq!(quick_acos(-1.0), std::f64::consts::PI as f32);
    assert_eq!(yaw(Vec3::Z), 0.0);
    assert_eq!(yaw(Vec3::X), HALF_PI);
    assert_eq!(yaw(-Vec3::X), -HALF_PI);
    assert_eq!(yaw(-Vec3::Z), PI_F32);
    assert_eq!(yaw(Vec3::new(1e-5, 0.0, 0.0)), 0.0);
}

#[test]
fn practice_pays_four_per_point_past_ten() {
    let mut p = Practice {
        lesson: 1.0,
        gain: 9.95,
        fatigue: 0.0,
    };
    assert_eq!(practise(&mut p), 40);
    assert!((p.gain - 0.05).abs() < 1e-5, "{}", p.gain);
    assert_eq!(p.lesson, (0.995f64) as f32);
    assert_eq!(p.fatigue, (f64::from(0.0001f64 as f32) * 0.9998) as f32);
    // A small lesson only rests the fatigue.
    let mut q = Practice {
        lesson: 0.005,
        gain: 3.0,
        fatigue: 0.5,
    };
    assert_eq!(practise(&mut q), 0);
    assert_eq!((q.lesson, q.gain), (0.005, 3.0));
    assert_eq!(q.fatigue, (0.5f64 * 0.9998) as f32);
    // The fatigue stops at 0.9.
    let mut r = Practice {
        lesson: 2.0,
        gain: 0.0,
        fatigue: 0.8999,
    };
    practise(&mut r);
    assert_eq!(
        r.fatigue,
        (f64::from(f32::from_bits(0x3f66_6666)) * 0.9998) as f32
    );
}

#[test]
fn entering_sets_the_eagerness_at_a_half() {
    let mut mind = CombatMind {
        eagerness: 3.0,
        pressing: true,
        ..CombatMind::default()
    };
    assert!(enter(&mut mind, false, 1234));
    assert_eq!(
        (mind.eagerness, mind.pressing, mind.entered_ms),
        (0.5, false, 1234)
    );
    mind.eagerness = 2.0;
    assert!(!enter(&mut mind, true, 9999));
    assert_eq!(mind.eagerness, 2.0);
}

#[test]
fn the_estimate_blends_back_by_skill() {
    let me = Body::default();
    let mut mind = CombatMind {
        estimate: Vec3::new(100.0, 0.0, 0.0),
        ..CombatMind::default()
    };
    // Seed 0 draws 2.3e-10, under the novice's 0.04.
    let mut roll = 0;
    let b = refresh_estimate(&mut mind, &me, Vec3::new(0.0, 0.0, 300.0), 0.0, &mut roll).unwrap();
    let x = 100.0 * 0.6f32;
    assert_eq!(
        mind.estimate,
        Vec3::new(x, 0.0, 300.0 + (0.0 - 300.0) * 0.6f32)
    );
    let e = mind.estimate;
    let d = (e.x * e.x + e.z * e.z).sqrt();
    assert_eq!(b.distance_cm, d);
    assert_eq!(b.to_target, e * (1.0 / d));
    // A master looks every tick and keeps the new estimate.
    let mut roll = 0x7fff_ffff;
    let b = refresh_estimate(&mut mind, &me, Vec3::new(0.0, 400.0, 300.0), 1.0, &mut roll).unwrap();
    assert_eq!(mind.estimate, Vec3::new(0.0, 400.0, 300.0));
    assert_eq!(b.distance_cm, 800.0);
    // Within 1 cm the distance is 1 and the direction is left as it is.
    let b = refresh_estimate(&mut mind, &me, Vec3::new(0.5, 0.0, 0.0), 1.0, &mut roll).unwrap();
    assert_eq!(
        (b.distance_cm, b.to_target),
        (1.0, Vec3::new(0.5, 0.0, 0.0))
    );
    // A novice draw of 0.5 does not look again.
    let mut roll = (0..u32::MAX)
        .find(|s| f64::from(next(*s)) / 4_294_967_296.0 > 0.5)
        .unwrap();
    assert!(refresh_estimate(&mut mind, &me, Vec3::ZERO, 0.0, &mut roll).is_none());
}

#[test]
fn a_running_or_stepping_foe_is_led() {
    let mut foe = Foe {
        chest: Vec3::new(0.0, 100.0, 0.0),
        step_goal: Vec3::new(40.0, 0.0, 0.0),
        stepping: true,
        ..Foe::default()
    };
    assert_eq!(foe_estimate(&foe, 1.0), Vec3::new(30.0, 100.0, 0.0));
    foe.running = true;
    foe.forward_speed = 2.0;
    assert_eq!(foe_estimate(&foe, 1.0), Vec3::new(0.0, 100.0, 60.0));
}

#[test]
fn bare_hands_draw_a_carried_weapon() {
    let mut me = Body {
        carried: 1,
        ..Body::default()
    };
    assert_eq!(draw_weapon(&me), Some(true));
    me.requested_motion = DRAW_WEAPON;
    assert_eq!(draw_weapon(&me), Some(false));
    me.requested_motion = 0;
    me.motion = DRAW_WEAPON;
    assert_eq!(draw_weapon(&me), Some(false));
    me.motion = 0;
    me.carried = 0;
    assert_eq!(draw_weapon(&me), None);
    me.weapon_class = 0x20;
    me.held = 1;
    me.carried = 1;
    assert_eq!(draw_weapon(&me), None);
}

#[test]
fn one_tick_in_three_hundred_lunges() {
    let me = Body {
        reach_cm: 90.0,
        ..Body::default()
    };
    let mut mind = CombatMind {
        level: 2,
        ..CombatMind::default()
    };
    // Seed 0's draw of 300 is 0.
    let mut roll = 0;
    lunge(&mut mind, &me, &mut roll);
    assert!(mind.lunging);
    assert_eq!(mind.lunge_reach_cm, 170.0);
    mind.level = 1;
    lunge(&mut mind, &me, &mut roll);
    assert!(!mind.lunging);
}

#[test]
fn the_chase_range_follows_the_foe() {
    let me = Body::default();
    let lying = Foe {
        condition: 0x200,
        ..Foe::default()
    };
    let mut roll = 0;
    assert_eq!(chase_range(&me, Some(&lying), &mut roll), 400.0);
    assert_eq!(roll, 0);
    let running = Foe {
        running: true,
        ..Foe::default()
    };
    assert_eq!(chase_range(&me, Some(&running), &mut roll), 200.0);
    let me_running = Body {
        running: true,
        ..Body::default()
    };
    let mut roll = 0;
    assert_eq!(chase_range(&me_running, None, &mut roll), 450.0);
}

#[test]
fn a_far_foe_is_chased_after_21_stuck_ticks() {
    let me = Body::default();
    let mut field = Flat::new(Vec3::ZERO);
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        ..CombatMind::default()
    };
    let mut roll = 0;
    for n in 1..=21 {
        let c = chase_and_eagerness(
            &mut mind, &me, None, 900.0, None, NOVICE, &mut field, &mut roll,
        );
        assert_eq!((c, mind.stuck), (Chase::Stay, n));
    }
    let c = chase_and_eagerness(
        &mut mind, &me, None, 900.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!(c, Chase::Started);
    assert!(field.chased);
    assert_eq!(mind.alert, 0.5);
    // Close again, the count resets.
    let c = chase_and_eagerness(
        &mut mind, &me, None, 100.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!((c, mind.stuck), (Chase::Stay, 0));
    // An unreachable foe only counts.
    let mut contact = Contact {
        unreachable: true,
        ..foe_contact(3)
    };
    mind.stuck = 30;
    field.blocked = true;
    field.chased = false;
    chase_and_eagerness(
        &mut mind,
        &me,
        None,
        100.0,
        Some(&mut contact),
        NOVICE,
        &mut field,
        &mut roll,
    );
    assert_eq!(mind.stuck, 31);
    assert!(!field.chased);
}

#[test]
fn the_eagerness_drifts_with_energy() {
    let mut field = Flat::new(Vec3::ZERO);
    let me = Body::default();
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        ..CombatMind::default()
    };
    // Seed 0: the range draw, then a 2.3e-10 eagerness draw.
    let mut roll = 0;
    chase_and_eagerness(
        &mut mind, &me, None, 100.0, None, NOVICE, &mut field, &mut roll,
    );
    let r = f64::from(next(next(0))) / 4_294_967_296.0;
    let r = f64::from(r as f32);
    assert_eq!(mind.eagerness, ((r * 0.02 - 0.006) * 3.0 + 0.5) as f32);
    // Above 6 it falls by 0.01.
    mind.eagerness = 6.5;
    chase_and_eagerness(
        &mut mind, &me, None, 100.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!(mind.eagerness, (6.5f64 - 0.01) as f32);
    // At half energy it is scaled by 0.75 and the press stops.
    let tired = Body {
        energy: 0.5,
        ..Body::default()
    };
    mind.eagerness = 2.0;
    mind.pressing = true;
    chase_and_eagerness(
        &mut mind, &tired, None, 100.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!((mind.eagerness, mind.pressing), (1.5, false));
    // Off balance, or not engaged, it is 0.
    let wobbly = Body {
        balance: 0.85,
        ..Body::default()
    };
    chase_and_eagerness(
        &mut mind, &wobbly, None, 100.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!(mind.eagerness, 0.0);
}

#[test]
fn the_drives_of_a_fresh_pressing_fighter() {
    let me = Body::default();
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        pressing: true,
        ..CombatMind::default()
    };
    // Seed 1's draw of 40 is 1: the press holds.
    let mut roll = 1;
    let d = drives(&mut mind, &me, None, NOVICE, &mut roll);
    // Fists double the pressed attack once more.
    assert_eq!((d.attack, d.strike, d.spacing), (4.0, 1.5, 6.0));
    assert!(!d.committed);
    // Short of energy: halved attack, four times the spacing.
    let tired = Body {
        energy: 0.5,
        now_ms: 10_000,
        ..Body::default()
    };
    mind.pressing = true;
    let mut roll = 1;
    let d = drives(&mut mind, &tired, None, NOVICE, &mut roll);
    assert_eq!((d.attack, d.strike, d.spacing), (0.125, 0.125, 24.0));
    // Below the engage level there is no attack.
    mind.level = 2;
    let d = drives(&mut mind, &me, None, NOVICE, &mut roll);
    assert_eq!((d.attack, d.strike), (0.0, 0.0));
    assert!(!mind.pressing);
}

#[test]
fn a_swing_commits_and_measures_its_overshoot() {
    let me = Body {
        condition: ATTACKING,
        clock: 0.55,
        target_point: Vec3::new(0.0, 0.0, 100.0),
        weapon_class: 0x10,
        ..Body::default()
    };
    let foe = Foe {
        lesson: 0.3,
        ..Foe::default()
    };
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        lunge_reach_cm: 130.0,
        ..CombatMind::default()
    };
    let mut roll = 0;
    let d = drives(&mut mind, &me, Some(&foe), NOVICE, &mut roll);
    assert!(d.committed && mind.pressing && mind.measured);
    assert_eq!(mind.overshoot, 60.0);
    assert_eq!(mind.overshoot_mean, 6.0);
    // Past 0.6 of the clock the eagerness was not spent yet (0.55).
    assert_eq!(mind.eagerness, 0.5);
    // Spacing (0·2 + 2)·2 = 4; attack ((0.5 − 0)·2 + 2)·2 = 6, then the
    // press at full energy doubles it.
    assert_eq!((d.attack, d.strike, d.spacing), (12.0, 1.5, 4.0));
    // Late in the swing the eagerness is spent and the foe's lesson kept.
    let late = Body { clock: 0.65, ..me };
    // A seed whose draw is past a half.
    let seed = (0..u32::MAX)
        .find(|s| f64::from(next(*s)) / 4_294_967_296.0 > 0.5)
        .unwrap();
    let mut roll = seed;
    let d = drives(&mut mind, &late, Some(&foe), NOVICE, &mut roll);
    assert_eq!(d.set_flags, 0x10_0000);
    assert_eq!(mind.practice.lesson, 0.3);
    // It spends (1 − age/2)·(0.5 − 0.2·skill)·draw.
    let r = f64::from((f64::from(next(seed)) / 4_294_967_296.0) as f32);
    assert_eq!(mind.eagerness, (0.5 - 1.0 * 0.5 * r) as f32);
    // The overshoot stays within −50 and 100.
    mind.measured = false;
    mind.overshoot = 90.0;
    let mut roll = 0;
    drives(&mut mind, &me, Some(&foe), NOVICE, &mut roll);
    assert_eq!(mind.overshoot, 100.0);
}

#[test]
fn the_press_ramps_in_over_three_seconds() {
    let me = Body {
        now_ms: 3000,
        ..Body::default()
    };
    let mut mind = CombatMind {
        level: 3,
        entered_ms: 0,
        ..CombatMind::default()
    };
    let mut roll = 0;
    drives(&mut mind, &me, None, NOVICE, &mut roll);
    assert!(mind.pressing);
    assert_eq!(mind.entered_ms, 3000);
}

#[test]
fn footwork_weighs_more_steps_with_skill() {
    let me = Body::default();
    let d = Drives {
        attack: 0.0,
        strike: 0.0,
        spacing: 0.0,
        committed: false,
        set_flags: 0,
    };
    let mut mind = CombatMind::default();
    let mut roll = 0;
    let plan = footwork(&mut mind, &me, &d, 1.0, &mut roll);
    assert_eq!((plan.first, plan.last, plan.mode), (0, 16, 0));
    assert_eq!(plan.change, 5.0);
    let big = Body {
        scale: 1.3,
        ..Body::default()
    };
    let plan = footwork(&mut mind, &big, &d, 1.0, &mut roll);
    assert_eq!(plan.last, 10);
    let over = 1.3f32 - 1.0;
    let s = over * over * 10.0 + 1.0;
    assert_eq!(plan.change, (5.0 * f64::from(s)) as f32);
    let held = Body {
        can_step: false,
        ..Body::default()
    };
    assert_eq!(footwork(&mut mind, &held, &d, 1.0, &mut roll).last, 0);
    // A queued step is taken up, with the cost of the change.
    let queued = Body {
        queue: -2,
        stepping: true,
        step_started_ms: 0,
        now_ms: 100,
        ..Body::default()
    };
    mind.queued_step = 3;
    let plan = footwork(&mut mind, &queued, &d, 1.0, &mut roll);
    assert!(plan.queue_taken);
    assert_eq!((mind.step, mind.queued_step, mind.step_cost), (3, 0, 8.0));
    assert_eq!((plan.first, plan.mode), (1, 1));
    let later = Body {
        now_ms: 301,
        ..queued
    };
    assert_eq!(footwork(&mut mind, &later, &d, 1.0, &mut roll).mode, 2);
    let second = Body {
        step_state: 2,
        ..queued
    };
    assert_eq!(footwork(&mut mind, &second, &d, 1.0, &mut roll).mode, 3);
}

#[test]
fn the_eight_directions() {
    let side = side_of(Vec3::Z);
    assert!((side - Vec3::X).length() < 1e-3);
    let look = Vec3::Z;
    let s = Vec3::X;
    assert_eq!(
        step_offset(1, look, s, 80.0),
        Some(Vec3::new(0.0, 0.0, 80.0))
    );
    assert_eq!(
        step_offset(2, look, s, 80.0),
        Some(Vec3::new(0.0, 0.0, -80.0))
    );
    assert_eq!(
        step_offset(3, look, s, 80.0),
        Some(Vec3::new(80.0, 0.0, 0.0))
    );
    assert_eq!(
        step_offset(4, look, s, 80.0),
        Some(Vec3::new(-80.0, 0.0, 0.0))
    );
    let diag = 80.0 / 2.0f32.sqrt();
    let five = step_offset(5, look, s, 80.0).unwrap();
    assert!((five - Vec3::new(diag, 0.0, diag)).length() < 1e-4);
    let eight = step_offset(8, look, s, 80.0).unwrap();
    assert!((eight - Vec3::new(diag, 0.0, -diag)).length() < 1e-4);
    assert_eq!(step_offset(0, look, s, 80.0), None);
}

#[test]
fn a_step_keeps_off_the_foe() {
    // Within 60 cm × scale, a step backs out to the edge only when the
    // distance squared is under the radius (the original compares the two).
    let mut v = Vec3::new(0.0, 0.0, 80.0);
    keep_off(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 1.0, &mut v);
    assert_eq!(v, Vec3::new(0.0, 0.0, 60.0));
    let mut v = Vec3::new(0.0, 0.0, 80.0);
    keep_off(Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO, 1.0, &mut v);
    assert_eq!(v, Vec3::ZERO);
    // Outside, a step heading in is cut at the circle only when the
    // original's test vector agrees: along z it does not.
    let mut v = Vec3::new(0.0, 0.0, -200.0);
    keep_off(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO, 1.0, &mut v);
    assert_eq!(v, Vec3::new(0.0, 0.0, -200.0));
    // A step that misses the circle is untouched.
    let mut v = Vec3::new(200.0, 0.0, 0.0);
    keep_off(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO, 1.0, &mut v);
    assert_eq!(v, Vec3::new(200.0, 0.0, 0.0));
}

#[test]
fn friends_in_the_way_hold_the_strike() {
    let ahead = Bystander {
        position: Vec3::new(0.0, 0.0, 100.0),
        stepping: false,
        step_goal: Vec3::ZERO,
        out: false,
        is_foe: false,
        concern_hostility: None,
    };
    assert_eq!(friend_in_the_way(Vec3::ZERO, Vec3::Z, 50.0, &[ahead]), 2);
    let aside = Bystander {
        position: Vec3::new(100.0, 0.0, 0.0),
        ..ahead
    };
    assert_eq!(friend_in_the_way(Vec3::ZERO, Vec3::Z, 50.0, &[aside]), 1);
    let behind = Bystander {
        position: Vec3::new(0.0, 0.0, -100.0),
        ..ahead
    };
    assert_eq!(friend_in_the_way(Vec3::ZERO, Vec3::Z, 50.0, &[behind]), 0);
    let far = Bystander {
        position: Vec3::new(0.0, 0.0, 131.0),
        ..ahead
    };
    assert_eq!(friend_in_the_way(Vec3::ZERO, Vec3::Z, 50.0, &[far]), 0);
    let hostile = Bystander {
        concern_hostility: Some(1.9),
        ..ahead
    };
    assert_eq!(friend_in_the_way(Vec3::ZERO, Vec3::Z, 50.0, &[hostile]), 0);
    let calm = Bystander {
        concern_hostility: Some(1.8),
        ..ahead
    };
    assert_eq!(
        friend_in_the_way(Vec3::ZERO, Vec3::Z, 50.0, &[calm, aside]),
        2
    );
}

#[test]
fn the_swing_sweep_follows_the_clock() {
    let mind = CombatMind::default();
    let at = |clock: f32, class: u32| {
        swing_sweep(
            &Body {
                clock,
                weapon_class: class,
                ..Body::default()
            },
            &mind,
        )
    };
    assert_eq!(at(0.0, 0x10), -80.0);
    assert_eq!(at(0.1, 0x10), -60.0);
    assert_eq!(
        at(0.35, 0x10),
        ((f64::from(0.35f32) - 0.2) * 300.0 - 60.0) as f32
    );
    assert_eq!(at(0.6, 0x10), 30.0);
    // Open hands turn 50° less; a heavy claw 50° more.
    assert_eq!(at(0.6, 0), -20.0);
    let claw = Body {
        clock: 0.6,
        heavy_claw: true,
        ..Body::default()
    };
    assert_eq!(swing_sweep(&claw, &mind), 80.0);
    let lunging = CombatMind {
        lunging: true,
        ..CombatMind::default()
    };
    assert_eq!(swing_sweep(&claw, &lunging), 120.0);
}

/// A fighter at the origin facing its foe 100 cm ahead.
fn duel() -> (Body, Foe) {
    let me = Body {
        now_ms: 5000,
        chest: Vec3::new(0.0, 120.0, 0.0),
        position: Vec3::new(0.0, 90.0, 0.0),
        target_point: Vec3::new(0.0, 0.0, 100.0),
        to_target: Vec3::Z,
        distance_cm: 100.0,
        weapon_class: 0x10,
        held: 1,
        swing_amount: Some(1.0),
        ..Body::default()
    };
    let foe = Foe {
        chest: Vec3::new(0.0, 120.0, 100.0),
        ..Foe::default()
    };
    (me, foe)
}

#[test]
fn an_unreachable_strike_place_leaves_the_fighter_on_guard() {
    let (me, foe) = duel();
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        ..CombatMind::default()
    };
    let mut contacts = [foe_contact(3)];
    let mut field = Flat::new(foe.chest);
    let mut roll = 1;
    let t = tick(
        &mut mind,
        &me,
        NOVICE,
        World {
            foe: Some(&foe),
            focus: None,
            contacts: &mut contacts,
            around: &[],
            field: &mut field,
            leader_calls: false,
        },
        &mut roll,
    );
    assert_eq!(t.exit, None);
    // No approach and no strike: every place costs its ground, so it stays
    // where it stands, guarding, and turns toward its foe.
    assert_eq!(t.chosen, Some((0, false)));
    assert_eq!(t.step, Footstep::Stay);
    assert!(t.blow.motion.is_none());
    assert!(t.blow.clear_flags);
    assert_eq!(t.blow.guard, None);
    assert_eq!(mind.aim, 0.0);
    assert_eq!(t.look, Some(of_yaw(mind.heading)));
}

#[test]
fn a_foe_in_reach_is_struck_from_where_it_stands() {
    let (me, foe) = duel();
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        pressing: true,
        ..CombatMind::default()
    };
    let mut contacts = [foe_contact(3)];
    let mut field = Flat::new(foe.chest);
    field.approach = 2.0;
    field.strike_within = 120.0;
    let mut roll = 1;
    let t = tick(
        &mut mind,
        &me,
        NOVICE,
        World {
            foe: Some(&foe),
            focus: None,
            contacts: &mut contacts,
            around: &[],
            field: &mut field,
            leader_calls: false,
        },
        &mut roll,
    );
    assert_eq!(t.exit, None);
    let (chosen, strikes) = t.chosen.unwrap();
    assert!(strikes);
    // A forward strike place gives way to the place it stands on.
    assert!(!matches!(chosen, 1 | 5 | 6));
    // The request and side follow the blow.
    match t.blow.motion.unwrap() {
        AttackMotion::Overhead => assert_eq!(mind.request, 8),
        AttackMotion::Stab => assert_eq!(mind.request, 0x10),
        AttackMotion::Other(_) => assert_eq!((mind.request, mind.side), (0, 2)),
        AttackMotion::Main(_) => assert_eq!((mind.request, mind.side), (0, 0)),
    }
    // The swing turn is (draw + 0.5) × the weapon's amount of 1.
    assert!((0.5..1.5).contains(&mind.swing_turn));
}

/// A field where a bystander contact is close and dangerous everywhere.
struct Crowded;

impl Field for Crowded {
    fn score(&mut self, _contact: usize, _level: i32, _point: Vec3) -> PlaceScore {
        PlaceScore {
            close: 1.0,
            danger: 0.5,
            ..PlaceScore::default()
        }
    }
}

#[test]
fn every_place_costs_its_ground_its_danger_and_its_closeness() {
    let me = Body::default();
    let mut mind = CombatMind::default();
    let d = Drives {
        attack: 0.0,
        strike: 0.0,
        spacing: 0.0,
        committed: false,
        set_flags: 0,
    };
    let plan = Footwork {
        first: 0,
        last: 16,
        mode: 0,
        change: 5.0,
        queue_taken: false,
    };
    let mut contacts = [Contact {
        level: 1,
        character: true,
        ..Contact::default()
    }];
    let found = search(
        &mut mind,
        &me,
        None,
        MASTER,
        plan,
        &d,
        &mut contacts,
        &mut Crowded,
    );
    // Ground 1, danger 0.5 × 3, closeness 1 × 2; where it stands the
    // contact is weighed as the foe, whose closeness is added once.
    assert_eq!(found.places[0].total, 3.5);
    assert_eq!(found.places[0].danger, 0.5);
    assert_eq!(found.places[1].total, 4.5);
    assert_eq!(found.places[9].total, 4.5);
    // Long steps past the first two directions cost 2 more.
    assert_eq!(found.places[11].total, 6.5);
    assert_eq!(found.best_move, Some(0));
    assert_eq!(found.best_attack, None);
    assert_eq!(found.places[3].attack, 4.5);
    // The place it stands on re-chose its foe: the one contact, at level 1.
    assert_eq!(found.retarget, Some(Some(0)));
    assert_eq!(mind.level, 1);
    // Its priority: the danger weighted by (ahead + 1)·0.4 and 2·skill + 1,
    // plus a tenth of its level.
    let w = (f64::from(1.0f32) * 0.4) as f32;
    let v = 0.0 * 1.0 + ((1.0 + 1.0) * 0.0 + 3.0 * 0.5 * w);
    assert_eq!(contacts[0].priority, (0.1 + f64::from(v)) as f32);
}

#[test]
fn a_friend_in_the_way_stops_the_strike() {
    let (me, foe) = duel();
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        pressing: true,
        ..CombatMind::default()
    };
    let mut contacts = [foe_contact(3)];
    let mut field = Flat::new(foe.chest);
    field.approach = 2.0;
    field.strike_within = 500.0;
    let friend = Bystander {
        position: Vec3::new(0.0, 120.0, 60.0),
        stepping: false,
        step_goal: Vec3::ZERO,
        out: false,
        is_foe: false,
        concern_hostility: None,
    };
    let mut roll = 1;
    let t = tick(
        &mut mind,
        &me,
        NOVICE,
        World {
            foe: Some(&foe),
            focus: None,
            contacts: &mut contacts,
            around: &[friend],
            field: &mut field,
            leader_calls: false,
        },
        &mut roll,
    );
    assert_eq!(t.chosen.map(|c| c.1), Some(false));
    assert!(t.blow.motion.is_none());
}

#[test]
fn early_exits() {
    let (me, foe) = duel();
    let mut mind = CombatMind::default();
    let mut field = Flat::new(foe.chest);
    let mut roll = 0;
    let mut run = |me: &Body, foe: Option<&Foe>, leader: bool| {
        let mut contacts = [foe_contact(3)];
        tick(
            &mut mind,
            me,
            MASTER,
            World {
                foe,
                focus: None,
                contacts: &mut contacts,
                around: &[],
                field: &mut field,
                leader_calls: leader,
            },
            &mut roll,
        )
        .exit
    };
    assert_eq!(run(&me, Some(&foe), true), Some(Exit::Follow));
    let down = Body {
        condition: DOWN,
        ..me
    };
    assert_eq!(run(&down, Some(&foe), false), Some(Exit::Down));
    let dead = Foe { dead: true, ..foe };
    assert_eq!(
        run(&me, Some(&dead), false),
        Some(Exit::Leave {
            look_ms: 8000,
            next_ms: 5500
        })
    );
    let charging = Body {
        running: true,
        forward_speed: 3.5,
        ..me
    };
    assert_eq!(run(&charging, Some(&foe), false), Some(Exit::Charge));
}

#[test]
fn the_guard_leans_against_a_side_swing() {
    let (me, mut foe) = duel();
    foe.motion = 5;
    foe.reach_cm = 100.0;
    let mut mind = CombatMind {
        level: 3,
        ..CombatMind::default()
    };
    let mut contacts = [foe_contact(3)];
    let mut field = Flat::new(foe.chest);
    let mut roll = 1;
    tick(
        &mut mind,
        &me,
        MASTER,
        World {
            foe: Some(&foe),
            focus: None,
            contacts: &mut contacts,
            around: &[],
            field: &mut field,
            leader_calls: false,
        },
        &mut roll,
    );
    // 100 cm is past the foe's reach − 70: the lean is (150 − 100)/120.
    let lean = (150.0f32 - 100.0) * f32::from_bits(0x3c08_8889);
    assert_eq!(mind.aim, wrap(0.0 + lean * 25.0 * DEGREE));
    let g = gap(mind.aim, 0.0);
    assert_eq!(mind.heading, wrap(0.0 + (f64::from(g) * 0.03) as f32));
}

#[test]
fn a_fighting_follower_answers_its_leader() {
    let leader = Leader {
        position: Vec3::ZERO,
        facing: Vec3::Z,
        sprinting: false,
        running: false,
    };
    // The band eases toward 700 cm: 600 → 601 (in double).
    let mut f = Following {
        band: 600.0,
        following: true,
        route_to: Some(Vec3::ZERO),
    };
    let near = follow_check(Vec3::new(0.0, 0.0, 300.0), &leader, &mut f);
    assert_eq!(f.band, (600.0f64 * 0.99 + 700.0 * 0.01) as f32);
    assert_eq!(
        near,
        FollowCheck {
            stop: true,
            ..FollowCheck::default()
        }
    );
    // Past 0.7 of the band a follower with a route answers; three bands off
    // it runs; a leader 100 cm from the route's end asks for a new route.
    let far = follow_check(Vec3::new(0.0, 0.0, 2000.0), &leader, &mut f);
    assert_eq!((far.calls, far.replan, far.urgency), (true, false, 2));
    f.route_to = Some(Vec3::new(101.0, 0.0, 0.0));
    let mid = follow_check(Vec3::new(0.0, 0.0, 500.0), &leader, &mut f);
    assert_eq!((mid.calls, mid.replan, mid.urgency), (true, true, 1));
    // Not yet following, it plans; within the band it does nothing.
    let mut g = Following {
        band: 700.0,
        following: false,
        route_to: None,
    };
    let start = follow_check(Vec3::new(0.0, 0.0, 800.0), &leader, &mut g);
    assert_eq!(
        (start.calls, start.replan, start.stop),
        (false, true, false)
    );
    let idle = follow_check(Vec3::new(0.0, 0.0, 600.0), &leader, &mut g);
    assert_eq!(idle, FollowCheck::default());
    // A sprinting leader pulls the band toward 500 + 100 × ahead: behind it
    // (ahead −1), 400.
    let sprint = Leader {
        sprinting: true,
        ..leader
    };
    let mut h = Following {
        band: 400.0,
        following: false,
        route_to: None,
    };
    follow_check(Vec3::new(0.0, 0.0, -300.0), &sprint, &mut h);
    assert_eq!(h.band, (400.0f64 * 0.8 + 400.0 * 0.2) as f32);
}
