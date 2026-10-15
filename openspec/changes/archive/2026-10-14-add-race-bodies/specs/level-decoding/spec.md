## MODIFIED Requirements

### Requirement: Placed characters
The system SHALL decode the characters a level places, with their records and worn items from the level's own object database and their races from `races.rdb`. It SHALL spawn each in its race's body playing its race's moveset where the level puts them. Characters of the human frame SHALL wear their items painted with their appearances.

#### Scenario: First story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the level's human inhabitants stand where the level places them, dressed in their own clothes, and its dead lie down

#### Scenario: Golem hall
- **WHEN** `exarust play exanima06` runs
- **THEN** its golem guards stand in their golem bodies, and no placed character lacks a body
