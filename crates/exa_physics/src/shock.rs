//! The electric shock a character's body takes (an energy arc, a shock
//! weapon or a tendril), as the character physics reacts to it.
//!
//! - **The window**: a shock does nothing while the body's next allowed
//!   shock is more than 150 ms away. Every shock that acts plays `elechit`
//!   at the chest: two uniform draws, the first giving the pitch
//!   0.4·u + 0.8, the second the gain 0.1·v + 0.3 (worked in double
//!   precision, stored as single).
//! - **Races with bit `0x1000`** of the race word: unless the body is
//!   collapsing (condition bit `0x800`), the shock rings, the body's owner
//!   is put into the shocked state ([`force_release`]) and the next shock
//!   is allowed 500 ms on.
//! - **Race index 1** (race word `& 0xFF00 == 0x100`): it rings, next shock
//!   500 ms on.
//! - **Every other race**: it rings and the next shock is allowed
//!   500 ms plus `(seed × 200) >> 32` ms on, the seed being the game's
//!   32-bit generator advanced once. Unless the body is collapsing it then
//!   burns: yellow health loses the shock's amount (below zero it is set
//!   to zero; a NaN stays), the flag word gets the shock-stagger bit
//!   `0x100`, the condition goes to 0 (normal), motion 3 starts with no
//!   blend, locomotion and its thrust stop, and a stagger step starts:
//!   kind 2, its length 1.5 × the body's step reach, its target the
//!   standing point plus the shock's direction × 200, started now, with no
//!   landings yet and not a balance step. Last, a pain reaction starts on
//!   part 2, or part 1 when the body's grip list holds a second entry,
//!   with two more draws: (0.2·v + 0.4, 0.2·u + 0.8), u the first.
//!
//! Evidence: the character physics' shock reaction, its force-release
//! helper and its sound helper (assembly read for the draw order, the
//! double-precision sums and the unordered compare on the health floor).
//! The motion start, the step, the pain reaction and the sound playback
//! belong to the body, the locomotion and the audio; this module returns
//! what they are asked to do. Not yet called by the running game.

use bevy::math::Vec3;

/// A shock is refused while the next allowed shock is further than this
/// ahead, ms.
pub const SHOCK_WINDOW_MS: u64 = 150;
/// A shock that acts allows the next one this far on, ms.
pub const SHOCK_DELAY_MS: u32 = 500;
/// A burning shock adds `(seed × this) >> 32` ms more.
pub const SHOCK_RANDOM_MS: u64 = 200;
/// The busy time a force release leaves, ms.
pub const RELEASE_BUSY_MS: u32 = 600;
/// The condition bit of a collapsing body: no burn, no release.
pub const COLLAPSING: u32 = 0x800;
/// The race word's bit whose races are released instead of burnt.
pub const RELEASED_RACES: u32 = 0x1000;
/// The race word's race-index byte and the index that only rings.
pub const RACE_INDEX: u32 = 0xFF00;
pub const RINGING_RACE: u32 = 0x100;
/// The flag-word bit a burn sets (the shock stagger).
pub const SHOCK_STAGGER: u32 = 0x100;
/// The condition a burn leaves (normal) and the one a release leaves
/// (shocked).
pub const NORMAL: u32 = 0;
pub const SHOCKED: u32 = 0xC0;
/// The combat condition a release resets.
pub const COMBAT: u32 = 2;
/// The swing flags a release out of combat sets (stalling and staggered).
pub const RELEASE_SWING_FLAGS: u32 = 0x6;
/// The motion a burn starts, with no blend.
pub const BURN_MOTION: u32 = 3;
/// The step kind and length factor of the burn's stagger step, and the
/// shock direction's reach for its target (cm).
pub const STEP_KIND: u32 = 2;
pub const STEP_LENGTH: f32 = 1.5;
pub const STEP_PUSH: f32 = 200.0;
/// The sound's set name.
pub const SHOCK_SOUND: &str = "elechit";

