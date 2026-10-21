## ADDED Requirements

### Requirement: Technique pick
The technique selector SHALL put a picked technique into its skill slot of the character record's six skills: an empty slot first forgets the technique in training (the first skill whose slot past its level holds one); with a positive pool a slot nothing was picked into before gains the unspent experience held to 0…5 000, or under the weighted rule `pool · 4 / (4 + Σ levels)` capped at 5 000 while the pool loses `gain · divisor / 4`.

#### Scenario: A first pick
- **WHEN** skill 1 holds 15 000, skill 2 10 000, the pool is 40 000 and a technique is picked into skill 1's empty third slot
- **THEN** skill 1 holds 20 000 and the pool stays 40 000

#### Scenario: The weighted rule
- **WHEN** the same record picks under the weighted rule into skill 2's first slot
- **THEN** skill 2 holds 15 000 and the pool is 28 750

### Requirement: Technique forget
Forgetting the technique in a slot followed by filled slots SHALL close the slots up, forget every technique among the first four that names a forgotten one, and take 5 000 from the skill for each; forgetting the last filled slot SHALL clear it and set the skill to one level less than the techniques it held. Outside the weighted rule a positive pool then refunds into the technique in training, at most to its level.

#### Scenario: A dependent chain
- **WHEN** skill 3 holds 20 000 with techniques 1, 2, 3, 4 and its first slot is forgotten
- **THEN** it holds technique 4 alone and 5 000

### Requirement: Technique selector keys
The technique selector SHALL pass a key to the base form only while a technique is picked in the confirmation mode or while row code 9 is hovered with the lock byte clear, and close with 0x40 otherwise.

#### Scenario: A plain key
- **WHEN** technique 3 is picked in the list mode and a key arrives
- **THEN** the form closes with 0x40

### Requirement: Compact skill hit
The compact skill screen SHALL hit-test a point against each row whose line plus six units is not above it, take its first cell strictly within five units, let a later row override, and return `row << 8 | column` with both counted from 1.

#### Scenario: The first cell
- **WHEN** the margin is 4 units, the spacing 24, the unit 4 and the point is (24, 80)
- **THEN** the hit is 0x101

### Requirement: Compact skill wheel
The compact skill screen's wheel SHALL take, for a mostly horizontal wheel, the row whose line is nearest the pointer and move one column that way, and for a vertical one the row below the pointer (wheel down) or above it (wheel up), skipping rows without slots.

#### Scenario: Wheel down
- **WHEN** the pointer is on row 1's line, row 2 has no slots and the wheel moves down
- **THEN** row 3 is taken

### Requirement: Loadout cost and equip
The loadout editor SHALL cost a weapon at its points less what it displaces from the hands, and equip an item only when a copy is free of the hands, its rank is at most the form's rank (5 when unset) and the points used plus its cost are within the rank's limit; a champion takes any item.

#### Scenario: Beside a sword
- **WHEN** the first hand holds a one-handed weapon of 4 points, the second a shield of 3, and an off-hand-capable one-handed weapon of 5 is dropped
- **THEN** it costs 2

### Requirement: Loadout layout
The loadout editor SHALL lay the arsenal 228 pixels wide at (4, 180), `492 · opacity` tall rounded half to even (the opacity 0.83739835 on a wide interface), and the equipment 236 pixels wide at y 88 four pixels from its parent's right edge, 628 tall or 548 on a wide interface.

#### Scenario: A wide interface
- **WHEN** the interface is wide
- **THEN** the arsenal is 412 pixels tall

### Requirement: Divination reading
The divination reading SHALL darken the screen to alpha 128 over a second, deal its three cards over 600 ms each rising by the ease `(3 − 2t)·t²`, turn them over 900 ms each from 1.8 s by half a turn eased the same way, and hold them face up from 4.5 s, sounding `PprMov` at each deal step and at the first three turns.

#### Scenario: Half way through a deal
- **WHEN** 300 ms have passed at a unit of 6
- **THEN** the first card is at y 50

### Requirement: Destination ring
The destination screen SHALL place row `i > 0` at the angle `(1 − turn) + (2.5 − i)·2π/(count − 1)`, `unit · sin · 40` across and `unit · cos · 40` down from the centre (each rounded half to even, then scaled by the turn), reveal over 500 ms with the alpha eased once and the turn twice by `x(2 − x)`, and hit only the three orbit nodes within 14 units.

#### Scenario: The first orbit node
- **WHEN** four rows sit on a 200 × 100 screen at unit 4, fully revealed
- **THEN** row 1 is at (100, −110)
