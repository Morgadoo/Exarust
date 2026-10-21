//! The electric shock a character's body takes, as the character physics
//! reacts to it: the cooldown that keeps one hit from ringing twice, the
//! race word that only lets it ring, the energy it burns and the stagger
//! and fall state it leaves, and the sound's random volume and pitch.
//!
//! - **The cooldown**: a shock within 150 ms of the body's next allowed time
//!   does nothing. A ringing shock sets the next time 500 ms on; a burning
//!   one adds up to 199 ms more (`(random × 200) >> 32` on the 32-bit
//!   generator).
//! - **The race word**: with `0x1000` set the body does not burn: it only
//!   rings, and a body that is not already down (`0x800` of the second
//!   flags) releases its holder with the force-release reaction.
//! - **Without `0x1000`**: a word whose family (`0xFF00`) is `0x100` only
//!   rings. Otherwise the body burns: its energy loses the shock's amount,
//!   floored at zero; its condition word clears; the `0x100` bit is set;
//!   its grab is dropped; it falls into state 2 at 1.5 times its fall
//!   pace; and it starts a pain reaction on the head (part 2) when it holds
//!   nothing, on the torso (part 1) when it does, with two random draws
//!   (`0.2u + 0.4`, `0.2v + 0.8`).
//! - **The force-release** (`0x17B050`): drops a grab, zeroes the stagger
//!   and shock grab marks, restores readiness when the condition was 2
//!   (`0xEA0 = 1`, `0xE9C = 0`, flags `|= 6`), and sets the stagger
//!   condition `0xC0` with the next shock 600 ms on.
//!
//! Evidence: the character physics' shock reaction, force-release helper
//! and sound helper. Sound playback itself is the audio crate's; this
//! module returns the parameters.

/// A shock this soon before the body's next allowed time does nothing, ms.
pub const SHOCK_WINDOW_MS: u32 = 150;
/// A ringing shock sets the next allowed time this far on, ms.
pub const SHOCK_DELAY_MS: u32 = 500;
/// A burning shock adds up to this many ms more, by a 32-bit draw.
pub const SHOCK_RANDOM_MS: u64 = 200;
/// The force-release's next shock delay, ms.
pub const RELEASE_DELAY_MS: u32 = 600;
/// The condition word's down bit of the second flags: a body already down
/// takes no fall or burn.
pub const DOWN: u32 = 0x800;
/// The race word's "only ring" bit.
pub const ONLY_RING: u32 = 0x1000;
/// The race word's family that only rings (a golem's).
pub const FAMILY_RING: u32 = 0x100;
/// The body flags bit a burn leaves.
pub const SHOCK_FLAG: u32 = 0x100;
/// The condition word a force-release leaves.
pub const STAGGER_CONDITION: u32 = 0xC0;
/// The condition word the burn clears.
pub const CALM_CONDITION: u32 = 0;
/// The fall state a burn enters.
pub const FALL_STATE: i32 = 2;
/// Its fall pace is this times the body's pace.
pub const FALL_PACE: f32 = 1.5;
/// The readiness the force-release restores when it was at condition 2.
pub const READY_AFTER_RELEASE: f32 = 1.0;
/// The flags a force-release adds.
pub const RELEASE_FLAGS: u32 = 6;
/// The parts a pain reaction starts on: the head when the hands are empty,
/// the torso when they hold something.
pub const PAIN_HEAD: usize = 2;
pub const PAIN_TORSO: usize = 1;
/// The sound's cache name.
pub const SHOCK_SOUND: &str = "elechit";

/// The random volume and pitch of a shock's sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sound {
    pub volume: f32,
    pub pitch: f32,
}

/// The body state the reaction reads and writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shock {
    pub now_ms: u32,
    /// The next allowed shock (`0x11B4`).
    pub next_ms: u32,
    /// The race/status word (`0x10A0 + 0x54`).
    pub word: u32,
    /// The condition word (`0xDF4`).
    pub condition: i32,
    /// The energy (`0xD58`).
    pub energy: f32,
    /// The flags (`0xDF8`).
    pub flags: u32,
    /// The second flags (`0xDF4`), whose `0x800` marks a down body.
    pub second_flags: u32,
    /// It holds the shock pain reaction (`0x12EE`).
    pub grabbing: bool,
    /// Its hands hold something (`0xF30 + 8`).
    pub holds: bool,
    /// The pace the fall multiplies (`0xFD0`).
    pub pace: f32,
    /// The fall pace it writes (`0xFD4`).
    pub fall_pace: f32,
    /// The readiness restored by a force-release (`0xEA0`).
    pub ready: f32,
    /// The readiness value it clears (`0xE9C`).
    pub readiness: f32,
}

