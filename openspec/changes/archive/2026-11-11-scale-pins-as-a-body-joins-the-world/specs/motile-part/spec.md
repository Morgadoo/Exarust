## ADDED Requirements

### Requirement: Pins scaled as a body joins the world
Every part's pins SHALL be scaled about the part's origin by its character's size (the race's size times 1 − 0.05·height) when its body is built, before the chest and support pins take their weights. A character of size 1 SHALL keep the stored pins.

#### Scenario: Tall human
- **WHEN** a human of size 1.05 joins the world with chest pins 40 cm over the thorax's origin
- **THEN** its chest point lies 42 cm from the thorax's origin

#### Scenario: Average human
- **WHEN** a human of size 1 joins the world
- **THEN** its pins are the stored ones

### Requirement: Setup smoothing on body links
The muscle setup SHALL compute a link's smoothing from its point's distance only for a link that carries the body bit, which every skeleton link of a part with a parent carries.

#### Scenario: Skeleton link
- **WHEN** a part with a parent is built from the skeleton
- **THEN** each of its links has the body bit and the setup's smoothing

### Requirement: Plain parts take the rigid step
A plain part of a living body SHALL step as a rigid body: its motion kept by the retention of its own speed band (or its environment's), then gravity, with no carry, no living retention and no pose drive. The body's muscled parts SHALL keep the living step.

#### Scenario: Slow plain part
- **WHEN** every part of a living body moves 0.0012 cm a tick and one part is plain
- **THEN** the plain part keeps the resting retention and falls by gravity, and the muscled parts keep the living retention

## MODIFIED Requirements

### Requirement: Ghoul chest pins
A ghoul's or hulking ghoul's body SHALL take its thorax pins, already scaled by the character size as the body joined the world, scaled again about the thorax's origin by half its character size, their weights taken again over the part's rest; every other race SHALL keep the pins as joining the world left them.

#### Scenario: Half size
- **WHEN** the chest pins are scaled by 0.5
- **THEN** the chest point lies half as far from the thorax's origin
