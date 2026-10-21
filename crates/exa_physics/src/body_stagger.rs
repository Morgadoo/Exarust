//! A blow's stagger: the part of the hit's speed that carries through its
//! guard or body and rocks it, and the countdown of the stagger's
//! recession. The original's damage routine starts it through a small
//! helper: the strength is the blow's speed × the struck part's factor ×
//! the attack's speed × 0.15, floored at 0; a guarded part scales it by
//! sqrt(share) (or, blocked, blends the share in), clamps it to 1, and the
//! stagger's own speed decays by (1 − it)·0.05 a tick. It sets the
//! stagger's flags (0x2 and 0x4) and its countdown, round(strength·400)
//! ticks from now.
//!
//! The body fields the original writes (the stagger speed, its decay, the
//! countdown and the flags) are [`Stagger`] here. Evidence: the helper's
//! assembly and its call sites in the damage routine. Lengths and speeds
//! are the original's.

use bevy::prelude::Component;

/// The part-speed factor of the strength, and the countdown's ticks per
/// unit of strength.
const PART_SHARE: f64 = 0.15;
const TICKS_PER_STRENGTH: f32 = 400.0;
/// The stagger speed's decay per tick (double precision).
const DECAY: f64 = 0.05;
/// The flags a stagger sets: stalling (0x2) and staggered (0x4).
pub const STAGGER_FLAGS: u32 = 0x6;
/// The single-precision epsilon below which a share is none.
const EPSILON: f32 = 1.192_09e-7;

/// A stagger in progress, as the original keeps it on the body: a
/// component on the body root a blow rocks.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Stagger {
    /// Its speed, 0 to 1.
    pub speed: f32,
    /// The amount its speed falls by a tick.
    pub decay: f32,
    /// The game clock (ms) the stagger ends.
    pub until_ms: u64,
    /// The flags it sets: [`STAGGER_FLAGS`] here.
    pub flags: u32,
}

impl Stagger {
    /// One body tick of the stagger: its speed loses `decay` (never below
    /// 0) while it holds the stalling bit, and it is over once the game
    /// clock is past its countdown. Returns whether it is over.
    pub fn advance(&mut self, now_ms: u64) -> bool {
        if self.flags & 0x2 != 0 {
            self.speed = (self.speed - self.decay).max(0.0);
        }
        now_ms > self.until_ms
    }
}

/// The stagger a blow of `speed` starts (the original's helper): the
/// strength is speed × the struck part's `part_factor` × the attack's
/// `attack_speed` × 0.15, floored at 0; `share` (0..1) is the guard's
/// share of the blow, `blocked` its fully-blocked blend, `now_ms` the game
/// clock. None while the blow is parried away (`parried`, the caller's
/// [`parries_stagger`]).
pub fn stagger_strength(
    speed: f32,
    part_factor: f32,
    attack_speed: f32,
    share: f32,
    blocked: bool,
    parried: bool,
    now_ms: u64,
) -> Option<Stagger> {
    if parried {
        return None;
    }
    // The factors multiply as speed × (part × (attack × 0.15)) in double.
    let strength = (f64::from(speed)
        * (f64::from(part_factor) * (f64::from(attack_speed) * PART_SHARE)))
        as f32;
    let strength = if strength < 0.0 { 0.0 } else { strength };
    let mut value = if blocked {
        (1.0 - share) * strength + share
    } else if share > EPSILON {
        share.sqrt() * strength
    } else {
        0.0
    };
    if value > 1.0 {
        value = 1.0;
    }
    // The difference is taken in single precision, the product in double.
    let decay = (f64::from(1.0 - value) * DECAY) as f32;
    let until = (strength * TICKS_PER_STRENGTH).round_ties_even() as i64;
    Some(Stagger {
        speed: value,
        decay,
        until_ms: now_ms.saturating_add_signed(until),
        flags: STAGGER_FLAGS,
    })
}

