//! What a wound sounds and feels like after the damage call: the victim's
//! wound sound at the blow, its pain voice (a groan or a grunt), and the
//! player's feedback (the camera's jolt and the pad's rumble).
//!
//! Evidence: the tail of the original's damage routine (`TMotileDynamics`),
//! read from the assembly. It runs after the damage call on the wound `L`
//! (the part's multiplier applied, before the red bar's 0.75) and the
//! yellow `Y` (before its 0.75). A deep wound (`L` above 0.3) plays the
//! wound sound at 0.8 of the voice's volume and groans; a lighter one (`L`
//! above the epsilon, or `Y` above 0.2) plays the wound sound only past
//! `L` 0.1, and groans past `Y` 0.5, else grunts. A victim whose voice
//! level is under 0.01 makes no voice. A struck player's camera jolts by
//! `7Y + 7L` (a big striker's blow by twice its speed and four times that,
//! an overhead's at 0.4) when that passes 1, and its pad rumbles by the
//! same; a striking player's pad rumbles by half the yellow. A scripted
//! damage hook, when one is set, replaces the damage call and skips all of
//! this. The wound sound is the character's own (`WndFls` for flesh,
//! `SkelHit` for a skeleton, `HitGlm` for a golem), chosen at its init.

use bevy::prelude::*;

use crate::damage::EPSILON;

/// A wound's pain sounds, written by the damage pass after the damage call
/// on flesh (not on a guard, a parry or a turned blow): the wound sound at
/// the blow `point`, the voice at the victim's standing place.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct PainSounded {
    pub fighter: Entity,
    pub point: Vec3,
    pub sounds: PainSounds,
}

/// A blow dealt on flesh, after the damage call and its pain sounds: what
/// the player's feedback reads. The struck player feels
/// [`struck_player_jolt`] (the camera's jolt along `direction`, the pad's
/// rumble); a striking player [`striking_player_rumble`].
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct BlowFelt {
    /// The struck fighter.
    pub target: Entity,
    /// The striking fighter.
    pub attacker: Entity,
    /// The wound `L`, before the red bar's 0.75.
    pub wound: f32,
    /// The yellow `Y`, before its 0.75.
    pub yellow: f32,
    pub striker: Striker,
    /// The contact's queued impact vector, which the camera's jolt is
    /// thrown along: the blade's velocity at the contact (cm a tick).
    /// [inference: the original's impact record holds a vector and the
    /// speed beside it; the vector is read as the blow's direction]
    pub direction: Vec3,
}

/// A swing's first clash or block, which rumbles the player's pad by half
/// the blow's volume when the player struck it or was struck.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ClashFelt {
    pub target: Entity,
    pub attacker: Entity,
    /// The blow's hit-sound volume.
    pub volume: f32,
}

/// A clash's rumble: half the blow's volume.
pub fn clash_rumble(volume: f32) -> f32 {
    volume * 0.5
}

/// The wound sound set a character's init picks from its record's race
/// byte: none for race 0x0E, `SkelHit` for the skeleton (0x01), `WndFls`
/// for a race without the 0x10 bit, `HitGlm` (the golems) otherwise.
pub fn wound_set(race: u8) -> Option<&'static str> {
    match race {
        0x0E => None,
        0x01 => Some("SkelHit"),
        r if r & 0x10 == 0 => Some("WndFls"),
        _ => Some("HitGlm"),
    }
}

/// A sound or voice line's volume and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    pub volume: f32,
    pub pitch: f32,
}

/// What a wound makes the victim sound like.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PainSounds {
    /// The victim's wound sound at the blow.
    pub wound: Option<Loudness>,
    /// The pain voice: its line (1 a grunt, 2 a groan, as
    /// [`crate::voice::Ask::line`]) and its loudness before the character's
    /// gains.
    pub voice: Option<(i32, Loudness)>,
}

/// What the victim brings to its pain sounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sufferer {
    /// It has a wound sound.
    pub wound_sound: bool,
    /// Its voice level: under 0.01 (a double compare) it is mute.
    pub voice_level: f32,
}

