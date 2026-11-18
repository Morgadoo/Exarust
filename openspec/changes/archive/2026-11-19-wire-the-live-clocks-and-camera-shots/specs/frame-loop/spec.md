## ADDED Requirements

### Requirement: Live game clock
The frame SHALL advance the game clock once a frame from the real clock, adding the whole frame to the paused accumulator while the game is paused.

#### Scenario: A paused frame
- **WHEN** the simulation's pause is up for a frame
- **THEN** the game clock does not advance

### Requirement: Live scene clock
The scene clock SHALL start when a level is in and step once a frame on the game clock, capping a step over 1/60 s with a scale of the cap over the step.

#### Scenario: A long frame
- **WHEN** a frame takes 50 ms
- **THEN** the scene's step is the capped 16.67 ms and its scale 16.67 / 50

### Requirement: Flicker on the scene step
The lights' flicker SHALL take the scene clock's step.

#### Scenario: A held game
- **WHEN** the game is paused
- **THEN** the flicker steps by 0 ms
