//! The Displacement domain's casts, as the original's Displacement unit
//! routines run them: vanish, the long door, constant displacement and the
//! summoning, their front ends and AI evaluators; the constant
//! displacement's phasing (a blow that passes through), the weapons it
//! lets through for half a second, and where the displacement wisps
//! ([`super::wisps`]) sit on a body.
//!
//! - **Table** ([`DISPLACEMENT_POWERS`]): vanish 0x10601 at 0.10 a second,
//!   the long door 0x602 at 0.20, constant displacement 0x20603 and the
//!   summoning 0x604 at 0.35. The player's front end refuses while status
//!   bit 0x2_0000 is set or a possession runs.
//! - **Vanish**: pressing it while it runs asks the body's hook event 0x10
//!   (its answer ignored), which the vanish's hook takes as its end; it needs 80 times its cost in focus, the instant gate
//!   ([`super::casting::displacement_gate`]) and a cast start without a
//!   motion. **The long door** needs 0.8 times its cost (in double
//!   precision), the instant gate and, for the player, the ground under the
//!   cursor (15 cm back along the hit's normal unless the surface asks
//!   otherwise); it drains the tick in milliseconds times its cost times
//!   0.001 a tick while it charges. **Constant displacement** is a
//!   persistent power under the held gate. **The summoning** needs its cost
//!   and the held gate, plays cast motion 0x28 at the cursor's point (or
//!   the AI's), and spends its cost at once.
//! - **Phasing** ([`PhaseStreak`]): a streak s decides each blow on a
//!   phased body. The blow passes when a percentage roll of the engine's
//!   generator is at most `100 >> (s + 1)` (s ≥ 0) or `100 − (100 >>
//!   (1 − s))` (s < 0): 50 % at first, 25 % after one pass, 75 % after one
//!   miss. A pass raises a non-negative streak (a negative one restarts at
//!   0), plays `thmdsp_phs` at the caster at volume 0.6 and pitch
//!   0.9 + 0.2·r, takes the weapon's collision bits 0xC000, leaves a wisp
//!   at the contact and lists the weapon; a miss lowers a non-positive
//!   streak (a positive one restarts at 0). A listed weapon gets its
//!   collisions back once more than 500 ms have passed, and its wisp
//!   freezes and fades.
//! - **AI**: the long door `round(160·cost) << 8 | 4` with 0.8 times the
//!   cost in focus, facing 1 or more, the target within 3 m, on its feet
//!   and not vanished; constant displacement as the other persistent self
//!   powers; the summoning `round(200·cost) << 8 | 0x800_0000`, with 0x20
//!   when the caster's health is at most three quarters, the cost within
//!   its focus, facing within 0.5 and the enemy 3 m away or more, and
//!   0x400_0000 while a summoned creature lives.
//!
//! Evidence: the Displacement table and AI table, the four casts, the two
//! gates, the front ends, the phasing hook and restore, the wisp table,
//! read from `Exanima.exe` (`context/re/thaum-casts.md`).

use bevy::math::Vec3;

use super::casting::{
    AiView, CastBody, ID_MASK, OFF_FEET, TickScale, VANISHED, advice, ask_hook, code,
    displacement_gate, displacement_hold_gate, free_slots, front_end_blocked, hook_event,
    not_above, not_below, persistent_advice, price, spend, start_cast, within,
};
use super::engine::Roll;

/// One row of the Displacement table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplacementRow {
    pub id: u32,
    pub cost: f32,
}

pub const VANISH: u32 = 0x10601;
pub const LONG_DOOR: u32 = 0x602;
pub const CONSTANT: u32 = 0x20603;
pub const SUMMON: u32 = 0x604;

/// The Displacement table.
pub const DISPLACEMENT_POWERS: [DisplacementRow; 4] = [
    DisplacementRow {
        id: VANISH,
        cost: 0.1,
    },
    DisplacementRow {
        id: LONG_DOOR,
        cost: 0.2,
    },
    DisplacementRow {
        id: CONSTANT,
        cost: 0.35,
    },
    DisplacementRow {
        id: SUMMON,
        cost: 0.35,
    },
];

