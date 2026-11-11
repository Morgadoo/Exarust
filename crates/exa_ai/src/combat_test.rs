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
    fn chase(&mut self, _estimate: Vec3, _roll: &mut u32) -> bool {
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
fn set_target_places_the_estimate_and_commits() {
    let mut mind = CombatMind {
        commitment: 0.25,
        overshoot_mean: 3.0,
        stuck: 7,
        fresh_target: false,
        aim: 2.0,
        ..CombatMind::default()
    };
    let b = set_target(
        &mut mind,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::ZERO,
        Vec3::ZERO,
    );
    assert!(mind.fresh_target);
    assert_eq!(mind.commitment, 1.0);
    assert_eq!((mind.overshoot_mean, mind.stuck), (0.0, 0));
    assert_eq!(mind.estimate, Vec3::new(100.0, 0.0, 0.0));
    assert_eq!(
        (b.distance_cm, b.to_target),
        (100.0, Vec3::new(1.0, 0.0, 0.0))
    );
    // The aim is the bearing's yaw: +x is a quarter turn.
    assert_eq!(mind.aim, yaw(Vec3::X));
    // No 300 cm height rule and no 1 cm floor: 500 cm up and 30 cm ahead
    // is 30 cm, the direction's height scaled by the flat length.
    let b = set_target(
        &mut mind,
        Vec3::new(0.0, 500.0, 30.0),
        Vec3::ZERO,
        Vec3::ZERO,
    );
    assert_eq!(b.distance_cm, 30.0);
    assert_eq!(b.to_target, Vec3::new(0.0, 500.0 * (1.0 / 30.0), 1.0));
    assert_eq!(mind.aim, 0.0);
    // On the fighter's own spot the previous direction stays.
    let b = set_target(&mut mind, Vec3::Y, Vec3::ZERO, Vec3::X);
    assert_eq!((b.distance_cm, b.to_target), (0.0, Vec3::X));
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
    // Within 1 cm the distance is 1 and the direction the last estimate
    // left is kept (the original does not write it).
    let me_close = Body {
        to_target: Vec3::new(-1.0, 0.0, 0.0),
        ..Body::default()
    };
    let b = refresh_estimate(
        &mut mind,
        &me_close,
        Vec3::new(0.5, 0.0, 0.0),
        1.0,
        &mut roll,
    )
    .unwrap();
    assert_eq!(
        (b.distance_cm, b.to_target),
        (1.0, Vec3::new(-1.0, 0.0, 0.0))
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
fn a_requested_motion_starts_only_when_the_body_is_free() {
    let free = RequestGate {
        muscles: true,
        fallen: false,
        busy: false,
        moving: false,
        free_walk: false,
    };
    assert_eq!(take_requested(DRAW_WEAPON, free), (0, true));
    assert_eq!(take_requested(0, free), (0, false));
    assert_eq!(
        take_requested(
            DRAW_WEAPON,
            RequestGate {
                muscles: false,
                ..free
            }
        ),
        (DRAW_WEAPON, false)
    );
    assert_eq!(
        take_requested(
            DRAW_WEAPON,
            RequestGate {
                fallen: true,
                ..free
            }
        ),
        (DRAW_WEAPON, false)
    );
    // Busy and standing: the request is dropped and nothing starts.
    assert_eq!(
        take_requested(DRAW_WEAPON, RequestGate { busy: true, ..free }),
        (0, false)
    );
    // Busy and walking toward a goal: kept, still not started.
    assert_eq!(
        take_requested(
            DRAW_WEAPON,
            RequestGate {
                busy: true,
                moving: true,
                ..free
            }
        ),
        (DRAW_WEAPON, false)
    );
    // A free walk drops it even while moving.
    assert_eq!(
        take_requested(
            DRAW_WEAPON,
            RequestGate {
                busy: true,
                moving: true,
                free_walk: true,
                ..free
            }
        ),
        (0, false)
    );
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
fn the_watched_point_sits_on_the_foes_ground() {
    let chest = Vec3::new(120.0, 190.0, -40.0);
    assert_eq!(target_point(chest, 60.0), Vec3::new(120.0, 60.0, -40.0));
}

#[test]
fn a_foe_half_a_metre_up_is_not_chased() {
    // The foe stands 60 cm higher (a rock), 378 cm away; its chest is
    // 130 cm over its feet. The chase check measures the watched point's
    // height against the fighter's ground: 60 cm, so it stays, where the
    // chest's 190 cm would have counted it as out of reach.
    let chest = Vec3::new(378.0, 190.0, 0.0);
    let me = Body {
        ground_y: 0.0,
        target_point: target_point(chest, 60.0),
        ..Body::default()
    };
    let mut field = Flat::new(chest);
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        stuck: 5,
        ..CombatMind::default()
    };
    let mut roll = 0;
    let c = chase_and_eagerness(
        &mut mind, &me, None, 378.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!((c, mind.stuck), (Chase::Stay, 0));
    // Watching the chest instead counts the foe as out of reach.
    let high = Body {
        target_point: chest,
        ..me
    };
    chase_and_eagerness(
        &mut mind, &high, None, 378.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!(mind.stuck, 1);
    // More than 150 cm of ground between them is a chase.
    let cliff = Body {
        target_point: target_point(chest, 151.0),
        ..me
    };
    mind.stuck = 21;
    let c = chase_and_eagerness(
        &mut mind, &cliff, None, 378.0, None, NOVICE, &mut field, &mut roll,
    );
    assert_eq!(c, Chase::Started);
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
fn a_recoil_holds_the_recovery_drive() {
    let at = |until: u32| {
        let me = Body {
            now_ms: 1000,
            recoil_until_ms: until,
            weapon_class: 0x10,
            ..Body::default()
        };
        let mut mind = CombatMind {
            level: 3,
            eagerness: 0.5,
            pressing: true,
            ..CombatMind::default()
        };
        // Seed 1's draw of 40 is 1, so a recovered fighter keeps the press.
        let mut roll = 1;
        drives(&mut mind, &me, None, MASTER, &mut roll)
    };
    // Still before the deadline the strike is half and the attack loses
    // half a skill. At the deadline it has recovered.
    assert_eq!((at(1500).attack, at(1500).strike), (1.5, 0.75));
    assert_eq!((at(1000).attack, at(1000).strike), (3.0, 1.5));
}

#[test]
fn a_large_foes_claws_quadruple_the_attack() {
    let me = Body {
        weapon_class: 0x10,
        ..Body::default()
    };
    let creature = |class: u32| Foe {
        rules: 1,
        weapon_class: class,
        ..Foe::default()
    };
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        pressing: true,
        ..CombatMind::default()
    };
    // Seed 1's draw of 40 is 1: the press holds.
    let mut roll = 1;
    let fists = drives(&mut mind, &me, Some(&creature(FISTS)), NOVICE, &mut roll);
    mind.pressing = true;
    roll = 1;
    let claws = drives(&mut mind, &me, Some(&creature(CLAWS)), NOVICE, &mut roll);
    assert_eq!((fists.attack, fists.strike), (4.0, 1.5));
    assert_eq!((claws.attack, claws.strike), (8.0, 1.5));
}

#[test]
fn a_cleaving_foe_doubles_the_drive() {
    let me = Body::default();
    let foe = Foe {
        attack_flags: exa_combat::swing::flags::CLEAVING,
        ..Foe::default()
    };
    let mut mind = CombatMind {
        level: 3,
        eagerness: 0.5,
        pressing: true,
        ..CombatMind::default()
    };
    // Seed 1's draw of 40 is 1: the press holds.
    let mut roll = 1;
    let open = drives(&mut mind, &me, None, NOVICE, &mut roll);
    mind.pressing = true;
    roll = 1;
    let cleaving = drives(&mut mind, &me, Some(&foe), NOVICE, &mut roll);
    assert_eq!((open.attack, open.strike), (4.0, 1.5));
    assert_eq!((cleaving.attack, cleaving.strike), (8.0, 3.0));
}

#[test]
fn a_recoiled_swing_is_not_driven_as_a_live_one() {
    let live = Body {
        condition: ATTACKING,
        clock: 0.55,
        weapon_class: 0x10,
        ..Body::default()
    };
    let recoiled = Body {
        attack_flags: exa_combat::swing::flags::RECOILED,
        ..live
    };
    let fresh = || CombatMind {
        level: 3,
        eagerness: 0.5,
        pressing: true,
        ..CombatMind::default()
    };
    let mut live_mind = fresh();
    let mut recoiled_mind = fresh();
    let mut roll = 0;
    let open = drives(&mut live_mind, &live, None, NOVICE, &mut roll);
    roll = 0;
    let held = drives(&mut recoiled_mind, &recoiled, None, NOVICE, &mut roll);
    // Still early in the clip, a live swing commits. A recoil leaves that
    // drive for the recovery one.
    assert!(open.committed);
    assert_eq!((open.attack, open.strike), (12.0, 1.5));
    assert!(!held.committed);
    assert_eq!((held.attack, held.strike), (2.0, 0.75));
}

#[test]
fn a_held_shield_quadruples_the_committed_attack() {
    let bare = Body {
        condition: ATTACKING,
        clock: 0.55,
        weapon_class: 0x10,
        ..Body::default()
    };
    let shielded = Body {
        shield: true,
        ..bare
    };
    let fresh = || CombatMind {
        level: 3,
        eagerness: 0.5,
        ..CombatMind::default()
    };
    let mut bare_mind = fresh();
    let mut shielded_mind = fresh();
    let mut roll = 0;
    let open = drives(&mut bare_mind, &bare, None, NOVICE, &mut roll);
    roll = 0;
    let held = drives(&mut shielded_mind, &shielded, None, NOVICE, &mut roll);
    // Past 0.3 the attack drive is doubled, or quadrupled with a shield,
    // and the shield doubles the strike drive. The press then scales both.
    assert!(open.committed && held.committed);
    assert_eq!((open.attack, open.strike), (12.0, 1.5));
    assert_eq!((held.attack, held.strike), (24.0, 3.0));
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
    // Along x the test vector is zero, its cosine not a number, and the
    // original's unordered compare cuts the step at the circle (t = 0.7).
    let mut v = Vec3::new(-200.0, 0.0, 0.0);
    keep_off(Vec3::new(200.0, 0.0, 0.0), Vec3::ZERO, 1.0, &mut v);
    assert_eq!(v, Vec3::new(-200.0 * (56_000.0f32 / 80_000.0), 0.0, 0.0));
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
fn a_high_parry_memory_picks_the_other_side() {
    let place = Place {
        distance_cm: 100.0,
        ..Place::default()
    };
    let foe = Foe {
        chest: Vec3::new(0.0, 0.0, 100.0),
        ..Foe::default()
    };
    let blow = |bias: i32| {
        let me = Body {
            parry_bias: bias,
            weapon_class: 0x10,
            reach_cm: 50.0,
            scale: 1.0,
            ..Body::default()
        };
        let mut mind = CombatMind::default();
        let mut roll = 22;
        choose_blow(&mut mind, &me, Some(&foe), 0.0, 0, &place, &mut roll).motion
    };
    // The same draw scores under 4 with no memory, and over 4 once the
    // memory has risen to 20, which is the other side.
    assert_ne!(blow(0), Some(AttackMotion::Other(0)));
    assert_eq!(blow(20), Some(AttackMotion::Other(0)));
}

#[test]
fn a_quadruped_thrusts_where_a_biped_swings() {
    let foe = Foe {
        chest: Vec3::new(0.0, 0.0, 100.0),
        ..Foe::default()
    };
    let place = Place {
        distance_cm: 100.0,
        ..Place::default()
    };
    let blow = |quadruped: bool| {
        let me = Body {
            quadruped,
            weapon_class: 0x10,
            reach_cm: 50.0,
            scale: 1.0,
            ..Body::default()
        };
        let mut mind = CombatMind::default();
        let mut roll = 0;
        choose_blow(&mut mind, &me, Some(&foe), 0.0, 0, &place, &mut roll).motion
    };
    // A hundred centimetres is beyond eight tenths of a 50 cm reach. The
    // quadruped's draw thrusts; the biped, open-handed, swings overhead.
    assert_eq!(blow(true), Some(AttackMotion::Stab));
    assert_eq!(blow(false), Some(AttackMotion::Overhead));
}

#[test]
fn the_attack_bonus_holds_a_forward_step_open() {
    let places = vec![
        Place {
            attack: 10.0,
            ..Place::default()
        },
        Place::default(),
    ];
    let plan = Footwork {
        first: 0,
        last: 8,
        mode: 0,
        change: 1.0,
        queue_taken: false,
    };
    let kept = |bonus: f32| {
        let me = Body {
            condition: ATTACKING,
            clock: 0.33,
            attack_bonus: bonus,
            size: 1.0,
            ..Body::default()
        };
        decide_attack(&me, 1.0, plan, &places, Some(1), 0.0, 0.0, false, &[]).1
    };
    // Skill 1 and no bonus: the gate is 0.3, and 0.33 is past it, so the
    // forward step stands. The Fael's 0.2 stretches the gate to 0.36, and
    // the step gives way.
    assert_eq!(kept(0.0), Some(1));
    assert_eq!(kept(0.2), Some(0));
}

#[test]
fn a_long_step_is_scaled_by_the_leg_extension() {
    let me = Body {
        now_ms: 500,
        step_started_ms: 0,
        stepping: true,
        step_cm: 80.0,
        look: Vec3::Z,
        chest: Vec3::ZERO,
        ..Body::default()
    };
    let continued = |factor: f32| {
        let mut mind = CombatMind {
            long_step: true,
            step: 1,
            ..CombatMind::default()
        };
        let mut field = Flat::new(Vec3::ZERO);
        let (step, _) = take_step(
            &mut mind,
            &Body {
                step_factor: factor,
                ..me
            },
            None,
            Footwork {
                first: 0,
                last: 0,
                mode: 0,
                change: 1.0,
                queue_taken: false,
            },
            0,
            Vec3::ZERO,
            &mut field,
        );
        match step {
            Footstep::Continue { goal, step_cm } => (goal, step_cm),
            other => panic!("expected the long step's second half, got {other:?}"),
        }
    };
    let (full, full_cm) = continued(1.0);
    let (half, half_cm) = continued(0.5);
    // Unladen, the second half is one and a half times the step, times the
    // extension: 120 cm, or 60 at half extension.
    assert_eq!(full_cm, 120.0);
    assert_eq!(half_cm, 60.0);
    assert_eq!(full, Vec3::new(0.0, 0.0, 120.0));
    assert_eq!(half, Vec3::new(0.0, 0.0, 60.0));
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
        &mut 0,
    );
    // Ground 1, danger 0.5 × 3, closeness 1 × 2; where it stands the
    // contact is weighed as the foe, whose closeness is added once.
    assert_eq!(found.places[0].total, 3.5);
    assert_eq!(found.places[0].danger, 0.5);
    // The standing place takes this contact as its foe, so the steps after
    // add its closeness once, as a foe's, rather than twice.
    assert_eq!(found.places[1].total, 3.5);
    assert_eq!(found.places[9].total, 3.5);
    // Long steps past the first two directions cost 2 more.
    assert_eq!(found.places[11].total, 5.5);
    assert_eq!(found.best_move, Some(0));
    assert_eq!(found.best_attack, None);
    assert_eq!(found.places[3].attack, 2.5);
    // The place it stands on re-chose its foe: the one contact, at level 1.
    assert_eq!(found.retarget, Some(Some(0)));
    assert_eq!(found.switched, Some(0));
    assert!(contacts[0].is_foe);
    assert_eq!(mind.level, 1);
    // Its priority: the danger weighted by (ahead + 1)·0.4 and 2·skill + 1,
    // plus a tenth of its level.
    let w = (f64::from(1.0f32) * 0.4) as f32;
    let v = 0.0 * 1.0 + ((1.0 + 1.0) * 0.0 + 3.0 * 0.5 * w);
    assert_eq!(contacts[0].priority, (0.1 + f64::from(v)) as f32);
}

#[test]
fn a_step_change_costs_less_once_a_foot_has_landed() {
    let places = |landings: i32| {
        let me = Body {
            stepping: true,
            landings,
            ..Body::default()
        };
        let mut mind = CombatMind {
            step: 1,
            step_cost: 4.0,
            ..CombatMind::default()
        };
        let plan = Footwork {
            first: 0,
            last: 8,
            mode: 0,
            change: 5.0,
            queue_taken: false,
        };
        let drives = Drives {
            attack: 0.0,
            strike: 0.0,
            spacing: 0.0,
            committed: false,
            set_flags: 0,
        };
        let mut contacts = [];
        search(
            &mut mind,
            &me,
            None,
            NOVICE,
            plan,
            &drives,
            &mut contacts,
            &mut Crowded,
            &mut 0,
        )
        .places
    };
    let airborne = places(0);
    let landed = places(1);
    // The direction it is already stepping is not a change.
    assert_eq!(airborne[1].total, 1.0);
    assert_eq!(landed[1].total, 1.0);
    // Another direction pays the change, and the step cost until a foot lands.
    assert_eq!(airborne[2].total, 21.0);
    assert_eq!(landed[2].total, 6.0);
}

/// A nearer character replaces the foe at the standing place, and the steps
/// after that are scored against the new one, so the step goes toward them.
struct Beside;

impl Field for Beside {
    fn score(&mut self, contact: usize, level: i32, point: Vec3) -> PlaceScore {
        let origin = if contact == 0 {
            Vec3::new(0.0, 0.0, 800.0)
        } else {
            Vec3::new(150.0, 0.0, 0.0)
        };
        let to = origin - point;
        let d = (to.x * to.x + to.z * to.z).sqrt();
        PlaceScore {
            approach: if contact == 1 && level == 3 { 3.0 } else { 0.0 },
            strike: if contact == 1 && level == 3 && point.x > 40.0 {
                1.0
            } else {
                0.0
            },
            bearing: yaw(to),
            distance_cm: d,
            ..PlaceScore::default()
        }
    }
}

#[test]
fn a_nearer_character_is_the_foe_for_the_steps_after() {
    let me = Body {
        look: Vec3::Z,
        heading: Vec3::Z,
        facing: Vec3::Z,
        chest: Vec3::ZERO,
        position: Vec3::ZERO,
        step_cm: 80.0,
        ..Body::default()
    };
    let mut mind = CombatMind::default();
    let d = Drives {
        attack: 0.0,
        strike: 5.0,
        spacing: 0.0,
        committed: false,
        set_flags: 0,
    };
    let plan = Footwork {
        first: 0,
        last: 8,
        mode: 0,
        change: 0.0,
        queue_taken: false,
    };
    let mut contacts = [
        Contact {
            level: 3,
            is_foe: true,
            character: true,
            estimate: Some(Vec3::new(0.0, 0.0, 800.0)),
            ..Contact::default()
        },
        Contact {
            level: 3,
            character: true,
            estimate: Some(Vec3::new(150.0, 0.0, 0.0)),
            ..Contact::default()
        },
    ];
    let found = search(
        &mut mind,
        &me,
        None,
        NOVICE,
        plan,
        &d,
        &mut contacts,
        &mut Beside,
        &mut 0,
    );
    assert_eq!(found.switched, Some(1));
    assert!(!contacts[0].is_foe);
    assert!(contacts[1].is_foe);
    let step = found.best_move.expect("a step");
    assert!(
        found.places[step].offset.x > 0.0,
        "place {step} totals {:?} offset {:?}",
        found.places.iter().map(|p| p.total).collect::<Vec<_>>(),
        found.places[step].offset
    );
}

#[test]
fn a_downed_character_just_taken_is_struck_overhead() {
    let (mut me, foe) = duel();
    me.weapon_class = 0x10;
    me.look = Vec3::Z;
    me.heading = Vec3::Z;
    me.step_cm = 80.0;
    let mut mind = CombatMind {
        level: 3,
        eagerness: 6.0,
        pressing: true,
        ..CombatMind::default()
    };
    let mut contacts = [
        Contact {
            level: 3,
            is_foe: true,
            character: true,
            estimate: Some(Vec3::new(0.0, 0.0, 800.0)),
            posture: Some(Posture {
                condition: 0,
                reach_cm: 50.0,
                running: false,
                crouch: 0.0,
                motion: 0,
            }),
            ..Contact::default()
        },
        Contact {
            level: 3,
            character: true,
            estimate: Some(Vec3::new(150.0, 0.0, 0.0)),
            posture: Some(Posture {
                condition: DOWN,
                reach_cm: 40.0,
                running: false,
                crouch: 0.0,
                motion: 0,
            }),
            ..Contact::default()
        },
    ];
    let mut field = Beside;
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
    assert_eq!(t.blow.motion, Some(AttackMotion::Overhead));
}

#[test]
fn a_crouched_character_just_taken_is_struck_overhead() {
    let (mut me, foe) = duel();
    me.weapon_class = 0x10;
    me.look = Vec3::Z;
    me.heading = Vec3::Z;
    me.step_cm = 80.0;
    let mut mind = CombatMind {
        level: 3,
        eagerness: 6.0,
        pressing: true,
        ..CombatMind::default()
    };
    let mut contacts = [
        Contact {
            level: 3,
            is_foe: true,
            character: true,
            estimate: Some(Vec3::new(0.0, 0.0, 800.0)),
            posture: Some(Posture {
                condition: 0,
                reach_cm: 50.0,
                running: false,
                crouch: 0.0,
                motion: 0,
            }),
            ..Contact::default()
        },
        Contact {
            level: 3,
            character: true,
            estimate: Some(Vec3::new(150.0, 0.0, 0.0)),
            posture: Some(Posture {
                condition: 0,
                reach_cm: 40.0,
                running: false,
                crouch: 0.95,
                motion: 5,
            }),
            ..Contact::default()
        },
    ];
    let mut field = Beside;
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
    assert_eq!(t.blow.motion, Some(AttackMotion::Overhead));
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
    // A dead foe is left even with a focus: taking the foe cleared it.
    let mut mind = CombatMind::default();
    let mut contacts = [foe_contact(3)];
    let mut field = Flat::new(foe.chest);
    let mut roll = 0;
    let t = tick(
        &mut mind,
        &me,
        MASTER,
        World {
            foe: Some(&dead),
            focus: Some(Vec3::new(0.0, 0.0, 100.0)),
            contacts: &mut contacts,
            around: &[],
            field: &mut field,
            leader_calls: false,
        },
        &mut roll,
    );
    assert_eq!(
        t.exit,
        Some(Exit::Leave {
            look_ms: 8000,
            next_ms: 5500
        })
    );
}

#[test]
fn a_fight_drops_the_slow_turn() {
    let (me, foe) = duel();
    let mut mind = CombatMind::default();
    let mut field = Flat::new(foe.chest);
    let mut roll = 0;
    let mut ran = |me: &Body, foe: Option<&Foe>, leader: bool| {
        let mut contacts = [foe_contact(3)];
        tick(
            &mut mind,
            me,
            NOVICE,
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
        .clear_slow_turn
    };
    assert!(ran(&me, Some(&foe), false));
    let charging = Body {
        running: true,
        forward_speed: 3.5,
        ..me
    };
    assert!(ran(&charging, Some(&foe), false));
    assert!(!ran(&me, Some(&foe), true));
    let down = Body {
        condition: DOWN,
        ..me
    };
    assert!(!ran(&down, Some(&foe), false));
    let dead = Foe { dead: true, ..foe };
    assert!(!ran(&me, Some(&dead), false));
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

#[test]
fn the_can_act_gate_reads_the_tone_the_support_and_the_block() {
    let ok = Body {
        condition: 0,
        tone: 0.7,
        support: 0.71,
        blocking: 0,
        ..Body::default()
    };
    assert!(can_act(&ok));
    // The tone compares in single precision with 0.7: just under fails.
    assert!(!can_act(&Body { tone: 0.69, ..ok }));
    // The support compares in double precision and must pass 0.7.
    assert!(!can_act(&Body { support: 0.7, ..ok }));
    assert!(!can_act(&Body { blocking: 1, ..ok }));
    assert!(!can_act(&Body {
        condition: exa_physics::motile::ACTION_BLOCK,
        ..ok
    }));
    // A value that is not a number fails every compare.
    let nan = f32::NAN;
    assert!(!can_act(&Body { tone: nan, ..ok }));
    assert!(!can_act(&Body { support: nan, ..ok }));
}

#[test]
fn a_run_stop_puts_the_step_goal_eight_speeds_along_the_facing() {
    let me = Body {
        position: Vec3::new(0.0, 90.0, 0.0),
        ground_y: 90.0,
        forward_speed: 1.0,
        facing: Vec3::X,
        ..Body::default()
    };
    // 8·1 + 20 = 28 cm: a goal a metre ahead is put 28 cm along the
    // facing (+x), not toward the goal (+z).
    let stopped = stop_run(
        &me,
        0,
        Vec3::new(0.0, 90.0, 1000.0),
        &mut Flat::new(Vec3::ZERO),
    );
    assert_eq!(stopped.flags, super::LOCO_STOP);
    assert_eq!(stopped.step_goal, Vec3::new(28.0, 90.0, 0.0));
    // Inside the radius it stays, and the stop clears the run bit.
    let near = stop_run(
        &me,
        super::LOCO_RUN,
        Vec3::new(3.0, 90.0, 20.0),
        &mut Flat::new(Vec3::ZERO),
    );
    assert_eq!(near.step_goal, Vec3::new(3.0, 90.0, 20.0));
    assert_eq!(near.flags, super::LOCO_STOP);
}

/// A field with a power system: two usable weight-5 powers.
struct Casting {
    running: bool,
}

impl Field for Casting {
    fn score(&mut self, _contact: usize, _level: i32, point: Vec3) -> PlaceScore {
        PlaceScore {
            bearing: yaw(Vec3::new(0.0, 0.0, 100.0) - point),
            distance_cm: (Vec3::new(0.0, 0.0, 100.0) - point).length(),
            ..PlaceScore::default()
        }
    }
    fn powers(&mut self) -> Option<&mut dyn crate::cast::Powers> {
        Some(self)
    }
    fn cast_running(&mut self) -> bool {
        self.running
    }
}

impl crate::cast::Powers for Casting {
    fn query(&mut self, _power: u32) -> u32 {
        crate::cast::USABLE | 5
    }
    fn cast(&mut self, _power: u32) {}
}

#[test]
fn a_fighter_with_powers_casts_at_the_first_place_and_stops_the_cast() {
    let (mut me, foe) = duel();
    me.power_slots[0] = 1;
    me.power_slots[1] = 2;
    me.power_pool = 1.0;
    let mut mind = CombatMind {
        level: 3,
        ..CombatMind::default()
    };
    let mut contacts = [foe_contact(3)];
    let mut field = Casting { running: false };
    let mut roll = 0;
    let mut cast = None;
    for _ in 0..400 {
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
        if t.cast.is_some() {
            cast = t.cast;
            break;
        }
        mind.casting = false;
    }
    let choice = cast.expect("a cast in 400 ticks");
    // Either power may win (the last of the table is drawn too).
    assert!(choice.slot < 2 && choice.power == choice.slot as u32 + 1);
    assert!(mind.casting);
    // A second tick does not choose again while the cast runs.
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
    assert_eq!(t.cast, None);
    // A second after its start, the power system no longer running it,
    // the cast stops.
    let later = Body {
        now_ms: mind.cast_start_ms.wrapping_add(1001),
        condition: CASTING,
        ..me
    };
    let mut field = Casting { running: false };
    let t = tick(
        &mut mind,
        &later,
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
    assert!(t.cast_stopped);
    assert!(!mind.casting);
    assert_eq!(t.cast, None);
}

/// The cast sits at the first place, after its contact pass: a pass that
/// keeps the foe may cast (the attack drive is cleared for the places), a
/// pass that drops it (nothing pressed) casts nothing, and a search that
/// starts past the first place casts nothing either.
#[test]
fn a_first_place_that_drops_the_foe_casts_nothing() {
    let (mut me, foe) = duel();
    me.power_slots[0] = 1;
    me.power_slots[1] = 2;
    me.power_pool = 1.0;
    let d = Drives {
        attack: 3.0,
        strike: 0.0,
        spacing: 0.0,
        committed: false,
        set_flags: 0,
    };
    let plan = |first| Footwork {
        first,
        last: 1,
        mode: 0,
        change: 5.0,
        queue_taken: false,
    };
    let mut field = Casting { running: false };
    let mut roll = 0;
    let run = |contacts: &mut [Contact], first, roll: &mut u32, field: &mut Casting| {
        let mut mind = CombatMind {
            level: 3,
            ..CombatMind::default()
        };
        search(
            &mut mind,
            &me,
            Some(&foe),
            NOVICE,
            plan(first),
            &d,
            contacts,
            field,
            roll,
        )
    };
    let cast = (0..400).find_map(|_| {
        let found = run(&mut [foe_contact(3)], 0, &mut roll, &mut field);
        found.cast.is_some().then_some(found)
    });
    let found = cast.expect("a cast in 400 searches");
    assert_eq!(found.attack, 0.0);
    for _ in 0..400 {
        let found = run(&mut [], 0, &mut roll, &mut field);
        assert_eq!(found.retarget, Some(None));
        assert!(found.cast.is_none());
        assert_eq!(found.attack, 3.0);
        let found = run(&mut [foe_contact(3)], 1, &mut roll, &mut field);
        assert!(found.cast.is_none());
    }
}

#[test]
fn the_morale_drifts_stress_alarm_courage_panic_and_jitter() {
    let mut state = MoraleDrift {
        stress: 1.0,
        alarm: 1.0,
        courage: 0.5,
        confidence: 0.5,
        ..MoraleDrift::default()
    };
    let body = MoraleBody {
        health: 1.0,
        max_health: 1.0,
        down: false,
    };
    let mut roll = 0;
    drift_morale(&mut state, body, &mut roll);
    assert!((state.stress - 0.99).abs() < 1e-6);
    assert!((state.alarm - 0.9999).abs() < 1e-6);
    // The factor (1 + 1)(0.5 + 1.1)/2 = 1.6; the floor is the doubled
    // stress and alarm (over the confidence plus a half).
    let factor = 1.6f32;
    let floor = (0.99f32 + 0.9999) * 2.0;
    let courage = 0.5 + (floor - 0.5) * factor * 0.0003;
    assert!((state.courage - courage).abs() < 1e-6, "{}", state.courage);
    let over = (0.99 + 0.9999) - courage;
    let expected = over * 0.001 * 0.995;
    assert!((state.panic - expected).abs() < 1e-6);
    assert_eq!(roll, 1);
    // A drop in health lowers the courage by (ratio·0.5 + 0.5)·0.9; at its
    // floor (the confidence plus a half) it then decays by 0.999.
    let mut hurt = MoraleDrift {
        health_ratio: 1.0,
        courage: 1.0,
        ..MoraleDrift::default()
    };
    drift_morale(
        &mut hurt,
        MoraleBody {
            health: 0.5,
            max_health: 1.0,
            down: false,
        },
        &mut 0,
    );
    assert!((hurt.courage - 0.75 * 0.9 * 0.999).abs() < 1e-6);
    // A downed body's ratio is scaled by 0.4 + 0.2·confidence, and its
    // jitter falls 0.003 a tick, floored at −1.
    let mut down = MoraleDrift {
        jitter: -0.999,
        confidence: 1.0,
        ..MoraleDrift::default()
    };
    drift_morale(
        &mut down,
        MoraleBody {
            health: 1.0,
            max_health: 1.0,
            down: true,
        },
        &mut 0,
    );
    assert!((down.health_ratio - 0.6).abs() < 1e-6);
    assert_eq!(down.jitter, -1.0);
}
