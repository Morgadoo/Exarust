## ADDED Requirements

### Requirement: Cloth sync
The sync SHALL skip its transform and pose when the object's meta flags carry 0x10. Otherwise it SHALL transform once and call the pose with 1.0 twice.

#### Scenario: The sync
- **WHEN** the meta flags lack 0x10
- **THEN** one transform and one pose call run with scale 1.0
- **WHEN** the meta flags carry 0x10
- **THEN** nothing runs