/// The cast motion the summoning plays.
pub const SUMMON_MOTION: u32 = 0x28;
pub const APPEAR_SOUND: &str = "thmdsp_apr";
pub const VANISH_SOUND: &str = "thmdsp_vns";
pub const CONSTANT_HUM: &str = "thmdsp_cns01";
pub const PHASE_SOUND: &str = "thmdsp_phs";
pub const SUMMON_SOUND: &str = "thmdsp_Smn";

/// What a Displacement cast makes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplacementStart {
    Vanish {
        cost: f32,
    },
    /// The long door toward `to` (none: the AI's door picks its own).
    LongDoor {
        cost: f32,
        to: Option<Vec3>,
        /// What it drains a tick while it charges.
        tick_cost: f32,
    },
    Constant {
        cost: f32,
    },
    /// The summoning at `at` (none: at the AI's point).
    Summon {
        cost: f32,
        at: Option<Vec3>,
    },
}

/// The cursor's ground hit, for the player's long door and summoning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub exact: bool,
}

impl CursorHit {
    /// The hit, 15 cm back along its normal unless the surface asks to keep
    /// it.
    pub fn point(&self) -> Vec3 {
        if self.exact {
            self.point
        } else {
            self.point + self.normal * -15.0
        }
    }
}

/// Who casts, and what it aims at.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DisplacementAim {
    /// The player casts (else an AI).
    pub player: bool,
    /// The cursor's ground hit (the player's).
    pub cursor: Option<CursorHit>,
    /// A possession runs.
    pub possessing: bool,
}

/// One Displacement cast of a priced row.
pub fn cast_displacement(
    row: &DisplacementRow,
    cost: f32,
    body: &mut CastBody,
    aim: &DisplacementAim,
) -> (i32, Option<DisplacementStart>) {
    let held = body.casting as i32;
    match row.id {
        VANISH => {
            if body.casting == row.id {
                let _ = ask_hook(body, hook_event::RELEASE);
                return (code::RELEASED, None);
            }
            if !within(cost * 80.0, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = displacement_gate(body, held);
            if gate < 0 || !start_cast(body, 0, row.id) {
                return (gate, None);
            }
            (code::STARTED, Some(DisplacementStart::Vanish { cost }))
        }
        LONG_DOOR => {
            if body.casting == row.id {
                body.release = true;
                return (code::RELEASED, None);
            }
            if f64::from(cost) * 0.8 > f64::from(body.focus) {
                return (code::REFUSED, None);
            }
            let gate = displacement_gate(body, held);
            if gate < 0 {
                return (gate, None);
            }
            let tick_cost = body.tick_ms as f32 * cost * 0.001;
            let to = if aim.player {
                match aim.cursor {
                    Some(hit) => Some(hit.point()),
                    None => return (gate, None),
                }
            } else {
                None
            };
            if !start_cast(body, 0, row.id) {
                return (gate, None);
            }
            (
                code::STARTED,
                Some(DisplacementStart::LongDoor {
                    cost,
                    to,
                    tick_cost,
                }),
            )
        }
        CONSTANT => {
            let free = match free_slots(body, row.id) {
                Ok(free) => free,
                Err(ended) => return (ended, None),
            };
            if free < 1 {
                return (code::REFUSED, None);
            }
            if body.casting == row.id {
                body.release = true;
                return (code::RELEASED, None);
            }
            if !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = displacement_hold_gate(body, held);
            if gate < 0 {
                return (gate, None);
            }
            (code::STARTED, Some(DisplacementStart::Constant { cost }))
        }
        _ => {
            if body.casting == row.id {
                body.release = true;
                return (code::RELEASED, None);
            }
            if !within(cost, body.focus) {
                return (code::REFUSED, None);
            }
            let gate = displacement_hold_gate(body, held);
            if gate < 0 {
                return (gate, None);
            }
            let at = if aim.player {
                match aim.cursor {
                    Some(hit) => Some(hit.point()),
                    None => return (gate, None),
                }
            } else {
                None
            };
            if !start_cast(body, SUMMON_MOTION, row.id) {
                return (gate, None);
            }
            spend(body, cost);
            (code::STARTED, Some(DisplacementStart::Summon { cost, at }))
        }
    }
}

/// The row for `asked` under the front ends' mask.
pub fn displacement_row(asked: u32) -> Option<&'static DisplacementRow> {
    DISPLACEMENT_POWERS
        .iter()
        .find(|row| asked & ID_MASK == row.id)
}

