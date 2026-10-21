## ADDED Requirements

### Requirement: Environment tick conversion
The system SHALL convert an environment to one tick by raising each positive per-second retention to `1/ticks` (zero otherwise), scaling the gravity vector by the reciprocal of `ticks` squared as a float multiply, and squaring each speed threshold after dividing it by the tick count.

#### Scenario: The inert environment
- **WHEN** the inert environment is converted at 83 ticks
- **THEN** every retention and the gravity are zero

### Requirement: Frame fraction
The system SHALL compute the frame interpolation fraction as `1 + (since - time)/step` when the time since the base exceeds the simulation's game time and `1 - (time - since)/step` otherwise, in double and rounded to a float, clamped to [-2, 2].

#### Scenario: A late frame
- **WHEN** the time since the base is 100 ms, the game time 0 and the step 12 ms
- **THEN** the fraction is 2.0
