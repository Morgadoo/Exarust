## ADDED Requirements

### Requirement: Living placement words
A living placement at a matrix SHALL set the body's tone and energy to 1, its carry to zero and its step radius base to 60 cm, end any step under way, take its standing position anew, and set every leg's IK corrections to the identity, before it puts the parts. It SHALL leave the sleep countdown as it is.

#### Scenario: A dazed body placed
- **WHEN** a body of tone 0.3 and energy 0.4 that is stepping is placed at a matrix
- **THEN** its tone and energy are 1, it does not step and its radius base is 60 cm

### Requirement: Living placement parts and feet
A living placement SHALL put every part at rest at the clip's fresh pose lowered by the pelvis's posed height onto the frame, and each foot's anchor, step target, previous anchor and ground at its posed contact on the frame less the clip root, raised by the sole and lowered 8 cm, in centimetres; it SHALL clear every foot state, take the feet centre as the anchors' mean and restart the chest with no velocity.

#### Scenario: A biped placed turned
- **WHEN** a biped's own pose is placed on a frame turned 0.5 rad at (2, 1.2, −1) m
- **THEN** its pelvis is at (2, 1.2, −1) m and each anchor is its contact on the frame less the root, 8 cm under the sole's height

### Requirement: Living placement frame
A living placement SHALL take its pelvis height from its caller's rule: the frame's own height, or over the frame's height the fresh pose's stand height, or its pelvis target plus a number of centimetres. The controller SHALL go to the frame's place across the ground, at the frame's height unless the height is the frame's own, the model SHALL take the frame's turn, and the locomotion SHALL take the placement's facing with no turn, no move and a clear flag word.

#### Scenario: Placed at a target
- **WHEN** a footless body is placed at (10, 0.5, 6) m facing 1 rad by its pelvis target plus 5 cm
- **THEN** its pelvis stands (2·scale + 5 + 5) cm over 0.5 m at (10, 6) and its yaw is 1 rad

### Requirement: Living placement held items
A living placement SHALL ask for its body's held items to be put back in its hands after it, unless its caller asked otherwise (the settling placement of a story respawn).

#### Scenario: A placed body with a weapon
- **WHEN** a body is placed living
- **THEN** its held items are asked to be re-placed

### Requirement: Surviving player's level placement
The surviving player's level placement SHALL put its body living at the probed feet, facing the placement's yaw, its pelvis at the pose's pelvis target over them, re-placing its held items unless it settles.

#### Scenario: A loaded save's player
- **WHEN** a saved player is placed on its level at (3, 0.4, −2) m facing 0.7 rad
- **THEN** its body is put living there at the pose's pelvis target, its held items re-placed
