## ADDED Requirements

### Requirement: Balance
The system SHALL keep each living body's balance. The centre of mass, projected ahead by its velocity, is measured against the segment between the feet with a threshold of (energy·0.75 + 0.25)·30 cm. The score is stability⁸. A counter grows by 10 − 10·score per tick and at 100 sets the balance to the score. Otherwise balance recovers by (5 − speed in cm/tick)·0.01 per tick.

#### Scenario: Standing still
- **WHEN** an alert body stands with its centre of mass over its feet
- **THEN** it keeps its balance indefinitely

### Requirement: Falling and getting up
The system SHALL make a body whose balance drops below 0.6 fall: it loses its controller's support, its tone drops to 0.5, it plays the fall clip in the direction it is going, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, placed under the body.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2`, and stands where it fell
