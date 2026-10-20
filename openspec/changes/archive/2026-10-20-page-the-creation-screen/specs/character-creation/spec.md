## ADDED Requirements

### Requirement: Creation page
Paging SHALL do nothing unless the maximum exceeds the index. Otherwise the mode SHALL become 1, the index SHALL rise by one and the refresh SHALL run.

#### Scenario: Page and stop
- **WHEN** the maximum is 5 and the index 2
- **THEN** the mode becomes 1, the index 3 and the refresh runs
- **WHEN** the maximum is 2 and the index 2
- **THEN** nothing changes

### Requirement: Creation select
Selecting SHALL use the wanted index less one. An unchanged index SHALL do nothing; otherwise the mode SHALL become 1 and the refresh SHALL run at that index.

#### Scenario: Select and stay
- **WHEN** the wanted index is 4 and the current 2
- **THEN** the mode becomes 1, the index 3 and the refresh runs
- **WHEN** the wanted index is 3 and the current 2
- **THEN** nothing changes