/// The player's Displacement front end.
pub fn displacement_front_end(
    asked: u32,
    body: &mut CastBody,
    aim: &DisplacementAim,
) -> (i32, Option<DisplacementStart>) {
    if front_end_blocked(body, aim.possessing) {
        return (code::REFUSED, None);
    }
    let Some(row) = displacement_row(asked) else {
        return (code::REFUSED, None);
    };
    let cost = price(row.id, row.cost, asked, body, TickScale::Thousandth);
    cast_displacement(row, cost, body, aim)
}

/// The AI's Displacement front end.
pub fn displacement_ai_front_end(asked: u32, view: &AiView) -> u32 {
    let Some(row) = displacement_row(asked) else {
        return advice::UNAVAILABLE;
    };
    let cost = price(row.id, row.cost, asked, &view.caster, TickScale::Thousandth);
    match row.id {
        LONG_DOOR => long_door_advice(cost, view),
        CONSTANT => persistent_advice(row.id, cost, view),
        SUMMON => summon_advice(cost, view),
        _ => advice::UNAVAILABLE,
    }
}

/// The long door's advice word. Every comparison refuses only on its ordered
/// branch (a not-a-number passes).
pub fn long_door_advice(cost: f32, view: &AiView) -> u32 {
    let use_it = not_below(f64::from(view.caster.focus) * 0.8, f64::from(cost))
        && not_below(view.facing, 1.0)
        && not_above(view.distance_sq, 90_000.0)
        && view.target_condition & OFF_FEET == 0
        && view.target_status & VANISHED == 0;
    if !use_it {
        return 0;
    }
    ((((cost * 160.0).round_ties_even() as i64) << 8) | 4) as u32
}

/// The summoning's advice word.
pub fn summon_advice(cost: f32, view: &AiView) -> u32 {
    if view.summoned {
        return advice::UNAVAILABLE;
    }
    let word = (((cost * 200.0).round_ties_even() as i64) << 8) as u32;
    let ready = not_above(view.health_ratio, 0.75)
        && not_below(view.caster.focus, cost)
        && not_above(view.facing, 0.5)
        && not_below(view.distance_sq, 90_000.0);
    word | 0x800_0000 | if ready { 0x20 } else { 0 }
}

/// The phasing streak of a constant displacement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhaseStreak(pub i32);

/// The collision bits a phased weapon loses.
pub const PHASED_BITS: u32 = 0xC000;
/// The collision bit a constantly displaced body's parts gain.
pub const DISPLACED_BIT: u32 = 0x4000;
/// A phased weapon's collisions come back after this many milliseconds.
pub const PHASE_MS: u64 = 500;
/// The phasing sound's volume.
pub const PHASE_VOLUME: f32 = 0.6;

impl PhaseStreak {
    /// The highest roll that lets a blow through.
    pub fn threshold(self) -> i32 {
        let s = self.0;
        if s < 0 {
            100 - (100u32 >> ((1 - s) as u32 & 0x1f)) as i32
        } else {
            (100u32 >> ((s + 1) as u32 & 0x1f)) as i32
        }
    }

    /// Decides one blow: true when it passes through. On a pass `pitch`
    /// draws the sound's pitch.
    pub fn pass(&mut self, roll: &mut Roll) -> Option<f32> {
        let limit = self.threshold();
        if limit < roll.percent() {
            self.0 = if self.0 < 1 { self.0 - 1 } else { 0 };
            None
        } else {
            self.0 = if self.0 < 0 { 0 } else { self.0 + 1 };
            Some((f64::from(roll.unit()) * 0.2 + 0.9) as f32)
        }
    }
}

/// The weapons a constant displacement let through, with when.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhasedWeapons {
    pub list: Vec<(u64, u64)>,
}

impl PhasedWeapons {
    /// Lists a weapon that passed at `now` (ms).
    pub fn add(&mut self, weapon: u64, now: u64) {
        self.list.push((weapon, now));
    }

    /// The weapons whose 500 ms are over at `now`, in the original's order:
    /// each leaves the list and the rest close up behind it.
    pub fn restore(&mut self, now: u64) -> Vec<u64> {
        let mut back = Vec::new();
        let mut i = 0;
        while i < self.list.len() {
            if self.list[i].1 + PHASE_MS < now {
                back.push(self.list.remove(i).0);
            } else {
                i += 1;
            }
        }
        back
    }
}

