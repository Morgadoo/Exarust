## ADDED Requirements

### Requirement: Story writer order
After the placement, the story writer SHALL emit the discovery chunk, the journal when it holds an entry, the global experience, the memory flags when non-zero and the transition state when its first word is non-zero. Chunks the writer does not know SHALL follow, in the order read.

#### Scenario: Story order
- **WHEN** transition, unknown, experience, journal, memory and discovery chunks are set in that order
- **THEN** they are written as discovery, journal, experience, memory, transition, unknown

### Requirement: Arena writer order
After the placement, the arena writer SHALL emit the journal when it holds an entry, the arsenal, the stash when non-empty, the shop stock when non-empty, the company record, the extra record, the employees, the merchant, the hireables and the tournament. Chunks the writer does not know SHALL follow, in the order read.

#### Scenario: Arena order
- **WHEN** the tournament, hireables, company, shop stock, arsenal, journal, employees, extra, merchant and stash chunks are set in that order
- **THEN** they are written as journal, arsenal, stash, shop stock, company, extra, employees, merchant, hireables, tournament

### Requirement: Save section order
A save SHALL put the world characters when their count is above zero, the player data, the world objects when their count is above zero, and then the level under its locale name. A player whose health, widened to double, is below 0.01 SHALL have the save deleted and nothing written.

#### Scenario: Sections
- **WHEN** a story save is made with world characters, world objects and a level
- **THEN** the directory lists them in that order after the sentinel

### Requirement: Save inspection
A save package SHALL be readable without being written. Reading and rewriting an untouched package SHALL give the original bytes, a deleted section's payload and the bytes between sections included.

#### Scenario: Rebuild
- **WHEN** a package is read and written back with no change
- **THEN** the bytes are equal
- **WHEN** its player data is parsed and written back
- **THEN** the section is equal
