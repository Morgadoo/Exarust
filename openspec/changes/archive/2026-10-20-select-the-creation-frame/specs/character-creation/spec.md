## ADDED Requirements

### Requirement: Creation frame selection
Selecting SHALL do nothing when the requested frame is below 1 or already equals the current frame plus one. Otherwise the current frame SHALL become the requested frame less one, the part search result SHALL be stored and the refresh SHALL run.

#### Scenario: Change, no change and no request
- **WHEN** the requested frame is 3, the current frame 5 and the part found
- **THEN** the current frame becomes 2, the part is stored and the refresh runs
- **WHEN** the current frame is already 2
- **THEN** nothing changes and no refresh runs
- **WHEN** the requested frame is 0
- **THEN** nothing changes
