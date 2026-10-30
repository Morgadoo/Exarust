# character-databases Specification

## Purpose
The local and world character databases: how character records are looked up, shared, added, released, changed, flushed, saved and loaded.

## Requirements

### Requirement: Character database lookup
The local and world character databases SHALL keep id-sorted entries, find an id by binary search, parse an entry's bytes into a shared object on its first lookup (one holder, its id tagged with the database's 0x80000000 or 0x40000000), and on later lookups return the same object with one more holder; an id with the world bit SHALL be looked up in the world database, any other in the local one, masked to its low 28 bits.

#### Scenario: Two lookups
- **WHEN** a stored world character 7 is looked up twice
- **THEN** both lookups return one object with id 0x40000007 and two holders

### Requirement: Character database changes
Adding a character SHALL store it live with an empty entry under the given id or the first free one from 1; releasing a character SHALL forget its live object in the database its id's tag names (world bit, else local bit, else none) and clear its id; a payload of the entry's own length SHALL be written in place, another length SHALL be appended after removing the old bytes and pulling the later offsets back; a flush SHALL re-serialise every live character.

#### Scenario: A longer payload
- **WHEN** entries 1, 2 and 3 hold 2, 3 and 1 bytes and entry 1 is set to one byte
- **THEN** the payload holds entry 2's, entry 3's and then the new byte

### Requirement: Character database save
The save SHALL write the magic 0xDBACCD00, the table size (count + 1) × 16, and, with entries, the table with a sentinel entry and the payload, without flushing; the load SHALL count table size / 16 − 1 entries, an empty database below two.

#### Scenario: An empty save
- **WHEN** an empty database is saved
- **THEN** the save is the 8-byte header and loads back empty

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
