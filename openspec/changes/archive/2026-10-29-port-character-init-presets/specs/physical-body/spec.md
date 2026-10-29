## ADDED Requirements

### Requirement: Footstep volume
A walking footstep's gain SHALL be multiplied by the character's footstep volume.

#### Scenario: An ogre's step
- **WHEN** a footstep plays for a character of footstep volume 5
- **THEN** its gain is five times a footstep volume 1 step's at the same speed and draws

### Requirement: Anchoring foot rustle
When a foot anchors and its rustle clock is free, the balance routine's rustle SHALL have the volume min(1, 0.0001·d² + 0.2) times the foley base times 0.02·g + 0.02, doubled while attacking, the pitch 0.3·p + 0.8, and the clock SHALL wait 400 ms plus the engine generator's jitter.

#### Scenario: A foot that anchors where it was
- **WHEN** the gap is 0 and the draws are 0 at a foley base of 1
- **THEN** the volume is 0.004 and the pitch 0.8

### Requirement: Releasing foot steps
When an anchored foot lets go and the step clock is free, the step SHALL have the gain step loudness times footstep volume times 0.005·g + 0.02 and the pitch 0.25·p + 0.2·loudness + 0.5; under water a wet step SHALL follow, scaled by its depth share of at most 1; the clock SHALL wait 250 ms plus the jitter.

#### Scenario: A foot that lets go in deep water
- **WHEN** a foot lets go 89 units under the surface value
- **THEN** the wet step's depth share is 1

### Requirement: Grip relax precision
After a grip change the second-hand, arm and extra links' slack steps SHALL be computed in double precision from the single-precision square root and narrowed once; the main link's step SHALL be 1 cm in single precision.

#### Scenario: Four centimetres of second-hand slack
- **WHEN** the second-hand link holds 4 cm of slack and relaxes once
- **THEN** 3.798 cm remain
