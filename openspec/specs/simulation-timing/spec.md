# simulation-timing Specification

## Purpose
The simulation's per-tick environment conversion and its frame interpolation fraction.

## Requirements

### Requirement: Environment tick conversion
The system SHALL convert an environment to one tick by raising each per-second retention that is above zero to `1/ticks` and storing zero for any other retention, a NaN included, scaling the gravity vector by the reciprocal of `ticks` squared as a float multiply, and squaring each speed threshold after dividing it by the tick count.

#### Scenario: The inert environment
- **WHEN** the inert environment is converted at 83 ticks
- **THEN** every retention and the gravity are zero

#### Scenario: A NaN retention
- **WHEN** an environment whose fast retention is NaN, slow retention −1 and resting retention 0.05 is converted at 83 ticks
- **THEN** the retentions are 0, 0 and 0.96455044

### Requirement: Frame fraction
The system SHALL compute the frame interpolation fraction as `1 + (since - time)/step` when the time since the base exceeds the simulation's game time and `1 - (time - since)/step` otherwise, in double and rounded to a float, clamped to [-2, 2].

#### Scenario: A late frame
- **WHEN** the time since the base is 100 ms, the game time 0 and the step 12 ms
- **THEN** the fraction is 2.0

### Requirement: Scene sector creation
The simulation SHALL create one sector per sector node of a scene batch; a new sector that holds no static nodes yet SHALL hand its node's children back for immediate registration, and when the scene shares the first sector every node of the batch SHALL append its children to that sector's pending list without changing its scene word or node.

#### Scenario: A shared batch
- **WHEN** a shared batch of two sector nodes with children 30 and 31, 32 is added after a first sector exists
- **THEN** no new sector is created, the first sector keeps its scene and node, and its pending list is 30, 31, 32

### Requirement: Step driver due test
The simulation's step driver SHALL count time in wrapping 32-bit milliseconds from a base and SHALL find the next step due once the time since the base, compared unsigned, reaches the simulation's time plus one step. Each step taken SHALL add one to the tick count and one step to the simulation's time.

#### Scenario: Back to back
- **WHEN** a 12 ms driver started at 0 runs every step due at 59 ms
- **THEN** four steps run and the simulation's time is 48 ms

### Requirement: Step driver backlog
When a due step is more than four steps late and the simulation does not keep its backlog, the step driver SHALL move the base on by the lateness so that the step ends now.

#### Scenario: Four steps late
- **WHEN** a 12 ms driver started at 0 polls at 60 ms
- **THEN** a step is due and the base stays at 0

#### Scenario: More than four steps late
- **WHEN** a 12 ms driver started at 0 polls at 61 ms
- **THEN** a step is due, the base moves to 49 ms, and after that step the driver waits 11 ms

#### Scenario: A pause dropped
- **WHEN** a 12 ms driver that has taken one step is polled again a second later
- **THEN** 988 ms are dropped and one step is due

### Requirement: Step driver wait
When no step is due the step driver SHALL wait one millisecond less than the time left to the next step, at most 100 ms.

#### Scenario: Just started
- **WHEN** a 12 ms driver started at 1000 ms polls at 1000 ms
- **THEN** it waits 11 ms

### Requirement: Fixed step boundary map
The game SHALL, when given `--sim-boundary <file>`, write, once the loading screen is gone and the fixed step has run 200 more ticks, every component and resource type that a system of the fixed step's schedules and a system of the frame's schedules both access, with its direction (written by the fixed step only, by the frame only, by both, or read by both) and each side's reading and writing systems, and SHALL list each side's systems that take the whole world.

#### Scenario: A body read by the frame
- **WHEN** a fixed-step system writes a component that an `Update` system only reads
- **THEN** the map lists the component as going from the simulation to the frame

### Requirement: Simulation world in lockstep
The game SHALL keep the simulation's own world, run on a simulation thread or, when asked, on the frame's thread, and SHALL run exactly one step of it at the start of every pass of the fixed step, waiting for it, so that the fixed step's results are the same bit for bit with or without it.

#### Scenario: Three passes
- **WHEN** the fixed step runs three passes
- **THEN** the simulation's world has stepped three times and its last snapshot is tick 3

#### Scenario: A body unchanged
- **WHEN** a shoved, muscle-driven body runs 480 ticks with no simulation world, with one inline, and with one on its thread
- **THEN** every part's pose after every frame is the same bit for bit in the three runs

### Requirement: Commands to the simulation
Commands the frame posts for the simulation SHALL run on the simulation's world at the start of the next step, before its schedule, in the order they were posted, each once.

#### Scenario: Three commands
- **WHEN** commands A, B and C are posted before a step
- **THEN** they run in that order before that step's schedule, and not again

### Requirement: Events to the frame
Events the simulation sends during a step SHALL run on the frame's world after that step, in the order they were sent, each once.

#### Scenario: Two steps
- **WHEN** a step sends events A and B
- **THEN** the frame runs A then B after that step, and neither after the next

### Requirement: Simulation snapshots
After each step the simulation SHALL publish a snapshot of its tick and game time into a slot that keeps only the newest one, and the frame SHALL keep the last two it took.

#### Scenario: Two published before a take
- **WHEN** snapshots 1 and 2 are published before the frame takes one
- **THEN** the frame takes 2, and nothing more until the next is published

### Requirement: Objects named in both worlds
An object the frame hands to the simulation SHALL be named by an id issued in order and linked to its entity in each world; removing it SHALL unlink it in both.

#### Scenario: Spawned and removed
- **WHEN** the frame hands over an object with a mass of 70 and then removes it
- **THEN** after the first step the simulation's entity for that id has a mass of 70, and after the second neither world links the id

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
