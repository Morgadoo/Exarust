## ADDED Requirements

### Requirement: Ground under an unsupported body
Each tick a living body whose feet give it no support SHALL ask the level for the ground under its pelvis: a 20 cm sphere from 20 cm above it, swept 2,000 cm down, the ground 100 m down when it meets nothing. A supported body SHALL not ask. A body getting up within 30 cm of that ground has support.

#### Scenario: Depths
- **WHEN** a body off its feet hangs with its pelvis 1.03 m over floors 15 m, 18.5 m and 19 m below where it stood
- **THEN** it finds the first two and, for the third, ground 100 m down

#### Scenario: Standing
- **WHEN** a body stands on the floor
- **THEN** it has no ground under the pelvis measured
