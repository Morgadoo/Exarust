## ADDED Requirements

### Requirement: Creation part refresh
The part refresh SHALL store the search result at the target only when the target exists.

#### Scenario: With and without a target
- **WHEN** the target exists and the part is found
- **THEN** the result is stored and the part is found
- **WHEN** the target is absent
- **THEN** nothing is stored

### Requirement: Creation destroy
Destroying SHALL run the base destructor and free on positive flags. A child above one SHALL be decremented and one at one or below SHALL be released, and the base SHALL always run.

#### Scenario: Decrement, release and free
- **WHEN** the flags are 1 and a child with count 2 exists
- **THEN** the base destructor runs, the child decrements, the base runs and the object is freed
- **WHEN** the child count is 1
- **THEN** the child is released
- **WHEN** the flags are 0
- **THEN** the base destructor and the free do not run