/// The wisps' anchors on each body part, by the part's bone (1–18), in its
/// own frame (cm).
pub fn wisp_anchors(bone: usize) -> &'static [[f32; 3]] {
    const CORNERS_PELVIS: [[f32; 3]; 4] = [
        [-10.0, -10.0, 18.0],
        [10.0, -10.0, 18.0],
        [-10.0, -10.0, -13.0],
        [10.0, -10.0, -13.0],
    ];
    const CORNERS_CHEST: [[f32; 3]; 4] = [
        [-10.0, -10.0, 20.0],
        [10.0, -10.0, 20.0],
        [-10.0, -10.0, -10.0],
        [10.0, -10.0, -10.0],
    ];
    match bone {
        1 => &CORNERS_PELVIS,
        2 | 5 => &[[0.0, -25.0, 0.0]],
        3 | 6 => &[[0.0, 0.0, 0.0], [0.0, -20.0, 0.0]],
        4 | 7 => &[[0.0, 0.0, 4.0]],
        9 => &CORNERS_CHEST,
        10 | 17 | 18 => &[[0.0, 0.0, 0.0]],
        12 => &[[-20.0, 0.0, 0.0]],
        13 => &[[-10.0, 0.0, 0.0]],
        15 => &[[20.0, 0.0, 0.0]],
        16 => &[[10.0, 0.0, 0.0]],
        _ => &[],
    }
}

/// Where a phased body's wisps hang: every part of the body, its anchors
/// in its frame. The set keeps room for eight more strands.
pub fn body_wisps(bones: &[usize]) -> Vec<(usize, Vec3)> {
    bones
        .iter()
        .flat_map(|&bone| {
            wisp_anchors(bone)
                .iter()
                .map(move |a| (bone, Vec3::from_array(*a)))
        })
        .collect()
}

/// Room the body set keeps for strands beyond its anchors.
pub const SPARE_STRANDS: usize = 8;

/// An AI's escape point for the long door: 300 cm behind the body along
/// its facing, along the path the finder gives. `path` returns the path's
/// points (none while the finder serves another body); the escape counts
/// when its last point is more than √200 cm away on the level.
pub fn escape_point(
    at: Vec3,
    facing: Vec3,
    path: impl FnOnce(Vec3, Vec3) -> Option<Vec<Vec3>>,
) -> (i32, Option<Vec3>) {
    let goal = at + facing * -300.0;
    let Some(points) = path(at, goal) else {
        return (0, None);
    };
    if 1 < points.len() {
        let last = points[points.len() - 1];
        let dx = last.x - at.x;
        let dz = last.z - at.z;
        let result = if 200.0 < dz * dz + dx * dx { 1 } else { -1 };
        return (result, Some(last));
    }
    (-1, None)
}

#[cfg(test)]
mod tests {
    use super::super::casting::{ActivePower, CASTING, COLLAPSING, POWERS_BLOCKED};
    use super::*;

    fn caster(focus: f32) -> CastBody {
        CastBody {
            focus,
            ..CastBody::default()
        }
    }

    #[test]
    fn vanish_needs_eighty_times_its_cost() {
        let aim = DisplacementAim::default();
        let cost = 12.0f32 * 0.1 * 0.001;
        let mut short = caster(cost * 80.0 - 1e-6);
        assert_eq!(
            displacement_front_end(VANISH, &mut short, &aim).0,
            code::REFUSED
        );
        let mut body = caster(1.0);
        let (result, start) = displacement_front_end(VANISH, &mut body, &aim);
        assert_eq!(result, code::STARTED);
        assert_eq!(start, Some(DisplacementStart::Vanish { cost }));
        assert_eq!(body.condition, 0, "no motion");
        body.cast_hook = Some(super::super::power::displacement::VANISH_HOOK);
        assert_eq!(
            displacement_front_end(VANISH, &mut body, &aim),
            (code::RELEASED, None)
        );
        assert_eq!(body.hook_asked, super::super::casting::hook_event::RELEASE);
    }

