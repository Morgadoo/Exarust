# Spec Delta

## ADDED Requirements

### Requirement: Long Door grid answer
The Long Door's grid query SHALL ask from its subject (the target, or the caster without one) toward the point 300 cm behind it along the subject's forward vector, and SHALL use the path's last point only when the path has at least two points and dz² + dx² from the subject's place exceeds 200 in single precision. The arrival facing SHALL be the target's place less the found point, level, scaled by the approximate reciprocal square root.

#### Scenario: Fifteen centimetres
- **WHEN** the path ends 15 cm across the floor from the subject
- **THEN** the answer is found

#### Scenario: A short path
- **WHEN** the path has one point
- **THEN** the answer is impossible

### Requirement: Shock burst record
Shock's burst SHALL be a non-respawning arc record with flags 0x30310, ten tendrils of fourteen points, life 40, size 10, growth 0.25, lift 0.5, light 1, no hum and a shock strength of 0.05, at the hand's −15, −5 cm.

#### Scenario: The live burst
- **WHEN** the charge passes one
- **THEN** the burst spawned at the right hand carries strength 0.05

### Requirement: Force dome GI colour
The force Barrier's and force Bubble's GI shapes SHALL multiply their colour by the manager's light scale once, and the Barrier's shape matrix SHALL be the inverse of its cap node's frame, turned to its facing at its drawn centre.

#### Scenario: A standing barrier
- **WHEN** a standing barrier's dome light is built at light scale 0.09
- **THEN** its colour is the dome colour times 0.8 times 0.09

### Requirement: Fire bed object bits
A fire preset's bed SHALL take the object bits 0x9000 in the medium-small tier and 0x8000 in the medium and large tiers.

#### Scenario: The medium tier
- **WHEN** a fire of size 0.8 is built
- **THEN** its bed's object bits are 0x8000
