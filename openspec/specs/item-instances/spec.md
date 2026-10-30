# item-instances Specification

## Purpose
Copies of items: the containers and random tables that make them, the condition, price and rolls that set them apart from their templates, and the palettes and materials they are painted with.

## Requirements

### Requirement: Container and table sections
The object database reader SHALL read a container section's size, capacity, largest item and 12-byte children, a random table's count range and 16-byte rows, and the model section's bulk and compact price word, returning nothing for a body too short for what it states. This SHALL hold for every container (137) and random table (24) in `objdb.rdb`.

#### Scenario: A chest
- **WHEN** a container body of 65 by 50 units, capacity 200, largest item 50 and two children is read
- **THEN** the sizes and children are those, and a body one byte short is refused

### Requirement: Weapon condition
A copy's condition factor SHALL be (1 − d·0.4/255)·(q·0.5/255 + 0.8) in the original's mix of float and double, and the factory weapons' stat pass SHALL use the same factor. A weapon SHALL scale slash and pierce by it, crush by 0.7c + 0.3 and thrust by it, each capped at 160 after the byte wraps.

#### Scenario: A pristine sword
- **WHEN** a sword at quality 255 and no damage takes a template of slash 100 and crush 100
- **THEN** its slash is 130 and its crush 121

#### Scenario: A byte past 255
- **WHEN** a factory weapon at quality 255 and no damage has a pierce byte of 200
- **THEN** its pierce is 4, the low byte of 260, and the cap leaves it there

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

### Requirement: Count draws
A random table SHALL make its minimum plus a draw below its maximum less its minimum plus one count draws, at most its empty slots. The draw's span SHALL be taken as signed, so that a maximum below the minimum wraps as the original's does.

#### Scenario: An inverted count span
- **WHEN** a table of minimum 3 and maximum 1 with one count-only row fills five slots
- **THEN** it makes two count draws

### Requirement: Factory grade gate
The grade routine SHALL copy and grade only a record whose name section's flag word has bit 0 and whose model starts with `@`, return the id unchanged for another record, and return 0 for a record it cannot read or without a name section.

#### Scenario: Ungraded
- **WHEN** a factory weapon's name flag word is 0
- **THEN** its own id is returned and nothing is copied

### Requirement: Picked copy condition
A picked copy's item body, checked before its weapon section, SHALL take the row's damage, dirt and spreads in its appearance block and a palette byte of its low two bits, the magnitude of 31 × (row wealth − those bits) rounded to nearest even in bits 2 and up, and the top bit for a negative difference.

#### Scenario: The palette byte
- **WHEN** a row of wealth 0 is applied to an apparel copy whose palette byte is 1
- **THEN** the byte becomes 0xFD

### Requirement: World-load refresh walk
The world-load walk SHALL refresh an item-body record whose body byte 0x1A8 is not zero, and every weapon record.

#### Scenario: A zero byte
- **WHEN** an apparel record's body byte 0x1A8 is 0
- **THEN** it is not refreshed

### Requirement: Protection figure order
The protection figure SHALL add, for each of the twenty bones with a nonzero coverage word c and weight w of the same index, the single-precision resistance sum times w·(√(c·0.01)·0.005) computed in double and rounded to single.

#### Scenario: First bone
- **WHEN** only the first coverage word is 200 and the resistances are 1, 0, 0, 0
- **THEN** the figure is √2·0.005 rounded to single

### Requirement: Repaired copy refresh
After a repair's timed write the system SHALL refresh a made copy from its template: the copy SHALL take the template's model price word (doing nothing when only the copy has a model section); an apparel copy's protection, coverage and encumbrance SHALL be the template's at the copy's quality, damage and variant, its price based on the template's quality; a weapon copy SHALL take the template's weapon bytes 0x30 to 0x3F and be scaled by its own condition, its price based on its own quality.

#### Scenario: A repaired weapon copy
- **WHEN** a quality 255, damage 0 weapon copy is refreshed from a template whose side and thrust bytes are 100
- **THEN** its slash, pierce and thrust are 130 and its crush 121, and its adjust and skill bytes stay its own
