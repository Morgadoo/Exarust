## ADDED Requirements

### Requirement: The fierce rule starts the phase word
Applying the roles SHALL start the phase word at −1 when the body carries the fierce rule bit, else at 0; a follower's phase word SHALL be 1.

#### Scenario: A fierce body
- **WHEN** the roles of a fierce body that does not follow are applied
- **THEN** its phase word is −1

### Requirement: Party join and leave
Applying the roles SHALL tell the party that a follower joins it, and that a body which followed and no longer does leaves it. The death hook of a follower SHALL tell the party it leaves.

#### Scenario: A marker resisted, then death
- **WHEN** a Loyalty marker makes a brain follow, it then resists the marker, follows again and dies
- **THEN** the party hears a join, a leave, a join and a leave

### Requirement: An arrival restarts the idle activity
A follower arriving through a door SHALL restart its idle activity: its search SHALL stand again at the world origin with nothing picked.

#### Scenario: A searching follower arrives
- **WHEN** a follower whose search had picked an object arrives
- **THEN** its search stands at the world origin with no candidates
