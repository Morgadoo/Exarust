## ADDED Requirements

### Requirement: Exit trace
With `EXA_EXIT_TRACE` set, the game SHALL log each stage of its exit (the exit message, the last frame, the frame world's resources, the render world, the simulation shell's channel and thread, the simulation world, `main`) with the time since the exit began, and SHALL report a stage that lasts longer than `EXA_EXIT_TRACE_DUMP` seconds (10 by default) with every thread's state and stack.

#### Scenario: A stalled stage
- **WHEN** a traced exit spends more than the limit in one stage
- **THEN** the log names the stage and the process, lists every thread's name, state and wait channel, and holds three stack samples of every thread

### Requirement: Prompt exit
A run that ends (`--frames`, the host closing the input, the menu's exit) SHALL leave the process within a second of its last frame; no teardown stage SHALL wait on the audio thread for longer than one output period.

#### Scenario: Constant Displacement in a four-scene grid
- **WHEN** four arena duels with Constant Displacement active end after 1100 frames each, twelve grids in a row
- **THEN** every scene's process returns from `main` within a second of its "frames played, exiting" line
