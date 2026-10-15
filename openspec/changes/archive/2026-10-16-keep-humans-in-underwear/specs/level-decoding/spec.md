## MODIFIED Requirements

### Requirement: Placed characters
The system SHALL decode the characters a level places, with their records and worn items from the level's own object database or the game's, and their races from `races.rdb`. It SHALL spawn each in its race's body playing its race's moveset where the level puts them. Characters of the human frame SHALL wear exactly the items their records list, including those without a ground model, each as the model its body names and painted with its appearance.

#### Scenario: First story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the level's human inhabitants stand where the level places them, dressed in their own clothes, and its dead lie down

#### Scenario: Braies and quilted trousers
- **WHEN** `exarust play exanima01` runs
- **THEN** Sticky and the two dead men wearing braies show pale grey braies, Patient and Jimmy wear their quilted trousers, and no placed human is given an item its record does not list

#### Scenario: Golem hall
- **WHEN** `exarust play exanima06` runs
- **THEN** its golem guards stand in their golem bodies, and no placed character lacks a body

#### Scenario: Corpus
- **WHEN** every level in `Resource.rpk` is decoded
- **THEN** each of the 103 placed humans wears an item that covers the groin, and all 481 items they wear name a model in `Characters.rpk`
