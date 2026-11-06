## ADDED Requirements

### Requirement: Equal waits go to the earlier spawned
When two brains have waited equally long for the target pass's or the route planner's turn, the turn SHALL go to the earlier spawned (the lower entity index), whatever order the brains claim in.

#### Scenario: Reversed claims
- **WHEN** three brains claim in reverse spawn order
- **THEN** they take the turns in spawn order

### Requirement: The AI's death hook
When a brain's body dies, a follower SHALL stop following (activity 0) and leave the party; another activity SHALL stay.

#### Scenario: A dying follower
- **WHEN** a following brain's condition becomes dead
- **THEN** its activity is 0 and it leaves the party
