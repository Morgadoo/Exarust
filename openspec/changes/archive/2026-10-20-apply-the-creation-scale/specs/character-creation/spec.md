## ADDED Requirements

### Requirement: Creation scale apply
Applying SHALL store `source × 0.17 + 0.9` at the child's scale and link. With the special flag, a clear global byte SHALL run the global call, a missing label SHALL be built and the label SHALL be drawn.

#### Scenario: Plain and special
- **WHEN** the source is 0 and the special flag is clear
- **THEN** the scale is 0.9 and only the store runs
- **WHEN** the source is 1, the special flag is set, the global byte is clear and no label exists
- **THEN** the scale is 1.07
- **AND** the global call, the label build and the draw all run
- **WHEN** the label already exists
- **THEN** it is not built and it is still drawn