/// Whether the original's guard test parries a stagger away: weapon class
/// 0x90, and either an active strike (flags 0x10000) or a clip past 0.4
/// with the part still able to parry.
pub fn parries_stagger(weapon_class: i32, flags: u32, progress: f32, can_parry: bool) -> bool {
    weapon_class == 0x90 && (flags & 0x10000 != 0 || (f64::from(progress) > 0.4 && can_parry))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    /// 1 × 1 × 1 × 0.15: sqrt(1) of it, decay 0.0425, 60 ticks.
    #[test]
    fn a_full_stagger_is_the_originals() {
        let s = stagger_strength(1.0, 1.0, 1.0, 1.0, false, false, 100).unwrap();
        assert!(close(s.speed, 0.15));
        assert!(close(s.decay, 0.0425));
        assert_eq!(s.until_ms, 160);
        assert_eq!(s.flags, STAGGER_FLAGS);
        let s = stagger_strength(1.0, 1.0, 1.0, 0.25, false, false, 0).unwrap();
        assert!(close(s.speed, 0.075));
        assert!(close(s.decay, 0.04625));
        let s = stagger_strength(1.0, 1.0, 1.0, 0.25, true, false, 0).unwrap();
        assert!(close(s.speed, 0.3625));
        assert!(close(s.decay, 0.031875));
    }

    /// A huge blow clamps to 1 (no decay); a negative strength is none.
    #[test]
    fn the_stagger_clamps() {
        let s = stagger_strength(100.0, 1.0, 1.0, 1.0, false, false, 0).unwrap();
        assert_eq!((s.speed, s.decay), (1.0, 0.0));
        let s = stagger_strength(-1.0, 1.0, 1.0, 1.0, false, false, 0).unwrap();
        assert_eq!((s.speed, s.decay), (0.0, 0.05));
        assert!(stagger_strength(1.0, 1.0, 1.0, 1.0, false, true, 0).is_none());
    }

    /// A stagger loses its decay a tick while stalling and is over only
    /// past its countdown; without the stalling bit its speed stays.
    #[test]
    fn the_stagger_decays_and_ends() {
        let s = stagger_strength(1.0, 1.0, 1.0, 1.0, false, false, 100).unwrap();
        let mut s = s;
        assert!(!s.advance(100));
        assert!(close(s.speed, 0.15 - 0.0425));
        assert!(!s.advance(160));
        assert!(s.advance(161));
        let mut quiet = Stagger { flags: 0, ..s };
        quiet.advance(100);
        assert!(close(quiet.speed, s.speed));
    }

    /// The guard test: weapon class 0x90 and an active strike or a parry
    /// window past 0.4.
    #[test]
    fn the_guard_parries_the_stagger() {
        assert!(parries_stagger(0x90, 0x10000, 0.0, false));
        assert!(parries_stagger(0x90, 0, 0.5, true));
        assert!(!parries_stagger(0x90, 0, 0.39, true));
        assert!(parries_stagger(0x90, 0, 0.4, true));
        assert!(!parries_stagger(0x90, 0, 0.5, false));
        assert!(!parries_stagger(0x80, 0x10000, 0.5, true));
    }
}

/// A recoil: the smaller stagger a body takes when its blow is parried or
/// its guard fails. Whatever the guard, its stagger speed and decay are set
/// to 0.5 and its parry bit (0x10) is cleared; unless the guard parries it
/// away, it then sets flags 0x26 and ends after round(speed·0.1·400) + 200
/// ms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recoil {
    /// The stagger speed and decay the recoil sets.
    pub speed: f32,
    pub decay: f32,
    /// The flags it clears.
    pub cleared: u32,
    /// What the recoil goes on to start; none when the guard parries it.
    pub started: Option<RecoilStart>,
}

/// What an unparried recoil starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecoilStart {
    /// The flags it sets and the game clock (ms) it ends.
    pub flags: u32,
    pub until_ms: u64,
    /// The stance start's blend (0x28).
    pub stance_blend: u32,
    /// The two arm stiffness targets, before the arms' floor
    /// ([`attack_arms`]'s), and the strength they are set with.
    pub arms: (f32, f32),
    pub strength: f32,
}

/// The recoil a blow of `speed` starts at `now_ms`; the original's guard
/// (weapon class 0x90 with an active strike or a clip past 0.4 that can
/// parry) leaves it only the speed, decay and cleared bit.
pub fn recoil(
    speed: f32,
    weapon_class: i32,
    flags: u32,
    progress: f32,
    can_parry: bool,
    now_ms: u64,
) -> Recoil {
    let started = (!parries_stagger(weapon_class, flags, progress, can_parry)).then(|| {
        // The speed scales by 0.1 in double, the ticks by 400 in single.
        let ticks =
            (((f64::from(speed) * 0.1) as f32) * TICKS_PER_STRENGTH).round_ties_even() as i64;
        RecoilStart {
            flags: 0x26,
            until_ms: now_ms.saturating_add_signed(ticks + 200),
            stance_blend: 0x28,
            arms: (0.2, 0.2),
            strength: 1.0,
        }
    });
    Recoil {
        speed: 0.5,
        decay: 0.5,
        cleared: 0x10,
        started,
    }
}

#[cfg(test)]
mod recoil_tests {
    use super::*;

    #[test]
    fn the_recoil_is_the_originals() {
        let r = recoil(0.5, 0, 0, 0.0, false, 1000);
        assert_eq!((r.speed, r.decay, r.cleared), (0.5, 0.5, 0x10));
        let start = r.started.unwrap();
        assert_eq!(start.flags, 0x26);
        assert_eq!(start.until_ms, 1000 + 20 + 200);
        assert_eq!(start.stance_blend, 0x28);
        assert_eq!(start.arms, (0.2, 0.2));
        // A parried recoil still sets the speed and decay and clears the
        // bit, but starts nothing.
        let r = recoil(0.5, 0x90, 0x10000, 0.0, false, 0);
        assert_eq!((r.speed, r.decay, r.cleared), (0.5, 0.5, 0x10));
        assert!(r.started.is_none());
    }
}
