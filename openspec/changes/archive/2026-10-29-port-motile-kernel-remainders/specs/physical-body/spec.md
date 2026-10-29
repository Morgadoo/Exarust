## ADDED Requirements

### Requirement: Standing body has no drive
A body that does not locomote in a tick SHALL have no locomotion drive that tick, so the carry is not pushed by an earlier walking tick's drive.

#### Scenario: A recovery step after a walk
- **WHEN** a walking body stops and takes a recovery step
- **THEN** its locomotion drive is zero during the step

### Requirement: Golem link smoothing
A golem's body SHALL give every body link of every part the smoothing 0.995 when it is built; other races SHALL keep the smoothing their link points give.

#### Scenario: A golem is spawned
- **WHEN** a golem character's body is built
- **THEN** each of its body links has the smoothing 0.995

### Requirement: Feet modes
The feet SHALL run in one of three modes: flat floor (every ground 0, the anchor at the sole height plus 1 cm, the pelvis aimed 10 cm under its standing height), hovering (one probe from 20 cm above the pelvis, 5 cm higher after each miss, a hit giving the ground) or walking; the flat-floor bit SHALL win over hovering.

#### Scenario: A hovering probe misses
- **WHEN** a hovering body's probe finds no ground twice
- **THEN** the next probe starts 30 cm above the pelvis and the ground is unchanged

### Requirement: Leg order
A body with more than one leg SHALL swap its first two legs when the first leg's thigh sits at positive x in the root.

#### Scenario: A human body
- **WHEN** a human body is built (first thigh at negative x)
- **THEN** its legs keep their order
