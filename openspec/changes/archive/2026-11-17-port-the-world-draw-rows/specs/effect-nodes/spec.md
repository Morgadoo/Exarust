## ADDED Requirements

### Requirement: Fire drives the host's geometry children
A fire SHALL scale the emissive of its host's first part and of the first part of every geometry child of the host by its emissive value, each from its own loaded colour.

#### Scenario: A host with a geometry child
- **WHEN** a fire of emissive 0.5 rides a host whose child has a loaded emissive colour
- **THEN** the child's emissive is half its loaded colour

### Requirement: Flame row animation per row
Each flame row SHALL draw with its own `_FlameMat01` whose `Anim` takes that row's fade.

#### Scenario: Two rows
- **WHEN** rows of fade 1 and 0.25 draw in one frame
- **THEN** their materials differ and carry 0 and 0.375 as the fade value
