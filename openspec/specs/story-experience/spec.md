# story-experience Specification

## Purpose
Story experience as the original earns it: discovery points from new item kinds, kills and explored map cells, their slow conversion into skill experience, techniques and power levels for the player and the companions, the account-wide experience, and the discovery data a save keeps.

## Requirements

### Requirement: Discovery points
In story mode a new item kind SHALL earn the points of its model's rarity tier (10, 20, 50, 100, 200, 500, 1000, 2000 for tiers 0 to 7) once, and a kind without a model SHALL earn nothing and stay unseen. A killed character SHALL earn the points of its rank. Every point SHALL go once to the account pool and four times to the player and to each companion.

#### Scenario: A new kind and a kill
- **WHEN** a kind of tier 3 is picked up twice, then a character of rank 5 is killed
- **THEN** the first pickup earns 100 (pool 100, the player and each companion 400), the second nothing, and the kill 500

### Requirement: Explored cells
The player's position SHALL fall in a cell of 600 units, rounded down, offset by half the grid. When that cell lies in the grid and in a sector, each not yet explored cell of the 5 × 5 around it, inside the grid's one-cell border, whose flag 0x80000 is set and whose sector is open, SHALL earn 10 points and be marked in the level's bitmap of `width / 32` words a row.

#### Scenario: A first step
- **WHEN** the player stands at the origin of a 10 × 10 grid with one hidden cell, twice
- **THEN** 24 cells earn 240 points the first time and nothing the second

### Requirement: Skill conversion
Once a second a character SHALL convert, at a cost of 4 plus its skill levels a point, the whole points due for 50 ms of that cost since its clock, at most what its experience pays. Each point SHALL add 2 to the trained skill (the first under level 5 whose technique at its level is chosen); a skill reaching a new multiple of 5000 SHALL snap to it. Too little experience SHALL restart the clock.

#### Scenario: Ten points
- **WHEN** a companion with 400 experience and its clock at 1000 converts at 3000
- **THEN** 10 points cost 40, add 20 to the trained skill, and the clock reads 3000

### Requirement: Companion level
A companion SHALL convert only in its active body state. At a new level it SHALL gain 10 combat skill and 10 courage, each at most 100. With no technique in training it SHALL take the next from the fixed table by its total level.

#### Scenario: A companion levels
- **WHEN** a companion at 4990 with 20 experience converts after 1000 ms
- **THEN** 5 points reach 5000, combat skill 95 stops at 100, courage reaches 10, and the next technique is Armour's second

### Requirement: Power training
The player's paid cost SHALL feed the first power flagged as in training: every 4 SHALL add a level to its low 12 bits, and a level that would reach 4096 SHALL leave the word 0x1fff.

#### Scenario: Two points
- **WHEN** a player with 1000 experience and a power at level 5 converts after 400 ms
- **THEN** 2 points cost 8 and the power reaches level 7

### Requirement: Account experience
The account pool SHALL convert at `(global / 5000 + 1) × 4` a point, adding 2 global experience a point, and SHALL stop at 25000, emptying the pool there. Global experience SHALL be worth its 5000-steps at 4/4, 5/4, 6/4 and so on, rounded down, and that worth SHALL cost the same experience back.

#### Scenario: Worth
- **WHEN** 12000 global experience is valued and its worth is costed
- **THEN** it is worth 14250 and 14250 costs 12000

### Requirement: Skill lump
A lump of experience SHALL add its amount divided by one more than the six skill levels, at most 5000, to the trained skill, and nothing when that rank exceeds the amount. The arena's lesson SHALL use it.

#### Scenario: A ranked lesson
- **WHEN** a fighter with five skills at level 1 takes a lump of 1000
- **THEN** the trained skill gains 166

### Requirement: Discovery save data
The discovery data SHALL hold the item kinds seen, then each level's 16-byte name, width, height and bitmap. A level entry with an empty name SHALL end the list there and mark the data recovered; short or negative data SHALL be an error.

#### Scenario: Round trip and recovery
- **WHEN** two kinds and one level map are written and read back, and a second, empty entry follows
- **THEN** the data reads back equal, and the empty entry keeps the first map and marks the data recovered

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

### Requirement: Live discovery pass
In the running story, each of the flow's once-a-second discovery passes SHALL convert the account pool, the player and each companion at the game clock, and SHALL write the player's remaining experience, skills and powers back to the player's record.

#### Scenario: One pass after the level start
- **WHEN** a player carrying 600 experience, training its first skill from 5000 points, starts at 0 and a pass runs at 1000
- **THEN** the record holds 580 experience and the first skill 5008 points
