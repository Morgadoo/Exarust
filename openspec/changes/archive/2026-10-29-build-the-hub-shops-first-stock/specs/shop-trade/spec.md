## ADDED Requirements

### Requirement: Live shop world
The running game's shop world SHALL read the object database's table once, taking each record's id, the table's flags word as its kinds and its shop fields, and SHALL make apparel and weapon copies as new instance ids that remember their source.

#### Scenario: Table flags
- **WHEN** a database record's table flags word is 0x4000 and its payload holds a model and a name section
- **THEN** the entry's kinds are 0x4000 and its rarity is the model's

#### Scenario: Copies
- **WHEN** an apparel and a weapon copy are made from records 10 and 20 with seed 7
- **THEN** they are the first and second instance ids, their sources are 10 and 20, and releasing the first forgets only its source
