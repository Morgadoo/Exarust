## ADDED Requirements

### Requirement: Writing a character record
The system SHALL write a character record as the original's writer does: version 7, the fields in file order, names one byte per character, a skill only when it has a chosen technique and at most five techniques, the powers only with an entry, the reserved pair only when its first word is set and the loadouts only when there is one. Every version 7 record of the shipped level databases and `actors.rpk` SHALL write back byte for byte except the voice names that are not in the formatter's form.

#### Scenario: Shipped records
- **WHEN** the 485 version 7 records of the installed levels and actors are written back
- **THEN** 483 are byte for byte the same and the other two differ only in a stored voice name `HM04` written as `vi04`

### Requirement: Trait and voice fields written
The system SHALL write each trait as the nearest-even conversion plus 100 in a byte (NaN and values out of range storing 100) and the voice name as `vi` and the voice number plus one by the original's formatter (two digits at least, `00` for 0, `0` for a negative sum).

#### Scenario: Traits and voice
- **WHEN** a record with traits 2.5, −100.5, 300 and NaN and voice numbers 0, 99 and 2³²−1 is written
- **THEN** the trait bytes are 102, 0, 144 and 100 and the voice names `vi01`, `vi100` and `vi00`

### Requirement: Record defaults
The system SHALL start a new record blank with yellow and red full, focus 0 and voice pitch 1, and the character creator's reset SHALL set a male human, physique 0.5, skin 0x8000, hair style 0, hair colour `40 80 40 00`, voice 0, pitch 1, yellow and red full, and append the underwear, leaving every other field as it was.

#### Scenario: Reset of a record in use
- **WHEN** a record with a name, focus 0x1234 and one worn item is reset
- **THEN** it keeps the name and focus and wears that item and then the underwear

### Requirement: Saved health
The system SHALL save yellow and red as value / maximum × 65535 and focus × 65535 in single precision, converted to the nearest even integer and cut to 16 bits (red 0 unless positive), with the seed 0xFFFF and the heal pool and potion entries; it SHALL load them, and spawn characters with them, as max × (stored × 1/65535) in double precision, every known entry taking the last entry's value.

#### Scenario: Double-precision load
- **WHEN** a share of 257 is loaded onto a maximum of 0.25
- **THEN** the health is the `f32` nearest 0.25 × 257 / 65535, not the single-precision product

#### Scenario: Save
- **WHEN** a body at half yellow, a quarter red and focus 0.75 of maximum 1 is saved
- **THEN** the pools store 32768, 16384 and 49151

### Requirement: Equip conflicts
The system SHALL refuse an equip without change when a worn item that overlaps the new item on any bone has an equip lock over 50, and otherwise take every overlapping worn item off, in order.

#### Scenario: Golem suit
- **WHEN** a character wearing the shipped golem suit equips an apparel item that overlaps it
- **THEN** the equip is refused and the suit stays on

### Requirement: Trading an item away
The system SHALL take the last inventory copy not marked equipped, else the last copy, else the worn copies through the original's skipping loop, and then clear the surplus hand and loadout references in both hand pairs.

#### Scenario: Trade
- **WHEN** an item held as an equipped and an unequipped inventory copy is traded away
- **THEN** the unequipped copy goes

### Requirement: Unlearning a level
The system SHALL remove a chosen level's technique, then the techniques of the same skill that depend on a removed one by the original's prerequisite table, and take 5000 points per technique removed in 16-bit arithmetic.

#### Scenario: Manoeuvring in Armour
- **WHEN** Manoeuvring in Armour I is unlearnt from Armour holding I, Deflection, II, III and Security at 25000 points
- **THEN** Armour holds Deflection and Security at 10000 points
