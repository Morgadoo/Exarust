## ADDED Requirements

### Requirement: Cloth body collision
A cloth tested against its own body SHALL return unchanged. An accepted hit SHALL make one callback, store its index and side, and, when resolved, SHALL snap the particle to e0 inside the radius or place it at the radius along the edge. Other particles SHALL be unchanged.

#### Scenario: The push-out
- **WHEN** a particle at (0,2,0) hits e0 (0,0,0) with radius 1 and resolves
- **THEN** it becomes (0,−1,0) and one callback runs
- **WHEN** the particle is at (0.5,0,0)
- **THEN** it becomes (0,0,0)

#### Scenario: The rejections
- **WHEN** the body is the cloth's own
- **THEN** nothing runs
- **WHEN** a particle misses, fails a segment test or does not resolve
- **THEN** it is unchanged and the last case still counts its callback
