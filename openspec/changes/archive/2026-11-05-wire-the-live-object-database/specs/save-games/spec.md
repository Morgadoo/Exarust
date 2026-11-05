# Spec Delta

## MODIFIED Requirements

### Requirement: World database export
A save SHALL write `#worldchrdb` only when the world character database holds a record, as one chunk `0xCECDDB00` with the database flushed and then saved (magic `0xDBACCD00`, table size `(records + 1) × 16`, table and payload); it SHALL write `#worldobjdb` only when the object world tier holds a record, as chunk `0xDBDB` with the tier's writer followed by chunk `0xDBCD` with the writer of the world tier's own string library.

#### Scenario: One character
- **WHEN** the world character database holds one live record
- **THEN** the export begins with `0xDBACCD00` and a table size of 32 and the save's import reads the record back

#### Scenario: Empty databases
- **WHEN** both databases are empty
- **THEN** neither section is exported

## ADDED Requirements

### Requirement: World library import
A load SHALL read the `#worldobjdb` section's second chunk into the world tier's own string library, not the shared one.

#### Scenario: The world library
- **WHEN** the world tier holds a record and its library the text "rusty key"
- **THEN** the save's import has that text in the world tier's library and the shared library stays empty

### Requirement: Live world sync
A story save, the checkpoint's save and a level change SHALL first promote the player's local items to the world tier with the link remap table on and then rewrite the local doors' links; the player's possessions SHALL take the new ids. An arena save SHALL promote the manager's record, loadouts included, with the id remap table on. A story or arena load SHALL import the save's world tier into the live database; a new story SHALL clear it.

#### Scenario: A repaired item through a save
- **WHEN** a kit copy of a marked template in the pack has its dirt and wear lowered, the story saves and the save is loaded
- **THEN** the pack names the copy's world id and the live world record keeps the lowered dirt and wear
