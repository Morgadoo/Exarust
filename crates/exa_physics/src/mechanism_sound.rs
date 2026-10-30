//! The mechanisms' sounds as their own steps ask for them: a door's hinge
//! creak, stop knock, slam and unlatch, a lever's creak and the tick loop
//! of its timed return, a push surface's grind and click, and the lock
//! sounds of an unlock or a rattled latch.
//!
//! The steps (`crate::door`, `crate::lever`, `crate::plate`) already work
//! out every level; this module turns a tick's report into the requests the
//! mixer plays ([`MechanismSounded`]), with the original's set names and
//! points:
//!
//! - **The restarted voice** ([`MechanismSounded::Voice`]). A door's
//!   `door?_hng`, a lever's `leverb_mov` and a push surface's `Stnb_mov`
//!   each keep one voice on the body. While the step's creak (or grind)
//!   plays, a voice that is not playing is restarted on a fresh variant of
//!   the set at the body's centre with the tick's volume and pitch, and a
//!   playing one only takes the new volume and pitch (its point stays where
//!   it started). The restart is not looping: a variant that ends is
//!   restarted on the next creaking tick. A quiet tick sets the voice's
//!   volume to 0, asks it to stop and frees it.
//! - **One-shots** ([`MechanismSounded::Shot`]) from a numbered set at a
//!   point, their random shares drawn by the mixer: the pitch first, then
//!   the volume, each only when it has a share ([`Sound::levels`]). A
//!   hinged door's knock plays the body's own collision set, or its slam
//!   set when it has none; a sliding door's knock plays the slam set. The
//!   slam (`slm`) and the unlatch (`opn`) play at the handle point, where
//!   the step measured it last; the open a signal's wait ends with plays
//!   there too, before the tick measures it again. A push surface's click
//!   (`Stnb_fit`, volume 0.3, pitch 0.9 plus 0.2 of a draw) plays at the
//!   plate's centre.
//! - **The lever's tick loop** ([`MechanismSounded::TickLoop`]). Arming a
//!   timed return (at its report of 1, or at load with a saved return)
//!   starts the `leverb_tck01` sound on a fresh voice at the lever's centre,
//!   looping at volume 0.1 and pitch 1. While the lever returns its volume
//!   falls by 0.01 a tick; below 0.01 it is set to 0 and freed. The report
//!   of 0 frees it as it is.
//!
//! The original draws the one-shots' shares from the runtime library's own
//! random stream; the mixer draws them from the engine generator, as the
//! door's lock sound already does ([inf] the stream is outside the
//! evidence). The knock and slam cooldowns' draws stay on the door's own
//! roll ([`crate::door::Tick::roll`]).
//!
//! Evidence: the hinged and sliding door steps, the door's open and unlock,
//! the hand's latched-door press and the wrong key, the lever step and its
//! return arming, the push-surface step, and the mixer's voice restart and
//! release, read in the instructions.

use bevy::math::Vec3;
use bevy::prelude::{Entity, Message};

use crate::door::{self, DoorTick, Record, Sound};
use crate::lever::{self, Lever, LeverTick};
use crate::plate::{self, PlateTick};

/// The lever's return tick loop as one sound file, not a set.
pub const TICK_LOOP: &str = lever::TICK_SOUND;

/// The suffixes of a door's sound sets after `door` and its letter.
pub const HINGE: &str = "hng";
pub const SLAM: &str = "slm";
pub const UNLATCH: &str = "opn";
/// The unlock (the right key) and the open order's lock sound.
pub const UNLOCKED: &str = "LkO";
/// The latched door's rattle and the wrong key.
pub const LOCKED: &str = "LkT";

/// What the lever's tick loop does this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickCue {
    /// A fresh voice starts the loop at volume 0.1 and pitch 1.
    Start,
    /// The loop's volume after this tick's fade.
    Gain(f32),
    /// The voice is freed: the fade's end or the report of 0.
    Free,
}

/// A mechanism's sound request.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum MechanismSounded {
    /// A one-shot from the numbered set `set` at `position` (m).
    /// `own_set_of` names a body whose own collision set plays instead,
    /// when it has one (a hinged door's knock).
    Shot {
        set: String,
        position: Vec3,
        sound: Sound,
        own_set_of: Option<Entity>,
    },
    /// The body's restarted voice on the set `set`: `Some((volume,
    /// pitch))` plays, `None` stops and frees it.
    Voice {
        source: Entity,
        set: String,
        position: Vec3,
        levels: Option<(f32, f32)>,
    },
    /// The lever's return tick loop.
    TickLoop {
        source: Entity,
        position: Vec3,
        cue: TickCue,
    },
}