impl Shock {
    /// A body at rest, never shocked, at full energy.
    pub const fn new(now_ms: u32) -> Self {
        Self {
            now_ms,
            next_ms: 0,
            word: 0,
            condition: CALM_CONDITION as i32,
            energy: 1.0,
            flags: 0,
            second_flags: 0,
            grabbing: false,
            holds: false,
            pace: 1.0,
            fall_pace: 1.0,
            ready: 0.0,
            readiness: 0.0,
        }
    }
}

/// What a shock did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Within the cooldown, or a down body the word spares: nothing.
    None,
    /// It only rang.
    Ring { sound: Sound, next_ms: u32 },
    /// It burned: the body fell and starts its pain reaction.
    Burn {
        sound: Sound,
        next_ms: u32,
        /// The reaction's part and its two random values.
        part: usize,
        amount: f32,
        again: f32,
    },
    /// The word marks a body that does not burn: it releases its holder
    /// with the force-release reaction.
    Release { sound: Sound, next_ms: u32 },
}

/// The force-release's result, written to the shocked body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Released {
    /// The grab it had is dropped.
    pub dropped_grab: bool,
    pub fall_pace: f32,
    pub ready: f32,
    pub readiness: f32,
    pub flags: u32,
    pub next_ms: u32,
    pub condition: i32,
}

/// Takes one shock of `amount` (1.0 is a full blow) at `point`: the sound's
/// two draws come from `sound_random` in [0, 1), and a burning shock takes
/// one 32-bit draw from `roll` for its extra delay. `sound_random` is
/// called only when a sound rings.
pub fn shock(
    body: &mut Shock,
    amount: f32,
    sound_random: &mut impl FnMut() -> f32,
    roll: &mut impl FnMut() -> u32,
) -> Outcome {
    let now = body.now_ms;
    if body.next_ms > now.wrapping_add(SHOCK_WINDOW_MS) {
        return Outcome::None;
    }
    if body.word & ONLY_RING == 0 {
        if body.word & 0xFF00 == FAMILY_RING {
            let sound = ring(sound_random);
            body.next_ms = now.wrapping_add(SHOCK_DELAY_MS);
            return Outcome::Ring {
                sound,
                next_ms: body.next_ms,
            };
        }
        let sound = ring(sound_random);
        body.next_ms = now
            .wrapping_add(SHOCK_DELAY_MS)
            .wrapping_add(((u64::from(roll()) * SHOCK_RANDOM_MS) >> 32) as u32);
        if body.second_flags & DOWN != 0 {
            return Outcome::Ring {
                sound,
                next_ms: body.next_ms,
            };
        }
        body.energy = (body.energy - amount).max(0.0);
        body.flags |= SHOCK_FLAG;
        body.condition = CALM_CONDITION as i32;
        body.grabbing = true;
        body.fall_pace = body.pace * FALL_PACE;
        let part = if body.holds { PAIN_TORSO } else { PAIN_HEAD };
        let amount = sound_random() * 0.2 + 0.4;
        let again = sound_random() * 0.2 + 0.8;
        return Outcome::Burn {
            sound,
            next_ms: body.next_ms,
            part,
            amount,
            again,
        };
    }
    if body.second_flags & DOWN != 0 {
        return Outcome::None;
    }
    let sound = ring(sound_random);
    body.next_ms = now.wrapping_add(SHOCK_DELAY_MS);
    Outcome::Release {
        sound,
        next_ms: body.next_ms,
    }
}

/// A shock's sound: volume `0.4u + 0.8`, pitch `0.1v + 0.3`.
fn ring(random: &mut impl FnMut() -> f32) -> Sound {
    let volume = random() * 0.4 + 0.8;
    let pitch = random() * 0.1 + 0.3;
    Sound { volume, pitch }
}

