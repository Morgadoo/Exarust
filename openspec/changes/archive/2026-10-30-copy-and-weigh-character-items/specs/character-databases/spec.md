# Spec Delta

## ADDED Requirements

### Requirement: Character entry walks
The whole-entry reader SHALL step eight bytes at a time after the record without skipping chunk bodies while the bytes consumed plus six are fewer than the entry's length, SHALL make a new database at each items header and SHALL replace the current database's library at each strings header. The item import's reader SHALL skip each chunk's body, SHALL walk while the bytes consumed plus eight are fewer than the length, and SHALL load strings and items into one database.

#### Scenario: Nested items header
- **WHEN** an items chunk lies inside another chunk's body on the eight-byte step
- **THEN** the whole-entry reader loads it

#### Scenario: Library before items
- **WHEN** a strings chunk precedes the items chunk
- **THEN** the whole-entry reader's database has no strings and the import reader's has both

### Requirement: Character item import
The item import SHALL first migrate every record of the entry's database from the live templates, then copy the first hand pair, the worn list and the inventory into the target tier, an inventory entry repeating a first-pair hand taking that hand's new id and every other tagged entry being copied.

#### Scenario: Repeated hand
- **WHEN** the inventory repeats the first hand's item
- **THEN** the inventory entry takes the hand's new id and no second copy is made

### Requirement: Equip weight
The equip weight SHALL be the new item's armour weight less the armour weight of every worn item with a body whose occupancy overlaps it on any bone, and SHALL be 0 when one such item's equip lock exceeds 50.

#### Scenario: Locked overlap
- **WHEN** an overlapping worn item has an equip lock of 51
- **THEN** the equip weight is 0
