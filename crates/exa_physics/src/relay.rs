//! Relays that combine inputs, forward a signal, or count a sequence.
//!
//! A combiner sets the source's bit from the value's low bit, then compares
//! the state with its code. A match signals 1, or 0 when the invert flag is
//! set. A mismatch signals the opposite, unless the silence flag is set.
//! The second combiner signals 1 on a match. On a mismatch it forwards the
//! original value to the source's second target when the state's low bit is
//! clear, and does nothing when that bit is set.
//!
//! A pair sends the same value and the same source to both targets.
//!
//! A sequence ignores a value of 0. A marked input counts one step and
//! stores that mark in the high byte. Overlap with the previous mark flags
//! a wrong order. Eight steps without that flag signal the first target
//! with 1. Fewer steps, or a wrong order, signal the second target with 1.
//! Either finish resets the count and the flag.
//!
//! Evidence: the relay steps. The broadcast across every sector is not in
//! this slice.

/// Match sends 0 instead of 1.
pub const INVERT: u16 = 1;
/// A mismatch sends nothing.
pub const SILENCE: u16 = 8;
/// A finished sequence does not clear a listener with this flag.
pub const SKIP_CLEAR: u16 = 2;
/// The sequence saw an input out of order.
pub const WRONG: u16 = 0x100;
/// A listener is cleared only when these class bits equal [`LISTENER_CLASS`].
pub const LISTENER_MASK: u32 = 0x7000;
/// The class-bit value a cleared listener has.
pub const LISTENER_CLASS: u32 = 0x4000;

/// One signal a relay sends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Forward {
    pub target: u32,
    pub source: u32,
    pub value: i32,
}

/// A combiner's state, flags, first target and code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combiner {
    pub state: u16,
    pub flags: u16,
    pub target: u32,
    pub code: u32,
}

/// Set the source's bit and signal whether the state matches the code.
/// A missing source leaves the combiner unchanged.
pub fn combine(
    relay: &mut Combiner,
    relay_id: u32,
    input_bit: Option<u16>,
    value: i32,
) -> Option<Forward> {
    let bit = input_bit?;
    write_bit(&mut relay.state, bit, value);
    let matched = u32::from(relay.state) == relay.code;
    let on_match = if relay.flags & INVERT == 0 { 1 } else { 0 };
    let sent = if matched {
        on_match
    } else if relay.flags & SILENCE != 0 {
        return None;
    } else {
        on_match ^ 1
    };
    Some(Forward {
        target: relay.target,
        source: relay_id,
        value: sent,
    })
}

/// The second combiner. A match signals 1. A mismatch forwards the original
/// signal to `second` unless the state's low bit is set.
pub fn combine_else(
    relay: &mut Combiner,
    relay_id: u32,
    input_bit: Option<u16>,
    second: u32,
    source_id: u32,
    value: i32,
) -> Option<Forward> {
    let bit = input_bit?;
    write_bit(&mut relay.state, bit, value);
    if u32::from(relay.state) == relay.code {
        Some(Forward {
            target: relay.target,
            source: relay_id,
            value: 1,
        })
    } else if relay.state & 1 == 0 {
        Some(Forward {
            target: second,
            source: source_id,
            value,
        })
    } else {
        None
    }
}

/// Send one value to both targets. A missing relay sends nothing.
pub fn forward_both(
    found: bool,
    first: u32,
    second: u32,
    source: u32,
    value: i32,
) -> [Option<Forward>; 2] {
    if !found {
        return [None, None];
    }
    let make = |target| {
        Some(Forward {
            target,
            source,
            value,
        })
    };
    [make(first), make(second)]
}

/// A sequence counter. `word` holds the count in its high nibble and the
/// last mark in its high byte. `wrong` is the out-of-order flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sequence {
    pub word: u16,
    pub wrong: bool,
    pub first: u32,
    pub second: u32,
}

/// What one sequence step reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceTick {
    pub forward: Option<Forward>,
    /// Listeners of this relay are cleared with a value of 0.
    pub clear_listeners: bool,
}

/// Count one input, or finish the sequence when the mark is 0.
/// A value of 0 does nothing. A missing source does nothing.
pub fn sequence(
    seq: &mut Sequence,
    relay_id: u32,
    input_mark: Option<u16>,
    value: i32,
) -> SequenceTick {
    let Some(mark) = input_mark else {
        return SequenceTick {
            forward: None,
            clear_listeners: false,
        };
    };
    if value == 0 {
        return SequenceTick {
            forward: None,
            clear_listeners: false,
        };
    }
    if mark == 0 {
        let count = (seq.word >> 4) & 0xF;
        let target = if count > 7 && !seq.wrong {
            seq.first
        } else {
            seq.second
        };
        seq.word &= 0xF;
        seq.wrong = false;
        SequenceTick {
            forward: Some(Forward {
                target,
                source: relay_id,
                value: 1,
            }),
            clear_listeners: true,
        }
    } else {
        let previous = u32::from(seq.word >> 8);
        if previous & u32::from(mark) != 0 {
            seq.wrong = true;
        }
        let low = seq.word & 0xFF;
        seq.word = low | mark.wrapping_shl(8);
        seq.word = seq.word.wrapping_add(0x10);
        SequenceTick {
            forward: None,
            clear_listeners: false,
        }
    }
}

