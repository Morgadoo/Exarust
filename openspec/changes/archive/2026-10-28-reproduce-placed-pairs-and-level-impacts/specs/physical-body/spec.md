# Spec Delta

## MODIFIED Requirements

### Requirement: Pair impact
A body pair SHALL keep the greatest squared relative travel of its hits, from the parts' smoothed velocities (a tenth of the way to each tick's motion) at its contact weights; when it passes 1 between two different trees its impact SHALL be that times the negated cosine of the travel against the summed push, the cosine being 0 when the product of their squared lengths is at most 1e-8. Over 0.5 it SHALL strike each side with the impact over 2·m + 1 of the other side's contact mobility m.

#### Scenario: Two limbs meeting fast
- **WHEN** two bodies' limbs meet head-on at speed
- **THEN** each side is struck with the impact scaled by the other's mobility

#### Scenario: A degenerate push
- **WHEN** a 3 cm travel meets a push of 1e-7 m, or of 0.01 m, straight against it
- **THEN** the impact is 0, or 9

## ADDED Requirements

### Requirement: Placed pairs
A body SHALL be placed while its settling countdown is above zero. A pair with exactly one placed body SHALL test spheres without sweeping, record contacts without impact hits, and in each pass move only the placed side by the whole correction over its non-negative weights, recording the opposite push in its offset array; the other side SHALL NOT move.

#### Scenario: Only the placed side moves
- **WHEN** a placed limb keeps a point 0.51 cm deep in an unplaced one
- **THEN** each of its four equally weighted particles moves 0.125 cm, its offset records −0.125 cm each, and the other limb stays

#### Scenario: A placed hit never strikes
- **WHEN** a ball thrown at a settling ball touches it
- **THEN** the pair is kept with no impact hits and neither is struck

### Requirement: Level impacts
A reacting part's level hits in one detection SHALL give a level impact when the greatest squared smoothed velocity at their contact weights passes 0.1 cm² a tick² and that square times the negated cosine against the summed pushes passes 0.1; its point SHALL be the summed points times the count's reciprocal. A foot SHALL NOT react.

#### Scenario: A falling crate
- **WHEN** a crate dropped from a metre lands, then lies still
- **THEN** it makes a level impact on landing and none while it lies

### Requirement: Slider link smoothing
Converting a body's stored links SHALL give every slider link (flag 4), a root's link to the world included, a smoothing of 0.01 whatever the file stores.

#### Scenario: Stored 0.2 and 0.3
- **WHEN** a child's slider link stores 0.2 and a hung crate's world slider stores 0.3
- **THEN** both take 0.01

### Requirement: Sector registration kinds
The registration walk SHALL build a body node's (flag 0x10000 without 0x80000) rigid body unless already built, then only insert it in the grid; a body with class bit 0x20000000 SHALL join the object list; a node with flags 0x89 and class bit 0x200000 SHALL register a static collider whose mask is 1, plus 4 when flagged or thinner than 1 cm and facing up over 0.99.

#### Scenario: Kinds
- **WHEN** a body node with class bit 0x20000000 and a 0x89 mesh node with class bit 0x200000 register
- **THEN** the first is a simulated object and the second a static collider
