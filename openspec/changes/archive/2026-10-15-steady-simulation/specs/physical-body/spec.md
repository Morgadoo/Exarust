## ADDED Requirements

### Requirement: Fixed simulation rate
The system SHALL step the simulation at the original's 83 Hz. A slow frame SHALL catch up at most 4 steps; the rest of the backlog SHALL be dropped, as the original drops it.

#### Scenario: Slow frame
- **WHEN** a frame takes 200 ms
- **THEN** at most 4 fixed steps run, and game time advances 48 ms

### Requirement: Smooth character motion
The system SHALL draw characters between their last two fixed steps, by the fraction of the next step already elapsed.

#### Scenario: Walking at 37 FPS
- **WHEN** the player walks while the frame rate is not a divisor of 83
- **THEN** the drawn position advances evenly from frame to frame

### Requirement: Sleeping limp bodies
The system SHALL stop simulating a limp body whose particles all stayed slower than 1 cm/s for 2 s, until it is shoved.

#### Scenario: Corpse at rest
- **WHEN** a corpse has lain still for 2 s
- **THEN** it is not simulated until a blow lands on it