/// The shock sound's gain and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sound {
    pub gain: f32,
    pub pitch: f32,
}

/// What the reaction reads of the shocked body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShockedBody {
    /// The game clock (ms).
    pub now_ms: u32,
    /// The next shock the body allows (game clock, ms).
    pub next_shock_ms: u32,
    /// Its character record's race word.
    pub race_word: u32,
    /// Its condition word.
    pub condition: u32,
    /// Its yellow health.
    pub yellow: f32,
    /// Its step reach (cm).
    pub reach: f32,
    /// Its standing point (cm), the stagger step's origin.
    pub standing: Vec3,
    /// Its grip list holds a second entry: the pain goes to part 1.
    pub second_grip: bool,
}

/// The burn's stagger step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub kind: u32,
    pub length: f32,
    pub target: Vec3,
    pub started_ms: u32,
}

/// The burn's pain reaction: the part and the two values passed to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pain {
    pub part: u32,
    pub first: f32,
    pub second: f32,
}

/// What a shock did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shock {
    /// Refused (the window) or a collapsing body of a released race.
    None,
    /// It rang only (race index 1, or a collapsing body of another race,
    /// whose next shock still takes the random delay).
    Rang { sound: Sound, next_shock_ms: u32 },
    /// It rang and released the body's owner ([`force_release`]).
    Released { sound: Sound, next_shock_ms: u32 },
    /// It rang and burnt the body.
    Burnt {
        sound: Sound,
        next_shock_ms: u32,
        /// The yellow health left.
        yellow: f32,
        step: Step,
        pain: Pain,
    },
}

/// The sound of a shock from its two draws.
pub fn sound(first: f32, second: f32) -> Sound {
    Sound {
        pitch: (f64::from(first) * 0.4 + 0.8) as f32,
        gain: (f64::from(second) * 0.1 + 0.3) as f32,
    }
}

/// One shock of `amount` along `direction` (a unit vector) on `body`.
/// `unit` gives the uniform draws (two for the sound, two more for a
/// burn's pain); `seed` advances the game's 32-bit generator once, only
/// for the races that burn, and returns its new value.
pub fn shock(
    body: &ShockedBody,
    amount: f32,
    direction: Vec3,
    unit: &mut impl FnMut() -> f32,
    seed: &mut impl FnMut() -> u32,
) -> Shock {
    let now = u64::from(body.now_ms);
    if u64::from(body.next_shock_ms) > now + SHOCK_WINDOW_MS {
        return Shock::None;
    }
    let collapsing = body.condition & COLLAPSING != 0;
    let later = body.now_ms.wrapping_add(SHOCK_DELAY_MS);
    if body.race_word & RELEASED_RACES != 0 {
        if collapsing {
            return Shock::None;
        }
        let sound = sound(unit(), unit());
        return Shock::Released {
            sound,
            next_shock_ms: later,
        };
    }
    let sound = sound(unit(), unit());
    if body.race_word & RACE_INDEX == RINGING_RACE {
        return Shock::Rang {
            sound,
            next_shock_ms: later,
        };
    }
    let extra = ((u64::from(seed()) * SHOCK_RANDOM_MS) >> 32) as u32;
    let next_shock_ms = extra.wrapping_add(later);
    if collapsing {
        return Shock::Rang {
            sound,
            next_shock_ms,
        };
    }
    let mut yellow = body.yellow - amount;
    if yellow < 0.0 {
        yellow = 0.0;
    }
    let step = Step {
        kind: STEP_KIND,
        length: body.reach * STEP_LENGTH,
        target: body.standing + direction * STEP_PUSH,
        started_ms: body.now_ms,
    };
    let first = unit();
    let second = unit();
    let pain = Pain {
        part: if body.second_grip { 1 } else { 2 },
        first: (f64::from(second) * 0.2 + 0.4) as f32,
        second: (f64::from(first) * 0.2 + 0.8) as f32,
    };
    Shock::Burnt {
        sound,
        next_shock_ms,
        yellow,
        step,
        pain,
    }
}

