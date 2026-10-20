## ADDED Requirements

### Requirement: Tree selection roots
With neither word positive the selection SHALL return the base result. In mode 3 a new or current word of 0x1000000 or 0x2000000 SHALL store the new word and return 3.

#### Scenario: No selection
- **WHEN** the new and current words are 0
- **THEN** the base result returns and nothing else changes

#### Scenario: The special roots
- **WHEN** the mode is 3 and the new word is 0x1000000
- **THEN** it is stored and the result is 3

### Requirement: Tree selection candidate
The candidate SHALL be 0 for bit 0x400000, the low 16 bits for a word in 1..0xfffe with a valid table entry, else 0. The new word SHALL be stored. An equal last focus SHALL return 0; an equal previous selection SHALL return 3; a fresh candidate SHALL return 3, store the focus, set the previous when positive and stamp the clock.

#### Scenario: The selection paths
- **WHEN** the new word is 5, the table is valid and the last focus is 5
- **THEN** the result is 0 and the focus is unchanged
- **WHEN** the last focus is 0 and the previous selection is 5
- **THEN** the result is 3 and the focus stays 0
- **WHEN** the last focus and the previous selection are 0
- **THEN** the result is 3, the focus and previous become 5 and the clock is stamped
- **WHEN** the new word carries 0x400000
- **THEN** the candidate is 0