    #[test]
    fn the_long_door_needs_the_cursor() {
        // 0.16 in single precision is short of 0.2 × 0.8 in double.
        let mut short = caster(0.16);
        let blind = DisplacementAim {
            player: true,
            ..DisplacementAim::default()
        };
        assert_eq!(
            displacement_front_end(LONG_DOOR, &mut short, &blind).0,
            code::REFUSED
        );
        let mut body = caster(0.1601);
        assert_eq!(
            displacement_front_end(LONG_DOOR, &mut body, &blind),
            (0, None)
        );
        let aim = DisplacementAim {
            player: true,
            cursor: Some(CursorHit {
                point: Vec3::new(100.0, 0.0, 0.0),
                normal: Vec3::Y,
                exact: false,
            }),
            possessing: false,
        };
        let (result, start) = displacement_front_end(LONG_DOOR, &mut body, &aim);
        assert_eq!(result, code::STARTED);
        assert_eq!(
            start,
            Some(DisplacementStart::LongDoor {
                cost: 0.2,
                to: Some(Vec3::new(100.0, -15.0, 0.0)),
                tick_cost: 12.0 * 0.2 * 0.001,
            })
        );
        let mut poor = caster(0.15);
        assert_eq!(
            displacement_front_end(LONG_DOOR, &mut poor, &aim).0,
            code::REFUSED
        );
    }

    #[test]
    fn a_held_cast_hands_back_the_casting_word() {
        let aim = DisplacementAim {
            player: true,
            ..DisplacementAim::default()
        };
        // The cursor finds nothing: the gate's word (the power being cast,
        // here none) comes back.
        let mut body = caster(1.0);
        assert_eq!(displacement_front_end(SUMMON, &mut body, &aim), (0, None));
        let mut dying = caster(1.0);
        dying.condition = COLLAPSING;
        assert_eq!(
            displacement_front_end(SUMMON, &mut dying, &aim).0,
            code::FAILED
        );
        let mut busy = caster(1.0);
        busy.condition = CASTING;
        assert_eq!(
            displacement_front_end(CONSTANT, &mut busy, &aim).0,
            code::NOT_NOW
        );
        assert!(busy.release);
    }

    #[test]
    fn the_ai_summons_at_once_and_spends() {
        let aim = DisplacementAim::default();
        let mut body = caster(1.0);
        let (result, start) = displacement_front_end(SUMMON, &mut body, &aim);
        assert_eq!(result, code::STARTED);
        assert_eq!(
            start,
            Some(DisplacementStart::Summon {
                cost: 0.35,
                at: None
            })
        );
        assert_eq!(body.cast_motion, Some(SUMMON_MOTION));
        assert_eq!(body.spent, 0.35);
    }

    #[test]
    fn constant_displacement_takes_a_slot() {
        let aim = DisplacementAim::default();
        let mut body = caster(1.0);
        assert_eq!(
            displacement_front_end(CONSTANT, &mut body, &aim),
            (
                code::STARTED,
                Some(DisplacementStart::Constant { cost: 0.35 })
            )
        );
        body.powers.push(ActivePower {
            id: CONSTANT,
            state: 1,
        });
        assert_eq!(
            displacement_front_end(CONSTANT, &mut body, &aim).0,
            code::ENDED
        );
        let mut blocked = caster(1.0);
        blocked.status_bits = POWERS_BLOCKED;
        assert_eq!(
            displacement_front_end(CONSTANT, &mut blocked, &aim).0,
            code::REFUSED
        );
    }

    #[test]
    fn the_ai_words_carry_the_cost() {
        // 0.25 × 0.8 in double precision is short of 0.2 in single.
        let mut view = AiView {
            caster: caster(0.25),
            facing: 1.0,
            distance_sq: 90_000.0,
            ..AiView::default()
        };
        assert_eq!(displacement_ai_front_end(LONG_DOOR, &view), 0);
        view.caster.focus = 0.26;
        let mut view = AiView {
            caster: view.caster.clone(),
            facing: 1.0,
            distance_sq: 90_000.0,
            ..AiView::default()
        };
        assert_eq!(displacement_ai_front_end(LONG_DOOR, &view), (32 << 8) | 4);
        view.target_status = VANISHED;
        assert_eq!(displacement_ai_front_end(LONG_DOOR, &view), 0);
        view.health_ratio = 0.75;
        view.facing = 0.5;
        view.caster.focus = 0.35;
        assert_eq!(
            displacement_ai_front_end(SUMMON, &view),
            (70 << 8) | 0x800_0000 | 0x20
        );
        view.health_ratio = 0.8;
        assert_eq!(
            displacement_ai_front_end(SUMMON, &view),
            (70 << 8) | 0x800_0000
        );
        view.summoned = true;
        assert_eq!(
            displacement_ai_front_end(SUMMON, &view),
            advice::UNAVAILABLE
        );
        assert_eq!(
            displacement_ai_front_end(VANISH, &view),
            advice::UNAVAILABLE
        );
    }

