## ADDED Requirements

### Requirement: Fixed step behind the bridge
When the bridge is on, the game SHALL run the fixed step's schedules, with their order and sets, in the simulation's world in lockstep, and SHALL copy the data they touch between the worlds before and after each step, so that the fixed step's results are the same as with the schedules in the frame's world.

#### Scenario: A body unchanged
- **WHEN** a shoved, muscle-driven body runs 480 ticks with the fixed step in the frame's world, and with it behind the bridge inline and on the simulation's thread
- **THEN** every part's pose after every frame is the same bit for bit in the three runs

#### Scenario: A character dies in its posture stage
- **WHEN** a collapsing body whose tone falls to a quarter is ticked three times through the fixed step, in one world and behind the bridge
- **THEN** in every run it is dead and limp after its posture stage and only the other body runs the later stages

### Requirement: Change detection across the bridge
A change, addition, removal, spawn, despawn or message made on one side SHALL be seen once by the other side's systems at the same point of the step order as in one world, and SHALL NOT be sent back.

#### Scenario: A scripted run
- **WHEN** the frame moves, adds, removes and despawns components and sends messages between steps while the fixed step moves bodies, spawns and despawns entities and answers messages
- **THEN** the fixed step's and the frame's logs of what changed are the same as in one world, and a value nobody changed is not copied

### Requirement: Shared entity ids
An entity SHALL have the same id in both worlds, whichever world spawned it.

#### Scenario: Spawned by the simulation
- **WHEN** the fixed step spawns entities behind the bridge
- **THEN** the frame's world has them at the same ids

### Requirement: Bridge coverage
When the bridge starts it SHALL list every data type the moved systems touch that is not bridged, and the boundary map SHALL list the same types.

#### Scenario: Fully registered groups
- **WHEN** the bodies' or the character tick's tests start the bridge
- **THEN** the list is empty