/// The deep wound's line: above 0.3 (double) the voice groans whatever
/// the yellow.
const DEEP: f64 = 0.3;
/// The lighter wound's yellow line, and the line over which it groans.
const FELT_YELLOW: f64 = 0.2;
const GROAN_YELLOW: f32 = 0.5;
/// The wound past which a lighter blow still plays the wound sound.
const SOUNDED: f64 = 0.1;
/// The voice's least level (double).
const VOICED: f64 = 0.01;

/// `a > b` with the original's double compare (false on NaN).
fn above(a: f32, b: f64) -> bool {
    f64::from(a) > b
}

/// The voice line's pitch from the routine's pitch: `0.3·p + 0.7` (double).
fn voice_pitch(pitch: f32) -> f32 {
    (f64::from(pitch) * 0.3 + 0.7) as f32
}

/// The wound sound's pitch: `(L·0.125) + 0.9`, the product in single
/// precision, the sum in double.
fn wound_pitch(wound: f32) -> f32 {
    (f64::from(wound * 0.125) + 0.9) as f32
}

/// The pain sounds of a wound `wound` and yellow `yellow` (both before the
/// bars' 0.75) on `victim`.
pub fn pain_sounds(wound: f32, yellow: f32, victim: &Sufferer) -> PainSounds {
    let voiced = f64::from(victim.voice_level) >= VOICED;
    if above(wound, DEEP) {
        let pitch = wound_pitch(wound);
        let volume = (f64::from(wound * 0.5) + 0.2) as f32;
        return PainSounds {
            wound: victim.wound_sound.then(|| Loudness {
                volume: (f64::from(volume) * 0.8) as f32,
                pitch,
            }),
            voice: voiced.then(|| {
                (
                    2,
                    Loudness {
                        volume,
                        pitch: voice_pitch(pitch),
                    },
                )
            }),
        };
    }
    // A NaN wound is not felt by its own test and falls to the yellow's.
    let felt = matches!(
        wound.partial_cmp(&EPSILON),
        Some(std::cmp::Ordering::Greater)
    );
    if !felt && !above(yellow, FELT_YELLOW) {
        return PainSounds::default();
    }
    let wound_sound = (above(wound, SOUNDED) && victim.wound_sound).then(|| Loudness {
        volume: (f64::from(wound) * 0.6) as f32,
        pitch: wound_pitch(wound),
    });
    let pitch = (f64::from(yellow * 0.25) + (f64::from(wound * 0.5) + 0.7)) as f32;
    let volume = (f64::from(yellow) * 0.3 + (f64::from(wound) * 0.4 + 0.1)) as f32;
    // `0.5 >= Y` (or NaN) grunts.
    let line = if yellow > GROAN_YELLOW { 2 } else { 1 };
    PainSounds {
        wound: wound_sound,
        voice: voiced.then(|| {
            (
                line,
                Loudness {
                    volume,
                    pitch: voice_pitch(pitch),
                },
            )
        }),
    }
}

/// The striker a struck player feels the blow of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Striker {
    /// Its body scale: above 1.3 (double) its blow is felt by its speed.
    pub scale: f32,
    /// The blow's speed as the damage routine took it (cm a tick).
    pub speed: f32,
    /// It swings overhead (attack motion 12).
    pub overhead: bool,
}

/// What a struck player feels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jolt {
    /// The camera jolts by the square root of the strength, when the
    /// strength passes 1.
    pub camera: Option<f32>,
    /// The pad's rumble is raised by the strength.
    pub rumble: f32,
}

/// The struck player's jolt: `7Y + 7L` (single precision, the yellow's
/// product first); a big striker's `2v + 4s`, an overhead's then ×0.4
/// (double).
pub fn struck_player_jolt(wound: f32, yellow: f32, striker: &Striker) -> Jolt {
    let mut s = yellow * 7.0 + wound * 7.0;
    if f64::from(striker.scale) > 1.3 {
        s = striker.speed * 2.0 + s * 4.0;
        if striker.overhead {
            s = (f64::from(s) * 0.4) as f32;
        }
    }
    Jolt {
        camera: (s > 1.0).then(|| s.sqrt()),
        rumble: s,
    }
}

/// The striking player's rumble: half the yellow it dealt.
pub fn striking_player_rumble(yellow: f32) -> f32 {
    yellow * 0.5
}

#[cfg(test)]
#[path = "pain_test.rs"]
mod tests;
