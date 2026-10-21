## ADDED Requirements

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
