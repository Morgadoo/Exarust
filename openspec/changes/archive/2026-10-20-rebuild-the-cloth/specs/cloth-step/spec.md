## ADDED Requirements

### Requirement: Cloth rebuild
A non-root cloth SHALL return without work. A root SHALL copy its origin pointer. A present override that differs SHALL stop the rebuild. Otherwise the rebuild SHALL transform each particle point into a fresh buffer and submit it.

#### Scenario: The rebuild
- **WHEN** the cloth is its root, has no override and has 3 particles
- **THEN** the origin is copied, 3 points are transformed and the buffer is submitted
- **WHEN** the cloth is not its root
- **THEN** nothing is copied or submitted
- **WHEN** the override differs from the origin
- **THEN** nothing is transformed or submitted
