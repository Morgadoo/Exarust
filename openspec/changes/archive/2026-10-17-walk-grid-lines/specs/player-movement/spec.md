## ADDED Requirements

### Requirement: Look-ahead on the grid
While the player walks in exploration its goal SHALL lie 75 + 6·v² cm ahead along its facing (v in cm a tick; 70 cm behind backwards; 45 + 6·v² for combat mode's walk), clamped by the grid's line walk from where it stands. A blocked look-ahead SHALL end the free walk, so the body walks to the clamped goal and brakes; with the goal within 100 cm it SHALL also clear the run and the sprint latch and keep the brake. A soft stop's goal SHALL be clamped the same way.

#### Scenario: Walking into the arena wall
- **WHEN** W is held from the arena's start toward its −Z wall
- **THEN** the free walk ends with the goal clamped at z = −4.95 m, the brake starts within 50 cm of it, and the player stands braked with W still held

#### Scenario: Jogging into a wall
- **WHEN** W and Shift are held toward the arena's −Z wall
- **THEN** the run bit stays (`0x44`) once the free walk ends, and is cleared with the brake kept (`0x50`) as soon as the clamped goal is under 100 cm away

### Requirement: Spawn snap
On the player's third behaviour tick with a navigation grid, unless it is down, getting up or dead, the player SHALL be moved across the ground onto the grid's nearest walkable point.

#### Scenario: Arena start
- **WHEN** `exarust play arena` starts
- **THEN** the log reports the spawn snap from (0, 0.196, 5) moving the player 0.000 m

### Requirement: Footwork step targets
In combat mode a footwork step's target SHALL be a step's length along its keys (the step reach × √scale, 80 cm for a human), clamped by the grid's line walk from where the player stands; a target within 20 cm SHALL not be stepped to. Until the original's steps replace it, the interim footwork SHALL walk only while its step's target lies farther.

#### Scenario: Footwork at a wall
- **WHEN** W is held in combat mode facing a wall that clamps the step's target 20 cm ahead
- **THEN** the player takes no step and plays no walk

## MODIFIED Requirements

### Requirement: Capsule executes the decision
Until the body walks by its carry, its capsule SHALL execute the movement decision: along the heading at 1.4 m/s walking up to 3.6 m/s at a full run blend (rising 0.05 a tick to the encumbrance's cap, falling 0.015), 1.22 m/s backpedalling, slowing while braking by the original's brake drive and enough to stop on its goal. A wall within the capsule's radius of the goal SHALL stop the capsule short of it, braking.

#### Scenario: Soft stop
- **WHEN** W is released at a walk
- **THEN** the capsule slows from 1.687 cm a tick and stops within 3 cm of the goal 8·v + 20 cm ahead

#### Scenario: Goal beside a wall
- **WHEN** the player walks into the arena's −Z wall, whose face lies 2.5 cm past the clamped goal
- **THEN** the capsule touches the wall braking and stands 26 cm short of the goal
