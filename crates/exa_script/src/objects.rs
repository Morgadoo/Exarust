//! Story glue of the game unit: using a scripted world object, its glow
//! pulse, the raise-dead revelation, and the drain of charged power
//! crystals.
//!
//! Using an object goes by its record's type byte: 3 runs its script (with
//! `Actor[0]` the player, at most once per 2.4 s across all such objects),
//! pulses its glow (1.2 s up, 1.2 s down) and plays the sound named in the
//! record at volume 0.3; 4 raises its event; 0x0F and 0x12 are teleporter
//! pedestals and devices; object 588 runs a once-per-session hook. The
//! revelation (event 0xDA0115, story mode only) teaches power 0x10F if the
//! player lacks it and narrates `RVL_RaiseDead_S` or `RVL_RaiseDead_F` after
//! a second. Charged crystals (item 499) lose 2 charge per game second;
//! a source with nothing left to drain leaves the list.
//!
//! Evidence: the object-use, script-run, glow-task, revelation and crystal
//! routines of the game unit and the drain constant (REA on `Exanima.exe`
//! 0.9.5).

/// Minimum time between two scripted-object uses, ms (one timer for all).
pub const USE_INTERVAL: u32 = 2400;
/// Glow pulse ramp of a used object, ms.
pub const PULSE_FADE: u32 = 1200;
/// Volume of a used object's sound.
pub const USE_VOLUME: f32 = 0.3;
/// The revelation's event code.
pub const RAISE_DEAD_EVENT: u32 = 0xDA_0115;
/// The power the revelation teaches.
pub const RAISE_DEAD_POWER: u32 = 0x10F;
/// Delay of the revelation's narration, ms.
pub const REVELATION_DELAY: u32 = 1000;
/// Item id of a power crystal.
pub const CRYSTAL: u32 = 499;
/// Charge drained per game second.
pub const DRAIN: u8 = 2;
/// The object whose use runs a once-per-session hook.
pub const SESSION_HOOK_OBJECT: u32 = 588;

/// What using an object does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Use {
    /// Run the object's script, pulse its glow, play its sound (if named).
    Script {
        sound: Option<String>,
    },
    /// Too soon after the last scripted use: refused.
    TooSoon,
    /// Raise the record's event.
    Event(u32),
    TeleporterPedestal,
    TeleporterDevice,
    /// The once-per-session hook (only the first time).
    SessionHook,
    /// Nothing special: the use goes ahead.
    Plain,
    /// Refused (the hook already ran).
    Refused,
}

/// The scripted-object state of a session.
#[derive(Debug, Clone, Default)]
pub struct Objects {
    /// Clock (ms) of the last scripted use.
    pub last_use: u32,
    pub hook_done: bool,
}

impl Objects {
    /// Use an object of `kind` (its record's type byte), id `object`, event
    /// word `event`, sound name `sound` (empty: none), at clock `now`.
    pub fn use_object(&mut self, kind: u8, object: u32, event: u32, sound: &str, now: u32) -> Use {
        match kind {
            3 => {
                if u64::from(now) < u64::from(self.last_use) + u64::from(USE_INTERVAL) {
                    return Use::TooSoon;
                }
                self.last_use = now;
                Use::Script {
                    sound: (!sound.is_empty()).then(|| sound.chars().take(15).collect()),
                }
            }
            0x0F => Use::TeleporterPedestal,
            0x12 => Use::TeleporterDevice,
            4 => Use::Event(event),
            _ if object == SESSION_HOOK_OBJECT => {
                if self.hook_done {
                    Use::Refused
                } else {
                    self.hook_done = true;
                    Use::SessionHook
                }
            }
            _ => Use::Plain,
        }
    }
}

/// The glow level of a pulse `t` ms after it started: up over `fade`,
/// held for `hold`, down over `fade`; `None` once it is over (the task
/// ends).
pub fn pulse(t: u32, fade: u32, hold: u32) -> Option<f32> {
    if u64::from(fade) * 2 + u64::from(hold) < u64::from(t) {
        return None;
    }
    Some(if t < fade {
        (f64::from(t) / f64::from(fade)) as f32
    } else if u64::from(fade) + u64::from(hold) < u64::from(t) {
        (1.0 - (f64::from(t) - f64::from(hold) - f64::from(fade)) / f64::from(fade)) as f32
    } else {
        1.0
    })
}

