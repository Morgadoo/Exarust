## MODIFIED Requirements

### Requirement: Head look direction
The head tracking SHALL take the look-at minus the eye, reuse the previous unit direction when the squared xz distance is under 2,500, and in the band under 10,000 replace the vertical component with (10000 − d²)·1e-4 before normalizing with the engine's fast normalise (the approximate reciprocal square root of (x² + z²) + y², zero staying zero). The written look direction SHALL be the engine's pitch-and-yaw direction of the tracked angles, by its fast sine and cosine.

#### Scenario: Distant and close targets
- **WHEN** the target is far away and off to the right
- **THEN** the direction comes from the normalized difference
- **WHEN** the target is under 50 cm away
- **THEN** the previous direction is used unchanged

#### Scenario: The engine's direction
- **WHEN** the far head tracking runs from yaw 0.3 and pitch −0.2
- **THEN** its look is the engine's pitch-and-yaw direction of the stored angles, bit for bit
