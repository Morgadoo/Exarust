## MODIFIED Requirements

### Requirement: Limp bodies
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity with speed-banded velocity retention, and joined to its parent at its origin and by the skeleton's links. A limp pivot SHALL close strength/(1 + 0.001|d|²) of its gap d in cm, and a slack link SHALL take back strength times its excess beyond the slack.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, every slack link stays within its slack and every part stays within a centimetre of its socket

### Requirement: Body iterations
The system SHALL solve a body in the original's order each tick: 20 iterations in which each part takes its links to its parent, then its level contacts from the last tick, then restores its edges (a living body children first, from the hands and feet in; a limp body parents first, depth first, from the pelvis out); then one test of the sphere colliders against static geometry, and 8 friction passes.

#### Scenario: Link and contact together
- **WHEN** a limb resting on a floor is pulled down by its parent within a tick
- **THEN** at the end of the tick its link is within its slack and its spheres are above the floor

#### Scenario: Solving order
- **WHEN** a body's parts branch from the pelvis into two chains
- **THEN** a limp body takes each chain from the pelvis out, one chain after the other, and a living body takes every part after all of its children

## ADDED Requirements

### Requirement: Joints at the part origins
The system SHALL join every part that has a parent part by its skeleton links and then a pivot from its origin to its place in the parent, the model's local translation of the part. A link's parent point SHALL be the stored point plus that translation, unturned. A correction SHALL be shared m/(m + m_parent), m being 0.175 times the bone's mobility, and spread over each part's particles by the point's weights, unclamped.

#### Scenario: Shipped human body
- **WHEN** the human body is built from the installed `humanoid.rsq` and `humanbase.rfc`
- **THEN** each femur has 3 links and each hand 4, the right humerus's first link meets the shoulder at (−21.58, −8.56, 6.51) cm, and the right knee's pivot sits at (−0.30, −43.19, −1.09) cm in the femur

#### Scenario: Shares
- **WHEN** a knee's link is corrected
- **THEN** the lower leg takes 0.588 of the correction and the femur 0.412

#### Scenario: Hips under a shove
- **WHEN** a standing body takes a 4 m/s blow to the torso
- **THEN** after every tick each femur's origin is within 2 mm of its socket on the pelvis

#### Scenario: Knee hinge
- **WHEN** a lower leg is struck sideways and forward
- **THEN** it turns about the knee's lateral axis and stays within 5° of the plane across it

### Requirement: Living joint corrections
In a living body a pivot (slack at most 0.01 cm) SHALL close d/(1 + 0.001|d|) of its gap d in cm, in full however far apart, and a slack link SHALL take back its excess beyond the slack at full weight, cut to 80 cm.

#### Scenario: Pivot pass
- **WHEN** a living pivot's two points are 10 cm apart
- **THEN** one pass closes 10/1.01 cm of the gap, shared by the two parts

#### Scenario: Torn limit
- **WHEN** a living slack link is stretched 3 m beyond its slack
- **THEN** one pass takes back 80 cm

### Requirement: Link strength and smoothing
Each link SHALL keep a strength: 1, except a lower leg's first link with over 10 cm of slack, its leg limit, at 0.2 while the body lives and 1 once it goes limp; the femur's first such link SHALL be its leg limit. Each link SHALL keep the smoothing (1 − (r² − 25)·0.001)², at most 0.99 and 0 within √0.1 cm, r being its point's distance from the part's origin in cm.

#### Scenario: Human leg
- **WHEN** a living human body is built
- **THEN** each lower leg's 12.66 cm link pulls at 0.2, each femur's 14.88 cm link is its leg limit, and the femur's point 12.49 cm from its origin has smoothing 0.755

#### Scenario: Gone limp
- **WHEN** the body goes limp
- **THEN** each lower leg's limit link pulls at full strength
