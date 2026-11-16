## ADDED Requirements

### Requirement: The chase leave clears the target
A chase that leaves combat for want of a fresh foe SHALL clear the body's target and the duellist's focus before the idle return.

#### Scenario: A stale chase
- **WHEN** a duellist's chase is neither following nor fresh
- **THEN** it leaves combat with no body target and no focus

### Requirement: The death hook fades a golem's glow
A dying body whose race word carries the AI bit SHALL fade its `go` skin material to 0 before the follower branch.

#### Scenario: A golem and a human die
- **WHEN** a golem's body and a human's turn dead
- **THEN** only the golem's glow goes to 0
