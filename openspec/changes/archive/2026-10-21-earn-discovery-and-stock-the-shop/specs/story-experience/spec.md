## ADDED Requirements

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
