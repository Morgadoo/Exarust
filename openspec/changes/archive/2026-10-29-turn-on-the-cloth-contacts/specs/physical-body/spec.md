## RENAMED Requirements

- FROM: `### Requirement: Cloth contacts are opt-in`
- TO: `### Requirement: Cloth contacts`

## MODIFIED Requirements

### Requirement: Cloth contacts
The level's cloths SHALL gather body and level contacts unless the cloth contacts switch is off (`EXA_CLOTH_CONTACTS=0`). A cloth's bound radius SHALL be its particles' half-extent in centimetres, scaled by its node. A cloth SHALL NOT meet the static meshes of its own scene node. A sleeping cloth SHALL take only awake bodies, and it SHALL skip the stage while its frame is unchanged and no body sphere reaches it.

#### Scenario: A falling cloth lands and sleeps
- **WHEN** a free cloth falls onto a level floor with the switch on and rests there
- **THEN** it gathers the floor's triangles, stays above the floor and falls asleep

#### Scenario: The cloth's own mesh
- **WHEN** the floor belongs to the cloth's own scene node
- **THEN** the cloth falls through it
