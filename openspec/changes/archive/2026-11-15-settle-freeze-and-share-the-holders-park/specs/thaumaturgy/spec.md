## ADDED Requirements

### Requirement: Spirit body settles
Spirit Form's new body SHALL be built settling for 16 ticks.

#### Scenario: The spirit body is built
- **WHEN** the spirit body's simulated body is built
- **THEN** it settles for 16 ticks

## MODIFIED Requirements

### Requirement: Barricade dismissal pick
A barricade cast while one runs SHALL take the nearest-mesh pick under the cursor (the ground plane's point when it meets nothing) and SHALL end the running barricade when the segment from the eye to that point meets its node's sphere and then its box by the separating-axis test.

#### Scenario: Cast again over the wall
- **WHEN** a barricade stands at the cursor's ground point and the player casts it again with the cursor unmoved
- **THEN** the running barricade ends and the cast answers 2

#### Scenario: A segment that stops short of the wall
- **WHEN** the segment is aimed at the wall's box but ends short of it, with no separating axis between them
- **THEN** the separating-axis test counts it as meeting the wall, where the slab test would not
