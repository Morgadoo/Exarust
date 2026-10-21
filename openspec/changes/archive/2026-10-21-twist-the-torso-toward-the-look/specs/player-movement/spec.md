## ADDED Requirements

### Requirement: Torso twist
Each body tick a body not helpless and over 0.85 tone SHALL twist its torso toward its look: the look's yaw in last tick's frame (by the original's arc cosine), clamped to ±1 rad, followed 0.05 rad a tick standing, 0.02 locomoting, or the weapon's handling × 0.12 × the swing strength attacking; its direction lowered by the crouch (0.3 of it attacking), its height × 0.75 (−height² casting a power with a low one). Otherwise the twist SHALL be the identity.

#### Scenario: Looking aside
- **WHEN** a standing human looks 0.5 rad off its yaw for 11 ticks
- **THEN** its twist's yaw is 0.05 after one tick and 0.500297 after eleven

#### Scenario: Crouched
- **WHEN** a standing human crouches fully looking ahead
- **THEN** its twist looks along (0, −0.6, 0.8)

#### Scenario: Down
- **WHEN** the body is down
- **THEN** the twist is the identity

### Requirement: Spine takes the twist
The sampler SHALL turn the midline chain (from the root part, its first child within 3 cm of the midline, recursively, but the head) by the twist, the part under the root halfway back (its forward midway, its up square to it), each part keeping its place.

#### Scenario: Lumbar and thorax
- **WHEN** the twist turns 0.6 rad about the vertical
- **THEN** the thorax turns 0.6 rad, the lumbar 0.3, and both stay where the pose puts them
