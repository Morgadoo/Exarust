## ADDED Requirements

### Requirement: Root displacement from the two-step point
The character tick's root displacement, which the locomotion routine and the lift servo read, SHALL be the two-step point of the root part's own origin, and the muscles-off snap SHALL write the current, previous and third particle sets.

#### Scenario: A velocity set is not read
- **WHEN** the root's previous particles are set to its current ones and its third set is a tick behind
- **THEN** the root displacement is the move since the third set, and after a snap it is zero
