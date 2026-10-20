## ADDED Requirements

### Requirement: Morph target switch
Switching SHALL do nothing unless the target's tag is `0xEDC0C589`. Otherwise it SHALL copy the point and points, store the point at the target and refresh it.

#### Scenario: Tagged and untagged
- **WHEN** the target's tag is `0xEDC0C589` and the point is 1, 2, 3
- **THEN** the switch and refresh run and the point is stored
- **WHEN** the tag differs
- **THEN** nothing is copied, stored or refreshed
