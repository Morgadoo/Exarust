## ADDED Requirements

### Requirement: Creation point copy
The copy SHALL take the child's three words into the object's point.

#### Scenario: The copy
- **WHEN** the child's point is 1, 2 and 3
- **THEN** the object's point is 1, 2 and 3

### Requirement: Creation resume
Resuming SHALL save the global list, refresh at the current index and call the resume slot.

#### Scenario: The resume
- **WHEN** the screen resumes
- **THEN** the save, the refresh at the current index and the resume call all run
