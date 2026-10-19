## ADDED Requirements

### Requirement: Unit draw
A unit generator SHALL fill 624 words from a seed, then twist before the first word. Mixing shifts by 11, 7, 15 and 18. Seed 5489 SHALL yield words 0xd091bb5c and 0x22ae9ef6. The unit SHALL be that word times 2 to the power -32. A mark that differs from the stamp SHALL seed from the mark and reverse the mark.

#### Scenario: Seed 5489 and a mark that does not match
- **WHEN** a generator is seeded with 5489 and two words are drawn, and another seeded with 99 has its mark set to 5489 and its stamp left at 0
- **THEN** the words are 0xd091bb5c and 0x22ae9ef6, the first unit's bits are 0x3fea12376b800000, the index is 2, and the mismatched draw yields 0xd091bb5c with the mark and the stamp reversed

### Requirement: Plain field copy
A record copy SHALL copy the bytes before each plain field, the field itself, and the tail through the record size. A short buffer, a field that starts before the previous end, or a field past the size SHALL be an error. The source SHALL stay unchanged.

#### Scenario: Gaps, a field, and the tail
- **WHEN** eight bytes 0x11 through 0x88 are copied with a field at 1 of width 2 and a field at 5 of width 1, a three-byte buffer is copied as size 8, and a field at 0 follows a field at 1
- **THEN** the copy is 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, the source is unchanged, and both bad copies are errors
