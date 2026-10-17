## ADDED Requirements

### Requirement: Character tick stages
Each living body's character tick SHALL run nine stages in the original's order (upkeep, readiness, anchor, behaviour, posture, attack, movement, sampler, carry) and finish them before the next body's tick starts. Its work SHALL sit in them: what the body does (posture); the gait floor, the controller's step, balance and ground (movement); the clip's capture, clock and sample (sampler); the targets on the frame, the lift and the controller's follow (carry).

#### Scenario: One character after another
- **WHEN** two characters tick
- **THEN** the first runs all nine stages, in order, before the second runs its first

#### Scenario: Who ticks
- **WHEN** a tick runs with the player, a walking character with a controller, a standing body, a limp body and a body whose clips play by frames
- **THEN** the walking character and the standing body tick with their controllers (none for the standing body), the player last, and the limp body and the frame-played body do not tick

### Requirement: Solver stages
The solver SHALL offer stages for added work at the original's points of its sector step: after integration; in each of the 20 iterations (numbered 1 to 20) before every body's links and after them; after the iterations, before the level contacts are dropped; then the sim objects, body–body detection and sim-object contacts, before level detection; at the start of each of the 8 friction passes; after the post-step. A stage with no work SHALL change nothing.

#### Scenario: Stage order
- **WHEN** a recording stage is solved with a body resting on the floor and a body pulled apart at a joint
- **THEN** it is called after integration, then 20 times at the pair pass and after the links (the pulled body moving between them), after the iterations with the level contacts still held, at the sim objects, detection and sim-object contacts with none held, 8 times at friction with this tick's contacts, then after the post-step

#### Scenario: No work, no change
- **WHEN** a body ticks 120 times with a stage that has no work
- **THEN** its particles are exactly where the solver without stages leaves them

## MODIFIED Requirements

### Requirement: World tick order
The system SHALL run each 83 Hz tick of the bodies as one world, in the original's order: a pre-step; every living body's character tick, the player's last, each body's whole tick before the next; the queued blows; the integration of every awake body; 20 iterations over all bodies; the bodies' level detection; 8 friction passes over all bodies; the post-step of every body; then the per-character post-step.

#### Scenario: Character ticks first
- **WHEN** a tick runs with the player and other living bodies
- **THEN** every living body's targets are set before any body is integrated, the player's after every other body's

#### Scenario: One body alone
- **WHEN** a single body ticks in the world
- **THEN** it ends the tick exactly where ticking it alone leaves it

#### Scenario: The tick's sets
- **WHEN** a tick runs
- **THEN** the pre-step runs first, then the character ticks, the solver and the per-character post-step
