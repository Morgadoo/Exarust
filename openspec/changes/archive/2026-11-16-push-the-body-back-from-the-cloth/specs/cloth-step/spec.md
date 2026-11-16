## ADDED Requirements

### Requirement: Cloth grab record
A hand's hold SHALL be a grab record: weights over three particles, the hand's target, a radius from the point's place in the mesh to the nearest pin's (5 cm with no pin, 0 at or under the small float), and a smoothed point starting at the target. The drag SHALL set the newest record's target and wake the cloth to 32; the release SHALL drop the newest record.

#### Scenario: Radius to the nearest pin
- **WHEN** a hold at weights (0.5, 0.25, 0.25) over (0,0,0), (10,0,0), (0,10,0) cm is made with pins on the third corner and one 30 cm away
- **THEN** its radius is the distance from (2.5, 2.5, 0) to (0, 10, 0)

### Requirement: Cloth grab follower
Each awake integrate SHALL move a hold's smoothed point before the constraint passes: the target, pulled onto the sphere of the hold's radius about its pin's particle when beyond it, then the motion of the last tick plus 0.1 of the way to that target, times 0.3.

#### Scenario: A new target
- **WHEN** a hold resting at a target takes a target 10 cm on
- **THEN** its point moves 0.3 cm, then 0.3·(0.3 + 0.97) cm more

### Requirement: Cloth grab spread
Each constraint pass SHALL, after the pins, move each hold's point on its particles onto its smoothed point, each particle by the gap times its weight over the largest weight.

#### Scenario: A hold on a triangle
- **WHEN** a hold at weights (0.5, 0.25, 0.25) is 4 cm below its smoothed point
- **THEN** the three particles rise 4, 2 and 2 cm

### Requirement: Cloth tangent rebuild
Once a frame, a moved first-part cloth mesh carrying tangents and texture coordinates SHALL have its tangents rebuilt from the moved faces: each face's u-gradient through the exact reciprocal, summed into its vertices, less its part along the vertex's rebuilt normal, scaled by the approximate reciprocal square root; the tangent's fourth lane SHALL keep the sign the mesh was loaded with.

#### Scenario: A corner rises
- **WHEN** a normal-mapped one-triangle cloth's corner rises 5 cm and the frame is drawn
- **THEN** its tangents are the rebuild of the moved face with their fourth lanes unchanged, and a later part's tangents are unchanged

### Requirement: Cloth object grid on the scene grid
The cloth object list and the body parts' index SHALL take the scene's grid: a cell is its flat index, the nine around one are the flat offsets, and a cloth in the first or last row SHALL be parked at `columns + 1`; a scene without a grid SHALL keep the 4096 × 4096 stand-in.

#### Scenario: Across the last column
- **WHEN** a cloth stands in column 7 of an 8 × 8 scene grid and a body part in column 0 of the next row
- **THEN** the part is offered to the cloth, and a cloth in row 0 is parked

### Requirement: Cloth motion energy
Each awake integrate SHALL set the cloth's motion energy to `min(√sum² · 0.001, 1)` (the product in double) when the squared velocity sum is over the small float, else to 0.

#### Scenario: A 5 cm sum
- **WHEN** a one-particle cloth moved 5 cm last tick
- **THEN** its energy is 0.005

## MODIFIED Requirements

### Requirement: Cloths step independently
The cloth stage SHALL leave each cloth that pushes no body exactly where stepping it alone would leave it, whatever the other cloths of the level, while the object list's re-buckets keep the list's order; cloths that push bodies SHALL meet them one at a time in the list's order, so two runs end bit for bit alike.

#### Scenario: Fifteen hairs
- **WHEN** fifteen hair cloths hang from fifteen living bodies among 336 body parts for 40 ticks
- **THEN** each hair's particles are bit for bit those of the same hair stepped with the other fourteen removed
