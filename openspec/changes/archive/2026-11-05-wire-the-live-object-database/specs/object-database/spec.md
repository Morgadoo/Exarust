# Spec Delta

## ADDED Requirements

### Requirement: Live local tier from a level
A level load SHALL make the live database's local tier the level's `0xDBDB` records as stored (table, flags and blob) and its library the level's `0xDBCD` table, replacing the previous level's and leaving the base and world tiers alone. A copy made in the local tier afterwards SHALL take the first id the level's records leave free. Every level of `Resource.rpk` with a local database SHALL read as such a tier.

#### Scenario: A copy after the level's records
- **WHEN** `exanima01` is loaded (local ids 1 to 10 taken, 11 free) and a marked template is instantiated
- **THEN** the copy's id is `0x8000000B`

### Requirement: World tier import and export
The world tier SHALL export as its records' writer and its own library's writer, and only when it holds a record. An import SHALL replace the world tier and its library with a save's and migrate every record from its template; a save without the section SHALL clear them.

#### Scenario: Round trip
- **WHEN** a world tier with one record and one own string is exported and imported into another database
- **THEN** the record and the string come back and the migration visits the record once

### Requirement: Carried items promotion
The story's sync SHALL promote a character's worn list, then the out pair (hands 0 and 1), then the stowed pair (hands 2 and 3), then each pack entry, an entry naming a hand's old id taking that hand's new id (the out pair first) and any other being promoted itself; ids that are not local SHALL stay.

#### Scenario: A held item repeated in the pack
- **WHEN** hand 0 holds local 1, hand 2 local 2, the worn list local 3, and the pack lists local 1, local 2 and local 4
- **THEN** the worn item becomes world 1, the hands world 2 and world 3, and the pack world 2, world 3 and world 4

### Requirement: Record promotion
The arena's record promotion SHALL promote the four hands, the worn list, each loadout's four hands and worn list, then each pack entry on its own; with the id remap table on, an id promoted earlier SHALL give its recorded world id.

#### Scenario: A hand repeated in the pack
- **WHEN** hand 0 and the pack both name local 1 and the id remap table is on
- **THEN** both become world 1

### Requirement: Container children write
Writing a container's children into a record SHALL resize its container section to `0x18` plus twelve bytes per child, keep the section's first bytes, and store the count at `0x14` and each child's id, flags, x and y; a record or section that is missing SHALL be left alone.

#### Scenario: Two children
- **WHEN** a local chest of width 4 is given two children
- **THEN** it reads back width 4 with those two children
