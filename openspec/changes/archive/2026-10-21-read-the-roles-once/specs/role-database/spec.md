## MODIFIED Requirements

### Requirement: Entry sections
The system SHALL read an entry's target, condition and value (attitude in byte 0, tolerance in byte 1) and its sections in the stored order: the unknown block, the dialogue graph, the encounter script, two unknown blocks and the interaction condition, each present when its kind bit is set. The corpse script (kind bit 0x20) SHALL be read after the sections of bits 0x1 to 0x8 only, where the original's corpse accessor reads it.

#### Scenario: A dialogue and an interaction condition
- **WHEN** an entry of kind 0x1042 aimed at role 0x1E holds a graph and the text `result = 1;`
- **THEN** its attitude and tolerance are 100, it has a dialogue aimed at 0x1E and its interaction condition is `result = 1;`

#### Scenario: The corpse script beside an interaction condition
- **WHEN** an entry of kind 0x60 holds one length-prefixed text `result = 1;`
- **THEN** both its corpse script and its interaction condition are `result = 1;`

## ADDED Requirements

### Requirement: Level locale
The system SHALL give a level the id of the `locales.rdb` record whose 20-byte name equals the level's name without its extension, ignoring case, and none for an unknown name or a bad database.

#### Scenario: The first story level
- **WHEN** the locale of `exanima01.rfc` is looked up in a database holding `Exanima` (1) and `Exanima01` (2)
- **THEN** it is 2