/// What the force release does to a body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Release {
    /// Its stagger step was on: the step is ended (feet placed).
    pub end_step: bool,
    /// It was in combat: the swing's readiness values are reset and the
    /// swing flags get [`RELEASE_SWING_FLAGS`].
    pub reset_swing: bool,
    /// Busy until this game clock (ms); locomotion, its thrust and the
    /// clip rate stop.
    pub busy_until_ms: u32,
    /// The clip rate it writes (0): the clock keeps it until the posture
    /// block's stance reset raises it to the race base again.
    pub clip_rate: f32,
    /// The condition it is left in: [`SHOCKED`].
    pub condition: u32,
}

/// What one body tick writes while the shock-stagger flag is set, after
/// this tick's energy recovery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaggerTick {
    /// Energy, the most minus 0.5 in single precision.
    pub energy: f32,
    /// The flag word, the stagger bit cleared once the clock is past the
    /// next shock.
    pub flags: u32,
    /// Rebalance starts with no blend. Already playing it, the clip stays.
    pub play_rebalance: bool,
}

/// The body tick's shock stagger. Nothing when the flag is clear. While it
/// is set, energy becomes `max_energy − 0.5` (single precision; a NaN
/// stays) and Rebalance starts unless `playing_rebalance`. The flag clears
/// once `now_ms` is past `next_shock_ms`; a deadline still ahead, or equal
/// to the clock, keeps it. The child-pose scatter that follows the energy
/// write in the original is not here.
pub fn shock_stagger_tick(
    flags: u32,
    max_energy: f32,
    playing_rebalance: bool,
    now_ms: u64,
    next_shock_ms: u64,
) -> Option<StaggerTick> {
    if flags & SHOCK_STAGGER == 0 {
        return None;
    }
    let mut flags = flags;
    if next_shock_ms < now_ms {
        flags &= !SHOCK_STAGGER;
    }
    Some(StaggerTick {
        energy: max_energy - 0.5,
        flags,
        play_rebalance: !playing_rebalance,
    })
}

/// What the shocked state's own recovery writes on the body: the grab it
/// held released, its foot-contact marks and carry point cleared, and, out
/// of combat (condition 2), its readiness value set to 1, its other combat
/// value to 0 and the swing flags' two low bits set. The slide word goes to
/// 0, the body is busy for 600 ms and left in [`SHOCKED`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recovered {
    /// The body held a grab (`0x12EE` set): it is released.
    pub released: bool,
    /// Its condition was [`COMBAT`]: the readiness and swing values reset.
    pub combat_reset: bool,
    /// The body's slide word, 0.
    pub slide: f32,
    /// Busy until this game clock (ms).
    pub busy_until_ms: u32,
    /// Its condition after the recovery.
    pub condition: u32,
}

/// The shocked state's recovery writes for a body in `condition` that is
/// (or is not) holding a grab.
pub fn recover(now_ms: u32, condition: u32, gripping: bool) -> Recovered {
    Recovered {
        released: gripping,
        combat_reset: condition == COMBAT,
        slide: 0.0,
        busy_until_ms: now_ms.wrapping_add(RELEASE_BUSY_MS),
        condition: SHOCKED,
    }
}

