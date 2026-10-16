## MODIFIED Requirements

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab, and SHALL turn it on at the start of a practice or arena fight. In combat mode the heading SHALL turn to the cursor's direction once it is more than 30° off it, and follow it while the left button is held; the body SHALL turn toward the heading at the body-turn rate. W, S, A and D SHALL step along and across the cursor's direction; their walk SHALL give back the clip it replaced, and a walk from before combat mode SHALL end in the stance.

#### Scenario: Side step
- **WHEN** A is held in combat mode
- **THEN** the player steps to its left while still facing the cursor

#### Scenario: Small cursor move
- **WHEN** the cursor moves 20° to the side of the facing in combat mode
- **THEN** the player keeps its facing, and turns toward the cursor once it is more than 30° off

#### Scenario: Fight start
- **WHEN** `exarust --opponent play arenao1` shows the arena
- **THEN** the player is in combat mode without Tab, and Tab turns it off

#### Scenario: Stepping from the guard
- **WHEN** A is held and released while the player stands in the Defence stance
- **THEN** it plays the combat walk while A is held and the Defence stance again after

#### Scenario: Tab while walking
- **WHEN** Tab is pressed while the player walks out of combat mode, and the keys are released
- **THEN** it stands in its stance instead of walking on the spot
