# Spec Delta

## ADDED Requirements

### Requirement: Factory weapon body
The body of an assembled factory weapon SHALL be fitted to its merged meshes by the factory-weapon shape builder with the factory's fit record (resolution scale 1.1, even axes, at least 6 cells, no free corners, no side growth), its spheres sorted around the weapon's origin and its mobility 10 000 over the fitted volume.

#### Scenario: The golem halberd's body
- **WHEN** a golem guard holding `@GlmHalbrd A` is dressed from the installation
- **THEN** its halberd's body spans from about 106 cm below the grip to the halberd's 176 cm tip

### Requirement: Factory weapon body sound
A factory weapon body's collision sound set SHALL be the factory's hit set for the weapon section's sound byte, a sword's without a section.

#### Scenario: Sound byte
- **WHEN** a factory weapon's section has sound byte 2
- **THEN** its body plays `WH_AxeA`, and with byte 3 `WH_HammerA`

### Requirement: Which factory weapons have a body
A factory weapon held in a hand, held in the off hand or dropped from the inventory SHALL have its body and be held as an item with a rigid root is held; one drawn for the inventory icon or the colour preview SHALL have none.

#### Scenario: Off hand
- **WHEN** a character holds a second factory weapon in the off hand
- **THEN** that weapon is a body gripped to the left hand

#### Scenario: A golem's halberd
- **WHEN** a golem guard is dressed with its factory halberd
- **THEN** the halberd is one body, gripped to the golem's hands, that collides with other bodies
