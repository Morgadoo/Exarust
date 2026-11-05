## MODIFIED Requirements

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab, and turn it on at a practice or arena fight's start. In combat mode the heading SHALL turn to the cursor once more than 30° off it, and follow it while the left button is held; the body SHALL turn toward it at the table's rate (300°/s standing). W, S, A and D SHALL step along and across the cursor's direction on the body's own feet, in the stance clip; a walk or run from before combat mode SHALL stop softly and end in the stance as a walk ends.

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
- **THEN** its feet step to its left while the Defence stance plays throughout

#### Scenario: Tab while walking
- **WHEN** Tab is pressed while the player walks out of combat mode, and the keys are released
- **THEN** it stands in its stance instead of walking on the spot

### Requirement: Footwork step targets
A footwork goal SHALL be chest + v·(r·extension·√scale/|v|) cm: v the key sum (W, A, D, S; a side key 0.8, height dropped), r the step reach or, tapped, (1.5 − 0.3·encumbrance)·reach in double, narrowed once, the step then long. The line walk SHALL clamp it from the chest's x and z at the ground, giving the body's step target; it SHALL be taken only when over 400 cm² from the chest in three dimensions, then moved onto the grid's nearest walkable point across the ground.

#### Scenario: Footwork at a wall
- **WHEN** W is held in combat mode facing a wall that clamps the step's target 20 cm ahead of the chest on its level, then 21 cm ahead
- **THEN** the player starts no step, then it starts one, its stance clip playing throughout

#### Scenario: A diagonal weighs the side by 0.8
- **WHEN** W and A are held together facing +Z, on open ground, for a fresh press of an 80 cm step
- **THEN** the target lies 0.8 as far to the side as it lies forward, and a side key alone still aims a full step

#### Scenario: A raised clamp
- **WHEN** the line walk clamps a step 15 cm ahead and 15 cm up, then 15 cm ahead on the level
- **THEN** the first starts a step and the second does not

#### Scenario: A held step is a long step
- **WHEN** W has been held past 200 ms with a step younger than 300 ms, for a human step reach and for a reach of 72 cm at encumbrance 0.5
- **THEN** the target is re-laid 120 cm and 97.2 cm along the keys

#### Scenario: A shortened extension halves the step
- **WHEN** the legs' extension is 0.5, for a plain and a tapped stride of an 80 cm reach
- **THEN** the strides are 40 cm and 60 cm

#### Scenario: A crouch of 0.1 keeps the short step
- **WHEN** W has been held past 200 ms with the crouch at 0.1, then at the float just under 0.1
- **THEN** the first target stays one step reach and the second is the long step

#### Scenario: The grid snap
- **WHEN** a taken goal lies 10 cm off the grid's nearest walkable point
- **THEN** the step's target is that point's x and z at the clamped height

### Requirement: Footwork step under way
A footwork step SHALL be under way while the body's step episode runs and either the episode is not a balance step (one the balance routine started from rest) or its target is strictly more than 20·scale cm from the feet centre across the ground; a NaN distance SHALL not be under way. With no step under way the footwork SHALL lay the movement goal from the held word and, when the goal is taken, start the body's step: stepping, no balance step, no landing yet in the episode, its clock now.

#### Scenario: Started from rest
- **WHEN** W is pressed in combat mode on open ground with an 80 cm reach
- **THEN** the body starts a step toward a target 80 cm ahead

#### Scenario: A settled balance step
- **WHEN** W is pressed while the body's balance step has its target 10 cm from the feet, then while it has its target 30 cm from them
- **THEN** the first starts a footwork step and the second does not

### Requirement: Queued step taken on landing
A landing with a queued step waiting SHALL take it by the body's queue rule: the target becomes the chest plus the queued vector, the step starts anew (no balance step, plain kind), the queue count −2 before the episode's first landing, −1 after. On the next behaviour tick the taken target SHALL be clamped by the line walk from the chest and moved onto the grid's nearest walkable point, and the movement word SHALL take the queued word back, clear its tap flag and restart its change clocks.

#### Scenario: A landing takes the queue
- **WHEN** a queued forward step waits and the body lands a foot
- **THEN** the body's new target is the chest plus 80 cm ahead, the queue count is −1, and on the next tick the target is clamped and the word is forward with no tap

## REMOVED Requirements

### Requirement: Footwork step landing
**Reason**: The capsule step velocity and its landing were Exarust's interim feet; the body now walks the step on its feet and ends it by its own rules.
**Migration**: See Footwork step under way and the physical body's Footwork step on the body.