/// The force release on a body that is `stepping` in `condition`.
pub fn force_release(now_ms: u32, stepping: bool, condition: u32) -> Release {
    Release {
        end_step: stepping,
        reset_swing: condition == COMBAT,
        busy_until_ms: now_ms.wrapping_add(RELEASE_BUSY_MS),
        clip_rate: 0.0,
        condition: SHOCKED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> ShockedBody {
        ShockedBody {
            now_ms: 1_000,
            next_shock_ms: 0,
            race_word: 0,
            condition: 1,
            yellow: 1.0,
            reach: 80.0,
            standing: Vec3::new(10.0, 0.0, 20.0),
            second_grip: false,
        }
    }

    fn draws(values: &'static [f32]) -> impl FnMut() -> f32 {
        let mut i = 0;
        move || {
            i += 1;
            values[i - 1]
        }
    }

    /// The shock start writes the clip rate 0, condition 0xC0 and a 600 ms
    /// busy timer.
    #[test]
    fn the_release_writes_a_zero_clip_rate() {
        let release = force_release(1_000, false, 0);
        assert_eq!(release.clip_rate, 0.0);
        assert_eq!(release.busy_until_ms, 1_600);
        assert_eq!(release.condition, SHOCKED);
        assert!(!release.end_step && !release.reset_swing);
    }

    /// The body tick while the stagger flag is set: energy is the most
    /// minus 0.5, Rebalance starts unless it plays, and the flag clears
    /// once the clock is past the next shock.
    #[test]
    fn the_stagger_pins_energy_and_clears_when_the_clock_passes() {
        let held = shock_stagger_tick(SHOCK_STAGGER | 0x10, 1.0, false, 500, 500).unwrap();
        assert_eq!(held.energy.to_bits(), 0x3f00_0000);
        assert_eq!(held.flags, SHOCK_STAGGER | 0x10);
        assert!(held.play_rebalance);
        let playing = shock_stagger_tick(SHOCK_STAGGER, 0.9, true, 0, 0).unwrap();
        assert_eq!(playing.energy.to_bits(), (0.9f32 - 0.5).to_bits());
        assert!(!playing.play_rebalance);
        assert_eq!(playing.flags, SHOCK_STAGGER);
        let done = shock_stagger_tick(SHOCK_STAGGER | 0x10, 1.0, false, 501, 500).unwrap();
        assert_eq!(done.flags, 0x10);
        assert!(shock_stagger_tick(0x10, 1.0, false, 0, 0).is_none());
        assert!(
            shock_stagger_tick(SHOCK_STAGGER, f32::NAN, false, 1, 0)
                .unwrap()
                .energy
                .is_nan()
        );
    }

    /// The window: refused while the next shock is more than 150 ms away.
    #[test]
    fn a_shock_inside_the_window_does_nothing() {
        let mut no_draw = || -> f32 { unreachable!() };
        let mut no_seed = || -> u32 { unreachable!() };
        let b = ShockedBody {
            next_shock_ms: 1_151,
            ..body()
        };
        assert_eq!(
            shock(&b, 0.5, Vec3::X, &mut no_draw, &mut no_seed),
            Shock::None
        );
        let b = ShockedBody {
            next_shock_ms: 1_150,
            race_word: RINGING_RACE,
            ..body()
        };
        assert!(matches!(
            shock(&b, 0.5, Vec3::X, &mut draws(&[0.0, 0.0]), &mut no_seed),
            Shock::Rang {
                next_shock_ms: 1_500,
                ..
            }
        ));
    }

    /// The sound: the first draw is the pitch, the second the gain.
    #[test]
    fn the_sound_takes_pitch_then_gain() {
        let s = sound(0.5, 0.25);
        assert_eq!(s.pitch, (0.5f64 * 0.4 + 0.8) as f32);
        assert_eq!(s.gain, (0.25f64 * 0.1 + 0.3) as f32);
    }

    /// A burn: yellow floored at zero, the random delay from the upper
    /// product, the stagger step and the pain with its draws swapped.
    #[test]
    fn a_burn_steps_and_hurts() {
        let b = ShockedBody {
            yellow: 0.3,
            ..body()
        };
        let mut seed = || u32::MAX;
        let Shock::Burnt {
            sound: s,
            next_shock_ms,
            yellow,
            step,
            pain,
        } = shock(
            &b,
            0.5,
            Vec3::Z,
            &mut draws(&[0.5, 0.0, 1.0, 0.5]),
            &mut seed,
        )
        else {
            panic!("a burn");
        };
        assert_eq!(s, sound(0.5, 0.0));
        // (0xFFFFFFFF × 200) >> 32 = 199.
        assert_eq!(next_shock_ms, 1_000 + 500 + 199);
        assert_eq!(yellow, 0.0);
        assert_eq!(step.kind, 2);
        assert_eq!(step.length, 120.0);
        assert_eq!(step.target, Vec3::new(10.0, 0.0, 220.0));
        assert_eq!(step.started_ms, 1_000);
        assert_eq!(pain.part, 2);
        assert_eq!(pain.first, (0.5f64 * 0.2 + 0.4) as f32);
        assert_eq!(pain.second, (1.0f64 * 0.2 + 0.8) as f32);
        let held = ShockedBody {
            second_grip: true,
            ..b
        };
        let Shock::Burnt { pain, .. } =
            shock(&held, 0.1, Vec3::Z, &mut draws(&[0.0; 4]), &mut || 0)
        else {
            panic!("a burn");
        };
        assert_eq!(pain.part, 1);
    }

    /// A NaN health is not floored (the compare is unordered).
    #[test]
    fn a_nan_health_stays() {
        let b = ShockedBody {
            yellow: f32::NAN,
            ..body()
        };
        let Shock::Burnt { yellow, .. } = shock(&b, 0.1, Vec3::Z, &mut draws(&[0.0; 4]), &mut || 0)
        else {
            panic!("a burn");
        };
        assert!(yellow.is_nan());
    }

    /// A collapsing body: a burning race only rings (with the random
    /// delay); a released race does nothing.
    #[test]
    fn a_collapsing_body_neither_burns_nor_releases() {
        let b = ShockedBody {
            condition: COLLAPSING,
            ..body()
        };
        assert_eq!(
            shock(&b, 0.5, Vec3::Z, &mut draws(&[0.0, 0.0]), &mut || 0),
            Shock::Rang {
                sound: sound(0.0, 0.0),
                next_shock_ms: 1_500
            }
        );
        let released = ShockedBody {
            race_word: RELEASED_RACES,
            ..b
        };
        assert_eq!(
            shock(&released, 0.5, Vec3::Z, &mut || unreachable!(), &mut || 0),
            Shock::None
        );
        let standing = ShockedBody {
            condition: 1,
            ..released
        };
        assert!(matches!(
            shock(&standing, 0.5, Vec3::Z, &mut draws(&[0.0, 0.0]), &mut || 0),
            Shock::Released {
                next_shock_ms: 1_500,
                ..
            }
        ));
    }

    /// The recovery: the grab released, the combat values reset in
    /// condition 2, the slide word 0, busy 600 ms and shocked.
    #[test]
    fn the_shock_recovery_releases_and_clears() {
        let out = recover(2_000, 2, true);
        assert_eq!(
            out,
            Recovered {
                released: true,
                combat_reset: true,
                slide: 0.0,
                busy_until_ms: 2_600,
                condition: SHOCKED
            }
        );
        let calm = recover(0, 1, false);
        assert!(!calm.released && !calm.combat_reset);
        assert_eq!(calm.busy_until_ms, 600);
        // The busy time wraps as the original's 32-bit clock does.
        assert_eq!(recover(u32::MAX - 100, 0, false).busy_until_ms, 499);
    }

    /// The force release: busy 600 ms, shocked, the step ended when on,
    /// the swing reset only out of combat condition 2.
    #[test]
    fn the_force_release_leaves_the_body_shocked() {
        assert_eq!(
            force_release(2_000, true, COMBAT),
            Release {
                end_step: true,
                reset_swing: true,
                busy_until_ms: 2_600,
                clip_rate: 0.0,
                condition: SHOCKED
            }
        );
        let calm = force_release(2_000, false, 1);
        assert!(!calm.end_step && !calm.reset_swing);
    }
}
