## MODIFIED Requirements

### Requirement: Simulation controls
F8 SHALL pause and resume the fixed step (character controllers, character ticks and the body solver), `.` SHALL run exactly one tick while paused, and `[` and `]` SHALL step its speed through ×0.1, ×0.25, ×0.5, ×1 and ×2; `--sim-speed` SHALL set the starting speed. While paused or slowed, rendering SHALL go on and the cameras SHALL move on real time.

#### Scenario: One step
- **WHEN** the simulation is paused at tick 300 and `.` is pressed three times
- **THEN** ticks 301, 302 and 303 run and the simulation stays paused at 303

#### Scenario: Orbit while paused
- **WHEN** the simulation is paused in play mode and the middle mouse button drags
- **THEN** the camera orbits the frozen bodies

## ADDED Requirements

### Requirement: Simulation keys yield to game keys
A simulation key SHALL do nothing while it is bound to a game action, so the game's own keys (the power tree's `P` among them) never pause, step or change the speed of the simulation.

#### Scenario: The power tree key
- **WHEN** a player who knows powers presses `P` in play
- **THEN** the power tree opens and fades in, and the simulation keeps running
