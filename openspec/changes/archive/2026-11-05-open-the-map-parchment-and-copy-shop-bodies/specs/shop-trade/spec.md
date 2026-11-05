# Spec Delta

## MODIFIED Requirements

### Requirement: Live shop world
The running game's shop world SHALL read the object database's table once, taking each record's id, the table's flags word as its kinds and its shop fields, and SHALL make apparel and weapon copies in the live object database. A copy's source SHALL be its header's parent word, and a copy dropped from the shelf SHALL be removed from its tier. Without a base tier the copies SHALL be stand-in ids under the instance bits that remember their source.

#### Scenario: Table flags
- **WHEN** a database record's table flags word is 0x4000 and its payload holds a model and a name section
- **THEN** the entry's kinds are 0x4000 and its rarity is the model's

#### Scenario: Stand-in copies
- **WHEN** an apparel and a weapon copy are made from records 10 and 20 with seed 7 and no base tier
- **THEN** they are the first and second instance ids, their sources are 10 and 20, and releasing the first forgets only its source

## ADDED Requirements

### Requirement: Live apparel copy
An apparel copy SHALL be the record's instance (or the record itself when it has no instance bit). Its body's condition byte SHALL take a quality `2u − 1` from the shared seed, with damage 0x40 and dirt 0x35 and their spreads; the apparel palette SHALL be rolled for the body's condition plus quality; then the copy SHALL be refreshed from its template: value word, paint seed when it has none, protection bytes, the layers' and variant's roll, the condition pass.

#### Scenario: An apparel copy's own body
- **WHEN** marked apparel record 10 is copied with seed 7
- **THEN** the copy is local 1 naming 10, its condition byte is the one seed 7 alone gives, and record 10's body is unchanged

### Requirement: Live weapon copy
A weapon SHALL be copied only when its header word has the instance bit and its model is a factory's (`@…`). The copy SHALL be graded at level 1 and refreshed from its template: value word, template bytes, the condition's roll, the factory's stats, the condition pass. Any other weapon SHALL be stocked as itself.

#### Scenario: A weapon copy
- **WHEN** a marked `@sword_x` record and an unmarked plain weapon are copied
- **THEN** the first is local 1 with its grade's damage and dirt drawn and the factory `sword_x` asked, and the second is stocked as itself