/// What the revelation does: `None` outside story mode, for another
/// event, or when the player already has the power; otherwise the node to
/// narrate (after [`REVELATION_DELAY`]) given whether the power could be
/// taught.
pub fn revelation(story: bool, event: u32, has_power: bool, taught: bool) -> Option<&'static str> {
    if !story || event != RAISE_DEAD_EVENT || has_power {
        return None;
    }
    Some(if taught {
        "RVL_RaiseDead_S"
    } else {
        "RVL_RaiseDead_F"
    })
}

/// One drain of a charge: what is left, `None` when it ran out (set to 0).
pub fn drain(charge: u8) -> Option<u8> {
    (charge > DRAIN).then(|| charge - DRAIN)
}

/// Where a charge sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Crystals in the level's containers.
    Containers,
    /// Crystals in one container record.
    Container(u32),
    /// Crystals in the player's inventory.
    Inventory,
    /// A socketed weapon or apparel item.
    Socketed(u32),
}

/// The charge sources being drained.
#[derive(Debug, Clone, Default)]
pub struct Drain {
    pub sources: Vec<Source>,
}

impl Drain {
    /// Add a source once.
    pub fn add(&mut self, s: Source) {
        if !self.sources.contains(&s) {
            self.sources.push(s);
        }
    }

    /// One game second: drain every source with `drain_source` (which
    /// reports whether anything was left to drain); a spent source is
    /// replaced by the last one, as the original swaps it out.
    pub fn tick(&mut self, mut drain_source: impl FnMut(Source) -> bool) {
        let mut i = 0;
        while i < self.sources.len() {
            if drain_source(self.sources[i]) {
                i += 1;
            } else {
                self.sources.swap_remove(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_objects_wait_two_point_four_seconds() {
        let mut o = Objects::default();
        assert_eq!(
            o.use_object(3, 7, 0, "lever01", 5000),
            Use::Script {
                sound: Some("lever01".into())
            }
        );
        assert_eq!(o.use_object(3, 8, 0, "", 7399), Use::TooSoon);
        assert_eq!(o.use_object(3, 8, 0, "", 7400), Use::Script { sound: None });
        assert_eq!(
            o.use_object(4, 9, 0xC1_0EC0, "", 7401),
            Use::Event(0xC1_0EC0)
        );
        assert_eq!(o.use_object(0, 588, 0, "", 0), Use::SessionHook);
        assert_eq!(o.use_object(0, 588, 0, "", 0), Use::Refused);
        assert_eq!(o.use_object(0x12, 1, 0, "", 0), Use::TeleporterDevice);
        assert_eq!(o.use_object(1, 1, 0, "", 0), Use::Plain);
    }

    #[test]
    fn the_pulse_rises_holds_and_falls() {
        assert_eq!(pulse(600, PULSE_FADE, 0), Some(0.5));
        assert_eq!(pulse(1200, PULSE_FADE, 0), Some(1.0));
        assert_eq!(pulse(1800, PULSE_FADE, 0), Some(0.5));
        assert_eq!(pulse(2400, PULSE_FADE, 0), Some(0.0));
        assert_eq!(pulse(2401, PULSE_FADE, 0), None);
        assert_eq!(pulse(1500, 1000, 1000), Some(1.0));
    }

    #[test]
    fn the_revelation_narrates_success_or_failure() {
        assert_eq!(
            revelation(true, RAISE_DEAD_EVENT, false, true),
            Some("RVL_RaiseDead_S")
        );
        assert_eq!(
            revelation(true, RAISE_DEAD_EVENT, false, false),
            Some("RVL_RaiseDead_F")
        );
        assert_eq!(revelation(true, RAISE_DEAD_EVENT, true, true), None);
        assert_eq!(revelation(false, RAISE_DEAD_EVENT, false, true), None);
    }

    #[test]
    fn crystals_drain_two_a_second() {
        assert_eq!(drain(10), Some(8));
        assert_eq!(drain(3), Some(1));
        assert_eq!(drain(2), None);
        let mut d = Drain::default();
        d.add(Source::Inventory);
        d.add(Source::Socketed(5));
        d.add(Source::Inventory);
        d.add(Source::Containers);
        assert_eq!(d.sources.len(), 3);
        // The inventory runs dry: the last source takes its place.
        d.tick(|s| s != Source::Inventory);
        assert_eq!(d.sources, [Source::Containers, Source::Socketed(5)]);
    }
}
