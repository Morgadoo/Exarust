# Spec Delta

## ADDED Requirements

### Requirement: Placement visibility rules
The system SHALL write a prop's placement flags `0xB80000` over its object node's own before the object spawns. When the object is spawned, a light under a root whose flags have `0x100000` (and not `0x200000`) SHALL be hidden, and a light or fire under a root with `0x200000` (and not `0x100000`) SHALL be hidden once the level's daylight scale is above 0.12 (a NaN or exactly 0.12 leaves it shown). A rule on a node that is not a root SHALL not be read.

#### Scenario: Arena lanterns
- **WHEN** every shipped level is read
- **THEN** exactly seven props carry a rule: arenao1's two `lantern a01 mb` with `0x100000` and one with `0x200000`, arenao3's three `brazier xaa02 01` with `0x200000` and exanima03's `beddouble` with `0x180000`, each a root, and each lantern's and brazier's one light or fire is hidden by day
