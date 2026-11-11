## ADDED Requirements

### Requirement: Sector list order
Each body SHALL take a place in the sector's list when it is first inserted, later bodies after earlier ones, and keep it when it is moved between worlds; characters and props SHALL be inserted in spawn order. The solver, its body–body detection and the body grid SHALL walk the bodies from the list's end (the last to join first), each body's parts from the last in its node order to its root. The detection SHALL meet, for each awake part, the parts of its nine cells (its own first) in the order a cell's list holds them, the last to join first.

#### Scenario: Two bodies
- **WHEN** two bodies are inserted one after the other, and the first is lent away and inserted again
- **THEN** the second is walked before the first, and the first keeps its place

#### Scenario: Two equally mobile weapons clash
- **WHEN** two parts with the same key meet
- **THEN** the one that joined the list first brings its spheres, whichever was walked first

#### Scenario: An all-pairs sector
- **WHEN** an awake part is listed after a sleeper it touches in a sector without a grid
- **THEN** the two do not meet

## MODIFIED Requirements

### Requirement: Contacts between bodies
The system SHALL keep body parts out of each other, for every body: after the iterations each awake part meets the parts near it, awake or asleep, that it has not met this tick. The part with the smaller key round(100/mobility) brings its spheres against the other's (of two equal keys, the one that joined the sector's list first), each where it is, or swept from the last tick when it moved over 0.1 cm² relative to the other part. Each hit pushes both at once; a pair keeps up to four points, spread widest over the sphere side.

#### Scenario: Two limbs meet
- **WHEN** two limbs with 10 cm spheres are thrown at each other at 2 m/s each
- **THEN** they stop at their spheres in the tick they meet, 0.6 cm in, and part again; without the contacts they pass through each other

#### Scenario: A cup on a table
- **WHEN** a light body (mobility 10, an 8 cm sphere) drops onto a heavy sleeping one (mobility 0.05, 6 cm spheres) lying on the floor
- **THEN** it wakes the heavy one, and both sleep again with the light one resting on the heavy one's spheres, 14 cm from the nearest centre

#### Scenario: Two sleepers
- **WHEN** two sleeping bodies lie against each other
- **THEN** no contact is looked for and both stay asleep

#### Scenario: Which side brings its spheres
- **WHEN** a human's pelvis (mobility 0.3) and hand (mobility 1.67) meet
- **THEN** their keys are 333 and 60, and the hand brings its spheres
