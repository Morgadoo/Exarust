## ADDED Requirements

### Requirement: Static mode of the placement's Height Query
A placement on the ground asked in the static mode SHALL find the ground under each leg by the Height Query over the level's static meshes only, with no body mesh and no rigid body's stand-in answering; every other placement SHALL take the sphere mode over the level and the listed body meshes.

#### Scenario: A plate over the floor
- **WHEN** a rigid body's stand-in plate lies 50 cm over the floor under a leg
- **THEN** the static mode finds the floor and the sphere mode the plate
