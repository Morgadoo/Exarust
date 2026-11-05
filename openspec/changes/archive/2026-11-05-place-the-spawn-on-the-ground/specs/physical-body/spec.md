# Spec Delta

## ADDED Requirements

### Requirement: Spawn placement at a matrix
A living character spawned without a saved pose SHALL take its first clip's pose-only sample, lowered by the pelvis's posed height, onto a frame at its placement's yaw and position raised by the pose-only sampler's pelvis target less 8 cm. That target SHALL be the not-walking margin: the stand height of the posed foot contacts (0 under the Unconscious 1 clip) plus twice the body scale plus 5 cm. No leg snap SHALL run at that spawn.

#### Scenario: A standing human
- **WHEN** a human's first sample stands its pelvis 103 cm over a lowest contact at its sole's height
- **THEN** its target is 110 cm and its pelvis is placed 102 cm over its placement

### Requirement: Placement on the ground
A placement on the ground SHALL run the leg snap from its frame, take the snap's frame height as the frame's, put every part at the current clip's pose-only sample lowered by the pelvis's posed height onto that frame at rest, anchor each foot at its posed contact on the frame less the clip root (unrotated) at its ground plus the sole, and restart every slack link's smoothed length. When no foot finds ground the body SHALL be left as it was.

#### Scenario: A platform
- **WHEN** a biped is placed on the ground over a platform 15 cm up
- **THEN** its pelvis stands 15 cm higher than over the floor, at the snap's height, every part at rest at its posed offset from the pelvis

## MODIFIED Requirements

### Requirement: Placement leg snap
A living body placed into the scene SHALL probe each leg 5·scale cm along its shin from its clip ankle with a 10 cm sphere from the frame's height plus the leg's length 300 cm down, a miss 10 000 cm under the frame, and average the hits into the character's ground. Each anchor SHALL be its clip ankle at its ground plus the sole, its ground and floor that ground, and the standing leg IK SHALL write its corrections at once with no rate and no thigh turn, reading the character frame the body stands on.

#### Scenario: A platform
- **WHEN** a biped is placed over a platform 15 cm up
- **THEN** each leg's ground and floor are 15 cm, each anchor 15 cm plus the sole, the character's ground 15 cm, and the corrections' rates zero
