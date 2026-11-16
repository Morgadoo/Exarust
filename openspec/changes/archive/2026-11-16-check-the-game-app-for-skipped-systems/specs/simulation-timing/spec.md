## ADDED Requirements

### Requirement: The game's App is checked for skipped systems
The workspace tests SHALL build the game's App as the binary builds it, headless with the simulation bridge in lockstep, run its scenes a few hundred ticks past their loads, and fail on any system of the frame's world or the simulation's that was not run for a missing resource or message queue, naming the system and the parameter.

#### Scenario: A message the bridge does not carry
- **WHEN** a fixed-step system reads a message that is registered only in the frame's world
- **THEN** the check fails and names the system, the simulation's world and the message's queue

### Requirement: The data-free check runs everywhere
Without an installation the check SHALL run the arena's command line with no game data until the bridge has started and 300 ticks have run, allowing only the systems that read the installation to be skipped; with `EXANIMA_DIR` it SHALL also run the arena with a duellist and a Shock cast and the first level, allowing no skip.

#### Scenario: No installation
- **WHEN** the workspace tests run without `EXANIMA_DIR`
- **THEN** the data-free arena runs and passes, and the level scenes are skipped with a note