/// The force-release reaction on the shocked body (or the holder it
/// releases): drops the grab, clears the stagger marks, restores
/// readiness after condition 2 and sets the stagger condition.
pub fn force_release(body: &mut Shock) -> Released {
    let dropped = std::mem::replace(&mut body.grabbing, false);
    if body.condition == FALL_STATE {
        body.ready = READY_AFTER_RELEASE;
        body.readiness = 0.0;
        body.flags |= RELEASE_FLAGS;
    }
    body.next_ms = body.now_ms.wrapping_add(RELEASE_DELAY_MS);
    body.condition = STAGGER_CONDITION as i32;
    Released {
        dropped_grab: dropped,
        fall_pace: body.fall_pace,
        ready: body.ready,
        readiness: body.readiness,
        flags: body.flags,
        next_ms: body.next_ms,
        condition: body.condition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body whose word burns with the race family, at 1 s, with its next
    /// shock passed.
    fn body() -> Shock {
        Shock {
            now_ms: 1_000,
            word: 0x0,
            ..Shock::new(1_000)
        }
    }

    #[test]
    fn a_shock_within_the_window_does_nothing() {
        let mut b = Shock {
            next_ms: 1_151,
            ..body()
        };
        let mut rnd = || 0.0;
        let mut roll = || 0;
        assert_eq!(shock(&mut b, 0.5, &mut rnd, &mut roll), Outcome::None);
        b.next_ms = 1_150;
        assert!(matches!(
            shock(&mut b, 0.5, &mut rnd, &mut roll),
            Outcome::Burn { .. }
        ));
    }

    #[test]
    fn the_ring_family_only_rings() {
        let mut b = Shock {
            word: 0x100,
            ..body()
        };
        let mut rnd = || 0.5;
        let mut roll = || 100;
        let Outcome::Ring { sound, next_ms } = shock(&mut b, 0.5, &mut rnd, &mut roll) else {
            panic!("a ring");
        };
        assert_eq!(next_ms, 1_500);
        assert!((sound.volume - 1.0).abs() < 1e-6);
        assert!((sound.pitch - 0.35).abs() < 1e-6);
        assert_eq!(b.next_ms, 1_500);
        assert_eq!(b.energy, 1.0);
    }

    #[test]
    fn a_burn_spends_energy_and_falls() {
        let mut b = Shock {
            word: 0x1,
            energy: 0.4,
            pace: 2.0,
            holds: false,
            ..body()
        };
        let mut rnd = || 0.5;
        let mut roll = || 0;
        let Outcome::Burn {
            sound,
            next_ms,
            part,
            ..
        } = shock(&mut b, 0.9, &mut rnd, &mut roll)
        else {
            panic!("a burn");
        };
        assert_eq!(next_ms, 1_500);
        assert_eq!(b.next_ms, 1_500);
        assert_eq!(b.energy, 0.0);
        assert_eq!(b.flags, SHOCK_FLAG);
        assert_eq!(b.condition, 0);
        assert!(b.grabbing);
        assert_eq!(b.fall_pace, 3.0);
        assert_eq!(part, PAIN_HEAD);
        assert!((sound.pitch - 0.35).abs() < 1e-6);
        // A torso reaction when the hands hold something.
        let mut holding = Shock {
            holds: true,
            next_ms: 0,
            ..b
        };
        let Outcome::Burn { part, .. } = shock(&mut holding, 0.0, &mut rnd, &mut roll) else {
            panic!("a burn");
        };
        assert_eq!(part, PAIN_TORSO);
    }

    #[test]
    fn the_extra_delay_comes_from_the_upper_product() {
        let mut b = body();
        let mut rnd = || 0.0;
        let mut roll = || u32::MAX;
        let Outcome::Burn { next_ms, .. } = shock(&mut b, 0.1, &mut rnd, &mut roll) else {
            panic!("a burn");
        };
        // (0xFFFFFFFF × 200) >> 32 = 199.
        assert_eq!(next_ms, 1_699);
    }

    #[test]
    fn a_down_body_does_not_burn_or_release() {
        let mut b = Shock {
            second_flags: DOWN,
            word: 0,
            ..body()
        };
        let mut rnd = || 0.0;
        let mut roll = || 0;
        assert!(matches!(
            shock(&mut b, 0.5, &mut rnd, &mut roll),
            Outcome::Ring { .. }
        ));
        b.word = ONLY_RING;
        b.next_ms = 0;
        assert_eq!(shock(&mut b, 0.5, &mut rnd, &mut roll), Outcome::None);
    }

    #[test]
    fn the_force_release_drops_the_grab_and_staggers() {
        let mut b = Shock {
            now_ms: 2_000,
            condition: FALL_STATE,
            grabbing: true,
            fall_pace: 1.5,
            ..Shock::new(2_000)
        };
        let released = force_release(&mut b);
        assert!(released.dropped_grab);
        assert!(!b.grabbing);
        assert_eq!(released.next_ms, 2_600);
        assert_eq!(released.condition, 0xC0);
        assert_eq!(released.flags, RELEASE_FLAGS);
        assert_eq!(released.ready, 1.0);
        assert_eq!(released.readiness, 0.0);
        // A condition other than 2 does not restore readiness.
        let mut calm = Shock { condition: 0, ..b };
        let released = force_release(&mut calm);
        assert_eq!(released.flags, RELEASE_FLAGS);
        assert_eq!(released.ready, 1.0);
        assert_eq!(calm.ready, 1.0);
    }
}
