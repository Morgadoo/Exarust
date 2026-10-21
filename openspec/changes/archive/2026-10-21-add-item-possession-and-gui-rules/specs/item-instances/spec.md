## ADDED Requirements

### Requirement: Container and table sections
The object database reader SHALL read a container section's size, capacity, largest item and 12-byte children, a random table's count range and 16-byte rows, and the model section's bulk and compact price word, returning nothing for a body too short for what it states. This SHALL hold for every container (137) and random table (24) in `objdb.rdb`.

#### Scenario: A chest
- **WHEN** a container body of 65 by 50 units, capacity 200, largest item 50 and two children is read
- **THEN** the sizes and children are those, and a body one byte short is refused

### Requirement: Weapon condition
A copy's condition factor SHALL be (1 − d·0.4/255)·(q·0.5/255 + 0.8) in the original's mix of float and double. A weapon SHALL scale slash and pierce by it, crush by 0.7c + 0.3 and thrust by it, each capped at 160 after the byte wraps.

#### Scenario: A pristine sword
- **WHEN** a sword at quality 255 and no damage takes a template of slash 100 and crush 100
- **THEN** its slash is 130 and its crush 121

### Requirement: Apparel condition
Apparel SHALL scale its resistances by the factor (capped at 160), its coverage by 0.3c + 0.7, divide its encumbrance by (1 − d·0.15/255)(q·0.3/255 + 0.85), then apply its armour variant's uncapped scales.

#### Scenario: Variant 4
- **WHEN** a pristine coat of slash 80 rolled variant 4
- **THEN** its slash is 140: 80 × 1.3 is 104, then × 1.35

### Requirement: Copy price
A copy's price SHALL be its template's divided by 2q₀² + 0.5 and multiplied by (2q² + 0.5)(2(1 − d)² + 0.5)/2, stored back in the compact price word.

#### Scenario: A pristine copy
- **WHEN** a copy of quality 255 and no damage is priced from a 1440-copper template of quality 255
- **THEN** it costs 1800 copper

### Requirement: Rolls
A pending roll SHALL draw a byte within its spread of the stored value, leaning toward it, from the engine's generator. A missing seed SHALL take a fresh one and clear the fixed-choices flag.

#### Scenario: Within the spread
- **WHEN** a value of one half is rolled two hundred times within a spread of a quarter
- **THEN** every byte lies from 64 through 191

### Requirement: Palettes and the apparel roll
A palette SHALL be rolled for a wealth level with three shades and their thresholds. An apparel copy SHALL give each painted layer a material from its random set weighed by wealth level, a colour by its material's class and colour mode, and an armour variant from its type's list with six extra chances of none.

#### Scenario: The same entropy
- **WHEN** an apparel copy is rolled twice with the same palette and entropy word
- **THEN** both rolls give the same materials, colours and variant

### Requirement: Random tables
Containers whose children name a random table SHALL be filled from it: quotas, then count draws among count-only rows, then a weighted draw for every empty slot, passing over rows bigger than the slot allows and descending into nested tables. A row SHALL weigh w·1000 unpicked, (1 − decay·0.005)·w·1000 after one pick, the engine's fast exp(n·ln(1 − decay·0.005))·w·1000 after n, and nothing at its limit.

#### Scenario: A decaying row
- **WHEN** a row of weight 10 and decay 100 is weighed after 0, 1, 2 and 3 picks
- **THEN** it weighs 10000, 5000, 2500 and 1250
