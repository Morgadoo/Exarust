## ADDED Requirements

### Requirement: Flame particles
The system SHALL draw small fires' flames as the original's particles. Particles start at rest on a ring around the fire node and rise by buoyancy at the 83 Hz tick. They die young or once they stray past the fire's reach. Each is drawn as a diamond stretched along its motion, textured with a wisp of `FirePrt02` and coloured by the fire's keys over its life, blended premultiplied.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** each torch shows a rising, tapering flame about as tall as the original's
