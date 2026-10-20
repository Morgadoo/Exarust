## ADDED Requirements

### Requirement: Budgeted path shortcut
The budgeted shortcut SHALL pass the clear predicate a budget of the later cost minus the kept cost plus ten, and otherwise SHALL shorten like the unbudgeted shortcut, keeping the first and last points.

#### Scenario: Short and long jumps
- **WHEN** the costs are 0, 100, 101 and 102 and only a budget of at most 20 is clear
- **THEN** the jump from 0 to 2 is blocked and the jump from 1 to 3 is clear, leaving points 0, 1 and 3
- **WHEN** every budget is clear
- **THEN** only the first and last points remain
