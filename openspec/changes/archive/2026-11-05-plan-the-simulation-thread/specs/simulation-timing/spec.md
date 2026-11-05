## ADDED Requirements

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
