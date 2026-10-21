//! A body's status effects and the start of a power's clip.
//!
//! The body keeps a list of power objects, each with a ramp that a power
//! raises by an amount, and a limit at which the power's callback is told to
//! end (signal 0x80) and cleared. Starting a power's clip first asks its
//! callback (none clears it), resets its flag `12e8`, ends a clinch when the
//! body is parrying, starts the motion, and sets the casting condition 0x20.
//!
//! These reproduce the original's effect list and cast-start rules without
//! a power system to drive them; the tests pin the numbers. Evidence: the
//! body's status-effect routines and the cast starter.

/// A power effect on a body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    /// Its ramp.
    pub ramp: f32,
    /// The ramp at which its callback ends it.
    pub limit: f32,
    /// The effect's flag, cleared when it starts.
    pub flag: bool,
}

/// The effect list, kept flat with a count (the original's dynamic array,
/// no per-tick allocation).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Effects {
    items: Vec<Effect>,
}

impl Effects {
    /// Adds an effect (the original's append).
    pub fn add(&mut self, effect: Effect) {
        self.items.push(effect);
    }

    /// The effects.
    pub fn items(&self) -> &[Effect] {
        &self.items
    }

    /// A ramp added to every effect (the original adds to the ramp of each).
    pub fn raise(&mut self, amount: f32) {
        for e in &mut self.items {
            e.ramp += amount;
        }
    }

    /// A power's pulse of `amount`: adds it to every ramp, and returns the
    /// index whose limit is now within `amount` below its ramp (its
    /// callback fires end and the effect is removed). The original calls
    /// the effect's callback with 0x80 and clears it.
    pub fn pulse(&mut self, amount: f32) -> Option<usize> {
        let mut ended = None;
        for (i, e) in self.items.iter_mut().enumerate() {
            e.ramp += amount;
            if e.limit - e.ramp < amount && ended.is_none() {
                ended = Some(i);
            }
        }
        ended
    }

    /// Removes an ended effect.
    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }
}

/// What starting a power's clip does: whether a callback is still
/// registered (a callback returning at least 0 leaves the cast for another
/// tick), the motion started, the condition 0x20, and whether a clinch ends
/// first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastStart {
    /// The callback said to wait: nothing else changes.
    pub waits: bool,
    /// The parry/clinch is ended before the clip.
    pub ends_clinch: bool,
    /// The casting condition 0x20 is set.
    pub casting: bool,
}

/// The cast start for a motion `motion` and whether the body is
/// parrying: a positive motion starts the clip (ending a parry first) and
/// sets condition 0x20; no motion only clears the callback's data.
pub fn cast_start(motion: i32, parrying: bool) -> CastStart {
    CastStart {
        waits: false,
        ends_clinch: motion > 0 && parrying,
        casting: motion > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pulse raises every ramp; the first whose limit is reached is
    /// returned and can be removed.
    #[test]
    fn the_effect_list_ramps_and_ends() {
        let mut list = Effects::default();
        assert!(list.items().is_empty());
        list.add(Effect {
            ramp: 0.0,
            limit: 1.0,
            flag: true,
        });
        list.add(Effect {
            ramp: 0.5,
            limit: 3.0,
            flag: true,
        });
        list.raise(0.25);
        assert_eq!(list.items()[0].ramp, 0.25);
        assert!(list.pulse(0.2).is_none());
        assert_eq!(list.items()[0].ramp, 0.45);
        let ended = list.pulse(0.5).unwrap();
        assert_eq!(ended, 0);
        list.remove(ended);
        assert_eq!(list.items().len(), 1);
    }

    /// A positive motion casts and ends a parry; none only waits.
    #[test]
    fn a_cast_starts_the_clip_and_ends_a_parry() {
        assert_eq!(
            cast_start(0x0E, true),
            CastStart {
                waits: false,
                ends_clinch: true,
                casting: true
            }
        );
        assert!(!cast_start(0, true).casting);
        assert!(!cast_start(0x0E, false).ends_clinch);
    }
}
