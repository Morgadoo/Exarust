# Spec Delta

## ADDED Requirements

### Requirement: Placed objects
The system SHALL decode the object-database objects a level places and spawn each with its item's model at its transform.

#### Scenario: First story level
- **WHEN** `exarust level exanima01` runs
- **THEN** the level's placed objects appear, and none of their models are missing
