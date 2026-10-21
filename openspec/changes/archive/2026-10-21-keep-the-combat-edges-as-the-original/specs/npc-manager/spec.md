## MODIFIED Requirements

### Requirement: Alert and approach
Raising the group alert SHALL add one and hold it 10 s. A step from outside a circle of 60 × the target's scale SHALL be cut where it crosses it when the original's test (the cut point's z offset and an unwritten 0) has a cosine of 0.75 or more, or none. Inside, a distance squared under the radius SHALL push out to the circle, else the step SHALL be dropped.

#### Scenario: A 60 cm circle
- **WHEN** a character 100 cm from a target of scale 1 steps 60 cm, 30 cm and diagonally past it, and characters 5 cm and 30 cm away step 30 cm in
- **THEN** the 60 cm step is cut to 40 cm, the others are kept, the 5 cm one is pushed out to 60 cm, and the 30 cm one is dropped