    #[test]
    fn a_not_a_number_focus_passes_the_displacement_checks() {
        let view = AiView {
            caster: caster(f32::NAN),
            facing: f32::NAN,
            distance_sq: f32::NAN,
            health_ratio: f32::NAN,
            ..AiView::default()
        };
        // The ordered shortfall branches are skipped when the compare is
        // unordered, so each evaluator answers as it would at a good range.
        assert_eq!(displacement_ai_front_end(LONG_DOOR, &view), (32 << 8) | 4);
        assert_eq!(
            displacement_ai_front_end(SUMMON, &view),
            (70 << 8) | 0x800_0000 | 0x20
        );
        assert_eq!(displacement_ai_front_end(CONSTANT, &view), advice::USE);
        let mut body = caster(f32::NAN);
        assert_eq!(
            displacement_front_end(VANISH, &mut body, &DisplacementAim::default()).0,
            code::STARTED
        );
    }

    #[test]
    fn the_streak_sets_the_odds() {
        assert_eq!(PhaseStreak(0).threshold(), 50);
        assert_eq!(PhaseStreak(1).threshold(), 25);
        assert_eq!(PhaseStreak(2).threshold(), 12);
        assert_eq!(PhaseStreak(-1).threshold(), 75);
        assert_eq!(PhaseStreak(-2).threshold(), 88);
        // A state that steps to 0 rolls 0: the blow passes, the streak
        // rises and the next draw (1/2³²) sets the pitch.
        let mut streak = PhaseStreak(0);
        let mut zero = Roll(0x26b0_5733);
        let pitch = streak.pass(&mut zero).expect("a pass");
        assert!((pitch - 0.9).abs() < 1e-6);
        assert_eq!(streak.0, 1);
        // A state that steps to 0xFFFE_0000 rolls 99: a miss restarts a
        // positive streak, then lowers it.
        let mut high = Roll(0xd516_5733);
        assert_eq!(streak.pass(&mut high), None);
        assert_eq!(streak.0, 0);
        let mut high = Roll(0xd516_5733);
        assert_eq!(streak.pass(&mut high), None);
        assert_eq!(streak.0, -1);
    }

    #[test]
    fn a_weapon_comes_back_after_half_a_second() {
        let mut phased = PhasedWeapons::default();
        phased.add(1, 1000);
        phased.add(2, 1200);
        assert!(phased.restore(1500).is_empty());
        assert_eq!(phased.restore(1501), vec![1]);
        assert_eq!(phased.restore(1701), vec![2]);
        assert!(phased.list.is_empty());
    }

    #[test]
    fn wisps_hang_on_each_part() {
        assert_eq!(wisp_anchors(1).len(), 4);
        assert_eq!(wisp_anchors(8).len(), 0);
        assert_eq!(wisp_anchors(12), &[[-20.0, 0.0, 0.0]]);
        assert_eq!(wisp_anchors(15), &[[20.0, 0.0, 0.0]]);
        let all: Vec<usize> = (1..=18).collect();
        assert_eq!(body_wisps(&all).len(), 23);
    }

    #[test]
    fn the_escape_runs_three_metres_back() {
        let (result, at) = escape_point(Vec3::ZERO, Vec3::Z, |from, to| {
            assert_eq!(to, Vec3::new(0.0, 0.0, -300.0));
            Some(vec![from, Vec3::new(0.0, 0.0, -20.0)])
        });
        assert_eq!(result, 1);
        assert_eq!(at, Some(Vec3::new(0.0, 0.0, -20.0)));
        let (near, _) = escape_point(Vec3::ZERO, Vec3::Z, |from, _| {
            Some(vec![from, Vec3::new(0.0, 0.0, -14.0)])
        });
        assert_eq!(near, -1);
        assert_eq!(escape_point(Vec3::ZERO, Vec3::Z, |_, _| None), (0, None));
    }
}
