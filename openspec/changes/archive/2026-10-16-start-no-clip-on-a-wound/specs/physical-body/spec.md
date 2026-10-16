## MODIFIED Requirements

### Requirement: Joints at the part origins
The system SHALL join every part that has a parent part by its skeleton links and then a pivot from its origin to its place in the parent, the model's local translation of the part. A link's parent point SHALL be the stored point plus that translation, unturned. A correction SHALL be shared m/(m + m_parent), m being 0.175 times the bone's mobility, and spread over each part's particles by the point's weights, unclamped. A joint's range SHALL come from its links alone.

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

#### Scenario: Elbow and knee ranges
- **WHEN** the installed `humanoid.rsq` links of each forearm and lower leg are turned about their hinge axes
- **THEN** each elbow bends from straight to 120° and not past straight, where 10° past straight stretches its long link 1.8 cm beyond its slack, and each knee bends from 10° to 130°
