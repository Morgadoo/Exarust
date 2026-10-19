# Spec Delta

## ADDED Requirements

### Requirement: Relay combiner
A combiner SHALL set the source's bit when the value's low bit is 1 and clear it otherwise. A missing source SHALL leave it unchanged. When the state then equals the code it SHALL signal 1, or 0 when the invert flag is set. A mismatch SHALL signal the opposite, unless the silence flag is set, and the bit SHALL still be written.

#### Scenario: A bit, a match, and a silenced mismatch
- **WHEN** a missing source is applied, bit 0 is set against code 1, that bit is cleared, bit 1 is set while silenced against code 0, and bit 0 is set with the invert flag
- **THEN** the missing source leaves the state at 0, the match signals 1, the clear signals 0, the silenced state is 2 with no signal, and the inverted match signals 0

### Requirement: Relay mismatch
The second combiner SHALL write the same bit. A match SHALL signal 1 to its first target. A mismatch SHALL forward the original value to the source's second target when the state's low bit is clear, and SHALL do nothing when that bit is set.

#### Scenario: Match, forward, and a held low bit
- **WHEN** bit 0 is set against code 1, bit 1 is set against code 1 with the source's second target 12 and value 3, and the same bit is set while the state's low bit is already set
- **THEN** the match signals 1, the mismatch forwards 3 to target 12, and the held low bit sends nothing

### Requirement: Relay pair and sequence
A pair SHALL send the same value and source to both targets, and a missing pair SHALL send nothing. A sequence SHALL ignore a value of 0. A marked input SHALL count one step and store that mark in the high byte, and SHALL flag a wrong order when the mark overlaps the previous one. Eight steps without a wrong order SHALL signal the first target with 1 and reset. Fewer steps, or a wrong order, SHALL signal the second target with 1 and reset.

#### Scenario: Both targets, seven steps, and eight
- **WHEN** a missing pair is asked to forward, a pair forwards 5, a sequence is given value 0, seven distinct marks are finished, and eight distinct marks are finished
- **THEN** the missing pair sends nothing, both targets receive 5, value 0 changes nothing, seven steps signal the second target, and eight steps signal the first

### Requirement: Sequence listener
A finished sequence SHALL clear a listener whose class bits match and whose flags lack the skip bit, and SHALL leave any other listener.

#### Scenario: Who a finish clears
- **WHEN** a finish looks at a matching listener, one with the skip bit, and one whose class bits do not match
- **THEN** only the matching listener is cleared
