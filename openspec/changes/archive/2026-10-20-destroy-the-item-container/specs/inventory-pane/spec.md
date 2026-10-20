## ADDED Requirements

### Requirement: Container destroy
The destroy SHALL make one release and one final call on a set free flag, call one per-entry release for each counted entry through 0x107a90 when detached and 0x1079e0 otherwise, destroy the array once and run the base once.

#### Scenario: The freeing path
- **WHEN** the free flag is set, the container is detached and 3 entries are counted
- **THEN** one release, three entry calls, one array teardown, one base and one final call run
- **WHEN** the free flag is clear and no entry is counted
- **THEN** only the array teardown and the base run
