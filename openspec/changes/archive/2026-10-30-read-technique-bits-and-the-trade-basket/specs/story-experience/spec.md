# Spec Delta

## ADDED Requirements

### Requirement: Technique word
When a skill reaches a new level the body's technique word SHALL be rebuilt from 0: for each of skills 0, 2, 3, 4 and 5 the techniques in its first `points / 5000` slots SHALL set skill 0's bits 0x1, 0x2, 0x4, 0x8 for techniques 1 to 4, skill 2's 0x1000 for technique 5, skill 3's 0x0100_0000 to 0x2000_0000 for techniques 1 to 6, skill 4's 0x0080_0000 for technique 6, and skill 5's 0x0010_0000, 0x0040_0000, 0x0020_0000, 0x0001_0000 for techniques 1, 5, 6, 8. Skill 1 SHALL set no bit.

#### Scenario: A mixed body
- **WHEN** skill 0 at level 2 holds 1, 2, skill 1 holds 1, skill 2 holds 5, skill 3 at level 3 holds 1, 2, 3, skill 4 holds 6 and skill 5 holds 8
- **THEN** the word is 0x0781_1003

### Requirement: Power progress division
The power progress SHALL be divided by 4 as an unsigned 32-bit word.

#### Scenario: A progress above 2^31
- **WHEN** a progress of 0x8000_0000 with a power at level 5 takes 8 more
- **THEN** 0x2000_0002 levels fill the power to 0x1fff and no progress is left

### Requirement: Level start and hooks
The level-start hook SHALL set the player's and the account's clocks to now and the account pool to the player's carried experience. The story SHALL install its hooks in the order new game, level enter, level start, tick, kill, pickup.

#### Scenario: A level starts
- **WHEN** a player carrying 640 experience starts a level at 5000
- **THEN** the pool is 640 and both clocks read 5000

### Requirement: Account experience setting
The account experience SHALL be stored in the settings store as its four little-endian bytes, and an absent or empty value SHALL leave the number held.

#### Scenario: Round trip
- **WHEN** 12345 is saved and read back, and an empty value is read
- **THEN** 12345 returns and the empty value changes nothing
