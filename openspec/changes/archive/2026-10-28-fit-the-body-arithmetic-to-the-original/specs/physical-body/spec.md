## ADDED Requirements

### Requirement: Edge restore arithmetic
The system SHALL restore a part's six edges in order, taking each length as the square root of (x² + z²) + y², and SHALL move the first end by (len − rest)/(len + len) times the gap and the second end back by the same.

#### Scenario: A stretched edge
- **WHEN** one edge of a part is stretched to twice its rest length
- **THEN** one restore brings it within a fifth of its rest length and lowers the six edges' total error

### Requirement: Step position sums
The system SHALL move each particle in a step by adding the tick's kept motion and then, as a separate sum, its gravity displacement.

#### Scenario: Two rounded sums
- **WHEN** a body steps under gravity
- **THEN** each particle's position is (position + kept motion) + gravity displacement

### Requirement: Sleep empties the motion buffers
A body that falls asleep SHALL empty each part's last-motion and smoothed-velocity buffers.

#### Scenario: Falling asleep
- **WHEN** a moving body's countdown runs out in a tick
- **THEN** after that tick every part's motion and smoothed velocity are zero