/// Registers the request message once: the mechanism bodies' plugins and
/// the mixer each ask for it.
pub fn register(app: &mut bevy::prelude::App) {
    if !app
        .world()
        .contains_resource::<bevy::prelude::Messages<MechanismSounded>>()
    {
        app.add_message::<MechanismSounded>();
    }
}

/// The name of a door's sound set for its record's letter.
pub fn door_set(letter: u8, suffix: &str) -> String {
    door::sound_name(letter, suffix)
}

/// The sounds of one door tick, in the step's order: the open a finished
/// signal wait made, the creak voice, the knock, the slam and the pull's
/// unlatch. `centre` is the body's centre (where the creak starts),
/// `handle` the handle point the tick measured and `before` the one the
/// last tick measured (where the wait's open plays).
pub fn door_cues(
    source: Entity,
    record: &Record,
    sliding: bool,
    out: &DoorTick,
    centre: Vec3,
    handle: Vec3,
    before: Vec3,
) -> Vec<MechanismSounded> {
    let mut cues = Vec::new();
    let shot =
        |suffix: &str, letter: u8, position: Vec3, sound: Sound, own| MechanismSounded::Shot {
            set: door_set(letter, suffix),
            position,
            sound,
            own_set_of: own,
        };
    if let Some(sound) = out.opened {
        cues.push(shot(UNLATCH, record.unlatch_sound, before, sound, None));
    }
    cues.push(MechanismSounded::Voice {
        source,
        set: door_set(record.hinge_sound, HINGE),
        position: centre,
        levels: out.playing.then_some((out.volume, out.pitch)),
    });
    if let Some(sound) = out.knock_sound {
        let own = (!sliding).then_some(source);
        cues.push(shot(SLAM, record.slam_sound, handle, sound, own));
    }
    if let Some(sound) = out.shut_sound {
        cues.push(shot(SLAM, record.slam_sound, handle, sound, None));
    }
    if let Some(sound) = out.unlatch_sound {
        cues.push(shot(UNLATCH, record.unlatch_sound, handle, sound, None));
    }
    cues
}

/// The lock sound a door plays at its handle: `LkO` for an unlock,
/// `LkT` for a rattled latch or a wrong key.
pub fn lock_cue(record: &Record, suffix: &str, handle: Vec3, sound: Sound) -> MechanismSounded {
    MechanismSounded::Shot {
        set: door_set(record.lock_sound, suffix),
        position: handle,
        sound,
        own_set_of: None,
    }
}

/// The rattle of a latched door the hand presses (and of a wrong key):
/// volume 0.15 plus 0.1 of a draw, pitch 0.8 plus 0.2 of a draw.
pub const RATTLE: Sound = Sound {
    volume: 0.15,
    volume_spread: 0.1,
    pitch: 0.8,
    pitch_spread: 0.2,
};

/// The sounds of one lever tick: the creak voice and the tick loop's cue.
/// `was_playing` is whether the tick loop played before the step.
pub fn lever_cues(
    source: Entity,
    centre: Vec3,
    out: &LeverTick,
    was_playing: bool,
    lever: &Lever,
) -> Vec<MechanismSounded> {
    let mut cues = vec![MechanismSounded::Voice {
        source,
        set: lever::CREAK_SOUND.to_owned(),
        position: centre,
        levels: out.playing.then_some((out.volume, out.pitch)),
    }];
    let cue = if out.arm_return {
        Some(TickCue::Start)
    } else if was_playing && !lever.tick_playing {
        Some(TickCue::Free)
    } else if lever.tick_playing && was_playing {
        Some(TickCue::Gain(lever.tick_volume))
    } else {
        None
    };
    if let Some(cue) = cue {
        cues.push(MechanismSounded::TickLoop {
            source,
            position: centre,
            cue,
        });
    }
    cues
}

/// The sounds of one push-surface tick: the grind voice and the click.
pub fn plate_cues(
    source: Entity,
    centre: Vec3,
    out: &PlateTick,
    plate: &plate::Plate,
) -> Vec<MechanismSounded> {
    let mut cues = vec![MechanismSounded::Voice {
        source,
        set: plate::GRIND_SOUND.to_owned(),
        position: centre,
        levels: out.playing.then_some((out.volume, out.pitch)),
    }];
    if out.click.is_some() {
        cues.push(MechanismSounded::Shot {
            set: plate::CLICK_SOUND.to_owned(),
            position: centre,
            sound: plate.click(),
            own_set_of: None,
        });
    }
    cues
}

#[cfg(test)]
#[path = "mechanism_sound_test.rs"]
mod tests;
