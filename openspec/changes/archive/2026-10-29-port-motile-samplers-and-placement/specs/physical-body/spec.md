## ADDED Requirements

### Requirement: Sampler heights
After posing the parts the system SHALL take the stand height as the clip root's height less the lowest posed foot contact less the sole (from 5000, a NaN contact never lowest; 0 when that lowest is at or over the root), and the leg reach as the root less half the clip contacts' heights summed; a hovering body takes the root less 10 cm for both.

#### Scenario: Two contacts
- **WHEN** the root is 100 cm, the posed contacts 9 and 7 cm, the clip contacts 8 and 12 cm and the sole 4.5 cm
- **THEN** the stand height is 97.5 cm and the leg reach 90 cm

### Requirement: Placement pelvis target
The placement's sampler SHALL set the pelvis target to (3 − 3·run) + (stand height + the pelvis's posed height) in the walking condition, and otherwise to the margin: the stand height (0 under the Unconscious clip) plus twice the body scale plus 5 cm when not stepping, times 0.7 (0.8 stepping) when crouched past 0.95 in double on the feet; then times the pelvis-height factor.

#### Scenario: Standing placement
- **WHEN** a body not walking, not stepping, of scale 1 is placed with a stand height of 95 cm
- **THEN** its pelvis target is 102 cm

#### Scenario: Float crouch at the threshold
- **WHEN** the crouch is the float 0.95
- **THEN** the target is not shortened

### Requirement: Living pelvis target
The living sampler SHALL ease the pelvis target: walking by (0.1 + 0.1·run) toward stand + 2·run + 3, otherwise by 0.1 toward the margin (set outright when crouched), a get-up under a fifth of its clip starting from 0; a hovering body's target SHALL be the root less 10 cm; then times the pelvis-height factor.

#### Scenario: Early get-up
- **WHEN** a body getting up at a tenth of its clip had a target of 80 cm
- **THEN** its target is a tenth of its margin without the stand height

### Requirement: Placement arithmetic
A placement SHALL lower every part's posed placement by the pelvis's posed height and put it on the frame, place each foot anchor at its clip contact on the frame less the clip root (unrotated), at the snapped ground plus the sole on the ground or the mapped height plus the sole less 8 cm at a matrix, put a matrix placement's pins 5 cm under their place on the frame, and take means by the float reciprocal of the count.

#### Scenario: The pelvis on the frame
- **WHEN** the pelvis is posed 98 cm up and the frame stands 100 cm up
- **THEN** the placed pelvis is 100 cm up

### Requirement: Down body ground probe
A body off its feet SHALL probe its ground with one 10 cm sphere cast 2000 cm straight down from its pelvis plus its zone's origin; a hit SHALL set its ground and previous ground to the hit's height, a miss keep the ground for both.

#### Scenario: A miss
- **WHEN** the probe of a body with ground 12 cm hits nothing
- **THEN** its ground and previous ground are 12 cm