/// Whether a finished sequence clears this listener with a value of 0.
pub fn sequence_clears(node_class: u32, flags: u16, listens_to: u32, relay_id: u32) -> bool {
    node_class & LISTENER_MASK == LISTENER_CLASS
        && listens_to == relay_id
        && flags & SKIP_CLEAR == 0
}

fn write_bit(state: &mut u16, bit: u16, value: i32) {
    let mask = 1u32 << (u32::from(bit) & 31);
    let mut bits = u32::from(*state);
    bits &= !mask;
    if value & 1 != 0 {
        bits |= mask;
    }
    *state = bits as u16;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combiner() -> Combiner {
        Combiner {
            state: 0,
            flags: 0,
            target: 9,
            code: 1,
        }
    }

    #[test]
    fn a_relay_sets_a_bit_forwards_both_and_counts_eight() {
        let mut both = combiner();
        assert!(combine(&mut both, 4, None, 1).is_none());
        assert_eq!(both.state, 0);

        let mut on = combiner();
        let sent = combine(&mut on, 4, Some(0), 1).unwrap();
        assert_eq!(on.state, 1);
        assert_eq!(
            sent,
            Forward {
                target: 9,
                source: 4,
                value: 1
            }
        );

        let mut off = on;
        let sent = combine(&mut off, 4, Some(0), 0).unwrap();
        assert_eq!(off.state, 0);
        assert_eq!(sent.value, 0);

        let mut quiet = combiner();
        quiet.flags = SILENCE;
        quiet.code = 0;
        assert!(combine(&mut quiet, 4, Some(1), 1).is_none());
        assert_eq!(quiet.state, 2);

        let mut flipped = combiner();
        flipped.flags = INVERT;
        let sent = combine(&mut flipped, 4, Some(0), 1).unwrap();
        assert_eq!(sent.value, 0);

        let mut other = combiner();
        other.code = 1;
        let sent = combine_else(&mut other, 4, Some(0), 12, 7, 3).unwrap();
        assert_eq!(
            sent,
            Forward {
                target: 9,
                source: 4,
                value: 1
            }
        );
        other.state = 0;
        other.code = 1;
        let sent = combine_else(&mut other, 4, Some(1), 12, 7, 3).unwrap();
        assert_eq!(other.state, 2);
        assert_eq!(
            sent,
            Forward {
                target: 12,
                source: 7,
                value: 3
            }
        );
        other.state = 1;
        assert!(combine_else(&mut other, 4, Some(1), 12, 7, 3).is_none());

        let gone = forward_both(false, 1, 2, 3, 1);
        assert_eq!(gone, [None, None]);
        let pair = forward_both(true, 1, 2, 3, 5);
        assert_eq!(
            pair,
            [
                Some(Forward {
                    target: 1,
                    source: 3,
                    value: 5
                }),
                Some(Forward {
                    target: 2,
                    source: 3,
                    value: 5
                })
            ]
        );

        let mut seq = Sequence {
            word: 0,
            wrong: false,
            first: 20,
            second: 21,
        };
        let tick = sequence(&mut seq, 8, Some(1), 0);
        assert!(tick.forward.is_none());
        assert_eq!(seq.word, 0);

        for mark in [1_u16, 2, 4, 8, 16, 32, 64] {
            let tick = sequence(&mut seq, 8, Some(mark), 1);
            assert!(tick.forward.is_none());
        }
        assert!(!seq.wrong);
        let early = sequence(&mut seq, 8, Some(0), 1);
        assert_eq!(
            early.forward,
            Some(Forward {
                target: 21,
                source: 8,
                value: 1
            })
        );
        assert!(early.clear_listeners);
        assert_eq!(seq.word, 0);
        assert!(!seq.wrong);

        for mark in [1_u16, 2, 4, 8, 16, 32, 64, 128] {
            sequence(&mut seq, 8, Some(mark), 1);
        }
        assert!(!seq.wrong);
        let done = sequence(&mut seq, 8, Some(0), 1);
        assert_eq!(done.forward.unwrap().target, 20);
        assert!(sequence_clears(LISTENER_CLASS, 0, 8, 8));
        assert!(!sequence_clears(LISTENER_CLASS, SKIP_CLEAR, 8, 8));
        assert!(!sequence_clears(0, 0, 8, 8));

        sequence(&mut seq, 8, Some(1), 1);
        sequence(&mut seq, 8, Some(1), 1);
        assert!(seq.wrong);
    }
}
