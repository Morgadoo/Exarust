## ADDED Requirements

### Requirement: Room walk arithmetic
The active-room walk SHALL take the anchor's cell as `row × width + column`, accepted from 0 to below the cell count, and SHALL want a door's room while the squared distance from the door point to the anchor is below the squared 2 m reach.

#### Scenario: Door reach squared
- **WHEN** the anchor is 1.99 m from a door point of its cell, then 2.01 m
- **THEN** the room behind the door is wanted, then not

### Requirement: Fade overlay alpha
A room fading in SHALL show its lid at `1 − t`, with `t` the elapsed time over 1000 ms in double precision, rounded once to single; when that single, widened again, is below 0.01 the room SHALL turn active and show no lid. A room fading out SHALL show its lid at `t`. The alpha drawn SHALL be clamped to 0…1.

#### Scenario: The end of a fade-in
- **WHEN** a room started fading in 989 ms ago, then 990 ms ago
- **THEN** its lid is drawn, then the room is active with no lid
