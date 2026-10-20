## ADDED Requirements

### Requirement: Match board show
Showing SHALL clear the stored pointer and byte, store the seed and each of the three columns' child count less one. Each of the five panels with a positive count SHALL be cleared, and a present pointer SHALL be cleared. The base result SHALL be returned and a present global SHALL get the minus-one call.

#### Scenario: The opening
- **WHEN** the seed is 7, the columns are 3, 1 and 0, the panels are 1, 0, 2, 0 and 5, the pointer is present, the base is true and the global is absent
- **THEN** the stored columns are 2, 0 and −1
- **AND** the cleared panels are the first, third and fifth
- **AND** the pointer is cleared, the result is true and the global is not called

### Requirement: Match board destroy
Destroying SHALL run the base destructor when the flags are positive, clear the stored pointer, release the list at `0x1a0`, run the base, and free when the object and flags are both present.

#### Scenario: The destroy
- **WHEN** the flags are 1
- **THEN** the base destructor, the list release, the base and the free all run
- **WHEN** the flags are 0
- **THEN** the base destructor and the free do not run
