# Spec Delta

## MODIFIED Requirements

### Requirement: Capsule executes the decision
A character without muscles SHALL execute the movement decision by its desired velocity: along the heading at 1.4 m/s walking up to 3.6 m/s at a full run blend (rising 0.05 a tick to the encumbrance's cap, falling 0.015), 1.22 m/s backpedalling, slowing while braking by the original's brake drive and enough to stop on its goal. Without a navigation grid the look-ahead SHALL clamp the goal 28 cm and a 30 cm cell short of a wall at hip height.

#### Scenario: Soft stop
- **WHEN** W is released at a walk
- **THEN** the character slows from 1.687 cm a tick and stops within 3 cm of the goal 8·v + 20 cm ahead
