## ADDED Requirements

### Requirement: Free simulation by default
The game SHALL run the simulation free of the frame unless lockstep is asked for, a fixed-frame scripted run is made, the simulation runs on the frame's thread, or the step stays in the frame's world; in lockstep every body's every tick SHALL stay the same as before.

#### Scenario: A scripted duel
- **WHEN** a fixed-frame scripted duel runs with the default options
- **THEN** it steps in lockstep and every body's trace is the same as the earlier build's
