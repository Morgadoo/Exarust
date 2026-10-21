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
//! The body fields the original writes (`+0xe9c` the stagger speed,
//! `+0xea0` its decay, `+0xea4` the countdown, `+0x1018` the flags) are
//! [`Stagger`] here. Evidence: the helper's assembly and its call sites in
//! the damage routine. Lengths and speeds are the original's.

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

/// A stagger in progress, as the original keeps it on the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stagger {
    /// Its speed, 0 to 1 (`+0xe9c`).
    pub speed: f32,
    /// The amount its speed falls by a tick (`+0xea0`).
    pub decay: f32,
    /// The game clock (ms) the stagger ends (`+0xea4`).
    pub until_ms: u64,
    /// Its flags (`+0x1018`): [`STAGGER_FLAGS`] here.
    pub flags: u32,
}

/// The stagger a blow of `speed` starts (the original's helper): the
/// strength is speed × the struck part's `part_factor` × the attack's
/// `attack_speed` × 0.15, floored at 0; `share` (0..1) is the guard's
/// share of the blow, `blocked` its fully-blocked blend, `now_ms` the game
/// clock. None while the blow is parried away by the original's test:
/// weapon class 0x90 with an active strike (flag 0x10000) or with a clip
/// past 0.4 that can still parry.
pub fn stagger_strength(
    speed: f32,
    part_factor: f32,
    attack_speed: f32,
    share: f32,
    blocked: bool,
    now_ms: u64,
) -> Option<Stagger> {
    let strength =
        (f64::from(speed) * (f64::from(part_factor) * f64::from(attack_speed) * PART_SHARE)) as f32;
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
    let decay = ((1.0 - f64::from(value)) * DECAY) as f32;
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
        let s = stagger_strength(1.0, 1.0, 1.0, 1.0, false, 100).unwrap();
        assert!(close(s.speed, 0.15));
        assert!(close(s.decay, 0.0425));
        assert_eq!(s.until_ms, 160);
        assert_eq!(s.flags, STAGGER_FLAGS);
        let s = stagger_strength(1.0, 1.0, 1.0, 0.25, false, 0).unwrap();
        assert!(close(s.speed, 0.075));
        assert!(close(s.decay, 0.04625));
        let s = stagger_strength(1.0, 1.0, 1.0, 0.25, true, 0).unwrap();
        assert!(close(s.speed, 0.3625));
        assert!(close(s.decay, 0.031875));
    }

    /// A huge blow clamps to 1 (no decay); a negative strength is none.
    #[test]
    fn the_stagger_clamps() {
        let s = stagger_strength(100.0, 1.0, 1.0, 1.0, false, 0).unwrap();
        assert_eq!((s.speed, s.decay), (1.0, 0.0));
        let s = stagger_strength(-1.0, 1.0, 1.0, 1.0, false, 0).unwrap();
        assert_eq!((s.speed, s.decay), (0.0, 0.05));
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
