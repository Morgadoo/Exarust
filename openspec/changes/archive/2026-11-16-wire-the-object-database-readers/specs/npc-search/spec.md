## ADDED Requirements

### Requirement: Gathering by the database's mask
An object SHALL be worth gathering only when the database's mask test over its record's section chain finds an apparel, weapon, operative or container part.

#### Scenario: A chest and a cup
- **WHEN** a level holds a record with only a model and one with a container part
- **THEN** only the second is listed as loot

### Requirement: Container sizes from the flagged lookup
An item's size for a container add SHALL be its model section's byte `0x1C` as the database's flagged section lookup gives it: none when the record's table flags lack the model, or for a world id.

#### Scenario: Flags without the model
- **WHEN** two level records hold the same model section and only one's table flags name it
- **THEN** only that one has a size, and a world id has none
