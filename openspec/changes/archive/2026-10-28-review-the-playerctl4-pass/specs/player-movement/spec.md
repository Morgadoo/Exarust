## ADDED Requirements

### Requirement: Item cursor placement sweep
The system SHALL slide a placed lifted item along the cursor ray from 600 cm before the picked point (never before the ray's origin) and stop it at its first contact with the level when that is nearer than the pick; a sweep that starts inside the level SHALL count as no contact.

#### Scenario: Wall before the pick
- **WHEN** the item's sphere meets a wall 50 cm before the picked point
- **THEN** the item stops at that contact

### Requirement: Item cursor hover word
The system SHALL give a lift's hover word as 0x30 when the item snaps onto a level placement marker whose link byte reads `L`, 0x10 for another marker, 1 for a free placement that passed its checks, and 0 otherwise.

#### Scenario: Plain marker
- **WHEN** the item snaps onto a marker whose link byte is not `L`
- **THEN** the hover word is 0x10

### Requirement: Use release dispatch
The system SHALL resolve a use's release over the world view as: a locked door unlocks when the held item is its key and rattles otherwise, an unlocked or keyless door does nothing; a character takes a held liquid (kinds 8 and 14); a world item with an item id is cleaned (kind 9) or repaired (kind 10). Over the GUI a character view SHALL take the liquid and an item container window SHALL apply the GUI use.

#### Scenario: Unlocked door
- **WHEN** a key is used on a door that is not locked
- **THEN** nothing happens

### Requirement: Game mode registration
The system SHALL register the Character mode and then the Spectate mode, each id the mode table's length after its append plus two.

#### Scenario: Empty table
- **WHEN** the table has no entries
- **THEN** Character gets id 3 and Spectate id 4

### Requirement: GUI drag spawned record
The system SHALL spawn a GUI drag's record only for the two drag events, both alike, carrying the dragged entry's item id and the drag's condition.

#### Scenario: Object drag event
- **WHEN** the second drag event spawns an entry of item 42 with condition 3
- **THEN** the record is item 42 with condition 3
