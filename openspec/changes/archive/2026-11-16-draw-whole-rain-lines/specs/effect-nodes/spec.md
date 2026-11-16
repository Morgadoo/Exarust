## ADDED Requirements

### Requirement: Whole rain lines
The rain's line draw SHALL draw only whole lines: an odd drawn count SHALL leave its last head out, and every supersample copy of the streaks SHALL pair each drop's head with its own tail.

#### Scenario: An odd count drawn twice
- **WHEN** 3527 vertices are drawn on a supersampled target with two copies
- **THEN** each copy draws 1763 lines, each from a drop's head to its own tail
