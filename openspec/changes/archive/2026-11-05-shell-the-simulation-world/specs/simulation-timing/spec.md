## ADDED Requirements

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
