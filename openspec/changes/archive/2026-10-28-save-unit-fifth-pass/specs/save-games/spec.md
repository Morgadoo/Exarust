## ADDED Requirements

### Requirement: World database export
A save SHALL write `#worldchrdb` only when the world character database holds a record, as one chunk `0xCECDDB00` with the database flushed and then saved (magic `0xDBACCD00`, table size `(records + 1) × 16`, table and payload); it SHALL write `#worldobjdb` only when the object world tier holds a record, as chunk `0xDBDB` with the tier's writer followed by chunk `0xDBCD` with its string library's writer.

#### Scenario: One character
- **WHEN** the world character database holds one live record
- **THEN** the export begins with `0xDBACCD00` and a table size of 32 and the save's import reads the record back

#### Scenario: Empty databases
- **WHEN** both databases are empty
- **THEN** neither section is exported

### Requirement: Player placement export
The player-data writers SHALL export the player's placement in save mode under the player's record remembered at the first save, whatever record the body holds, and SHALL flush the record first unless the transition state's first word is 2; the placement SHALL be the first chunk after the header.

#### Scenario: Pending respawn
- **WHEN** a later save runs with the transition word 2 and the body holding another record
- **THEN** the placement carries the first record and the flush is skipped

### Requirement: Arena list regeneration
After an arena load's id pass each employee whose profession byte is 4 SHALL have its own list emptied; every other employee and then the manager SHALL have each loadout's first two non-zero hand ids and its worn ids other than 8 added by a unique append of `(id, 0, 0x28 | 4 << 16)`, the special duplicates removed and each entry shelved with flag `0x400` when its shelf holds no copy; every arsenal box SHALL then be deduplicated.

#### Scenario: Worn id eight
- **WHEN** the manager's loadout wears ids 8, 12 and 11 and holds 11
- **THEN** 12 and 11 are listed once each and 8 is not

### Requirement: Live transition flow
Each in-process level change SHALL record the transition flow with no save package open: an archive destination that exists SHALL read the archive and import, a non-zero spawn SHALL choose the portal matrix, and a destination missing from the archive SHALL abort after the save is opened.

#### Scenario: Missing level
- **WHEN** a change names a level the archive lacks
- **THEN** the recorded flow is aborted and saves stay enabled
