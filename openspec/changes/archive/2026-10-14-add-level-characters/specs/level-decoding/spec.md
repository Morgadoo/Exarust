# Spec Delta

## ADDED Requirements

### Requirement: Placed characters
The system SHALL decode the characters a level places, with their records and worn items from the level's own object database, and SHALL spawn placed humans where the level puts them, wearing their items painted with their appearances.

#### Scenario: First story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the level's human inhabitants stand where the level places them, dressed in their own clothes, and its dead lie down
