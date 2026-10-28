## ADDED Requirements

### Requirement: Outdoor scene preset
The outdoor preset SHALL draw from a local generator started at its seed (a zero seed replaced by a fresh one): a day time of 0.32 + 0.42 times the eased draw unless nights are allowed and a six-way roll is zero, then a night time of 0.76 + 0.24 times the draw; it SHALL rain on three of seven rolls.

#### Scenario: Dry day and rainy night
- **WHEN** the preset runs with seed 7 and no nights, and with seed 1 and nights allowed
- **THEN** the first is a dry day at time 0.3605303 with cloudiness 1.251776, fog 0.50763315 and wind 0.53202665; the second is a rainy night at 0.9666516 with rain 0.34562913 and fog 0.3662673

### Requirement: Outdoor preset weather streams
The rain target SHALL be 0.1 + 0.9 times a draw, raised by the rain setter's 0.3 floor before the fog reads it; the setter's thunder draw SHALL come from the engine's shared generator, never the preset's local one; the fog, cloud and wind sums SHALL be computed in double where the original computes them.

#### Scenario: Thunder from the shared stream
- **WHEN** seed 6 draws a rain target of 0.91921914 at time 1000
- **THEN** the next strike is the shared generator's first draw below 30 000 plus 5000, and the local fog, cloud and wind are 0.3195439, 2.3890958 and 0.46616265

### Requirement: Indoor scene preset
The indoor preset SHALL turn the sun light off and the sky off through their setters, set a 50 000 far plane, fog distance 2 (or the variant's 1.8, 1.4 or 2.0), the grey 0.75 global light, a black background, mid-grey fog, no wind and no rain.

#### Scenario: Indoor fog
- **WHEN** the indoor preset is applied
- **THEN** the fog span is 200 100 and the sun mode is off

### Requirement: Level transition placement
A follower SHALL be placed at the player's placed position plus its stored offset taken as a row vector times the spawn matrix's rotation rows, summed first row, second, then third; the offsets SHALL be (−100, 0, −100), (100, 0, −100), (0, 0, 150), (100, 0, 200) and two zeros. A re-attached follower SHALL get controller word 1, a cleared carry word and a minimum distance raised to 5 below 5 (a NaN kept).

#### Scenario: Quarter turn
- **WHEN** the first offset is placed with rows (0, 0, −1), (0, 1, 0), (1, 0, 0) around (10, 0, 20)
- **THEN** the follower lands at (−90, 0, 120)

### Requirement: Scene content lists
A scene node with an object record SHALL join the first list for kind pair 0x1100 or 0x1200, else the second for 0x2200, and independently the third when its 0x10000 property's first byte is 2. A node without a record SHALL join the second list only when its name starts with `[`.

#### Scenario: Two lists
- **WHEN** a node's record has kind pair 0x1100 and a 0x10000 byte of 2
- **THEN** it joins the first and the third list

### Requirement: Scene character flush
Flushing a scene character SHALL register it when its reference has the top bit set, then remap its worn items, its first hand pair, its second hand pair and its carried items in that order; a carried item holding a hand's old id SHALL take that hand's new id, the first pair looked up first. Loadouts SHALL be left alone.

#### Scenario: Hand reuse
- **WHEN** hands 40, 41, 42, 52, worn 45 and carried 50, 41, 52 are flushed with a remap adding 100
- **THEN** the remap sees 45, 40, 41, 42, 52, 50 and the carried items become 150, 141, 152

### Requirement: Settings chunk on a saved level
The story's level exporter SHALL write the settings chunk only for an old read tag or a saved word above zero (unsigned), carrying the read tag when it was old and the current tag (0, 1 100 000, 0) otherwise.

#### Scenario: Gate
- **WHEN** the tag is current and the saved word is 0
- **THEN** no settings chunk is written

### Requirement: Arena merchant and arsenal chunks
The arsenal chunk SHALL be 25 item lists written shelf by shelf with the five boxes inside. The merchant chunk SHALL be skipped only when the staff is non-empty and holds no merchant (class 4).

#### Scenario: Merchant condition
- **WHEN** the table writer runs with no staff, with a staff of classes 2 and 8, and with a staff holding class 4
- **THEN** the merchant chunk is written, skipped, and written
