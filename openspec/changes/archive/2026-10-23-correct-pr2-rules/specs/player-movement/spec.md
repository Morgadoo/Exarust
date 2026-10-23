## MODIFIED Requirements

### Requirement: Clips follow movement
The system SHALL play the walk clip whenever the player moves forward, walking or running: the body tick's locomotion SHALL start it in the tick it first moves the player, from any clip, and the steering SHALL end it in the idle clip when the player stops out of combat mode. A walk or a run SHALL blend into idle over 20 frames with the fast ease; a backpedal SHALL end at once. Standing still out of combat mode, the steering SHALL start no other clip: the stance decides.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk in the tick the body first moves, and back to idle

#### Scenario: A walk ends over twenty frames
- **WHEN** the player stops a walk or a run out of combat mode, and later a backpedal
- **THEN** idle starts with a fast-eased blend of 20 frames, and the backpedal ends at once

#### Scenario: Combat idle after combat mode
- **WHEN** Tab turns combat mode off while the player stands in Cmbt - Idle with the Defence stance, and it then turns in place, walks and stops
- **THEN** it keeps Cmbt - Idle while standing and turning, plays the walk from the tick W is decided, and stops in idle

#### Scenario: Running plays the walk
- **WHEN** the player runs forward
- **THEN** the clip is the walk

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab, and turn it on at a practice or arena fight's start. In combat mode the heading SHALL turn to the cursor once more than 30° off it, and follow it while the left button is held; the body SHALL turn toward it at the table's rate (300°/s standing). W, S, A and D SHALL step along and across the cursor's direction; their walk SHALL give back the clip it replaced; a walk or run from before combat mode SHALL stop softly and end in the stance as a walk ends.

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

### Requirement: Footwork step targets
In combat mode a footwork step SHALL aim from the chest: target = chest + v·(r·√scale/|v|) cm, v the key sum with a side key weighing 0.8, r the step reach × the legs' extension (80 cm for a human), or with the long-step latch (1.5 − 0.3·encumbrance) × the reach, narrowed once, times the extension. The line walk SHALL clamp it from the chest's x and z at the ground, and the step SHALL be taken only when the clamped target is over 400 cm² from the chest in three dimensions.

#### Scenario: Footwork at a wall
- **WHEN** W is held in combat mode facing a wall that clamps the step's target 20 cm ahead of the chest on its level, then 21 cm ahead
- **THEN** the player takes no step and plays no walk, then it steps and plays the combat walk

#### Scenario: A diagonal weighs the side by 0.8
- **WHEN** W and A are held together facing +Z, on open ground, for a fresh press of an 80 cm step
- **THEN** the target lies 0.8 as far to the side as it lies forward, and a side key alone still aims a full step

#### Scenario: A raised clamp
- **WHEN** the line walk clamps a step 15 cm ahead and 15 cm up, then 15 cm ahead on the level
- **THEN** the first is stepped to and the second is not

#### Scenario: A held step is a long step
- **WHEN** a combat step's keys have already been held past 200 ms, on open ground, for a human step reach and for a reach of 72 cm at encumbrance 0.5
- **THEN** the target is 120 cm and 97.2 cm along the keys, and a fresh press of the same reach stays at 80 cm

#### Scenario: A shortened extension halves the step
- **WHEN** the legs' extension is 0.5, on open ground, for a human's fresh press and for the same keys held past 200 ms
- **THEN** the targets are 40 cm and 60 cm along the keys

#### Scenario: A crouch of 0.1 keeps the short step
- **WHEN** a combat step's keys have already been held past 200 ms and the crouch is 0.1, then the float just under 0.1
- **THEN** the first target stays one step reach and the second is the long step

## ADDED Requirements

### Requirement: Long step latch
The footwork's keys SHALL form a key code in which opposing keys cancel; the cursor walk is its own code. The clock SHALL be stamped when the code changes, and the long-step latch SHALL be set when the code is held, the stamp plus 200 ms is below the clock and the crouch widened to double is below 0.1. Only a code change or a release SHALL clear it; a later crouch or a blocked step SHALL not. The interim footwork SHALL walk only while a step is taken.

#### Scenario: The latch holds until the keys change
- **WHEN** W is pressed and held with steps of 80 and 120 cm, then the crouch goes to 0.5, then A and D are added, then W and A are held, then every key is released
- **THEN** the target is 80 cm through 16 ticks after the press and 120 cm from the 17th, stays 120 cm after the crouch and with W, A and D, the hold restarts at 0 with W and A, and the release clears the latch
