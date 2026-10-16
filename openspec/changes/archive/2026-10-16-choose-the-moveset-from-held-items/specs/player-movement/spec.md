## MODIFIED Requirements

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab. In combat mode the character's heading SHALL turn to the cursor's direction once it is more than 30° off it, and follow it while the left button is held; the body SHALL turn toward the heading at the body-turn rate. W, S, A and D SHALL step along and across the cursor's direction.

#### Scenario: Side step
- **WHEN** A is held in combat mode
- **THEN** the player steps to its left while still facing the cursor

#### Scenario: Small cursor move
- **WHEN** the cursor moves 20° to the side of the facing in combat mode
- **THEN** the player keeps its facing, and turns toward the cursor once it is more than 30° off
