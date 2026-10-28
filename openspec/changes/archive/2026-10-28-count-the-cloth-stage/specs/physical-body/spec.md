## REMOVED Requirements

### Requirement: Cloth contacts are live
**Reason**: exanima01 ran at 14 FPS with the contacts on (round 26 GPU check).
**Migration**: `EXA_CLOTH_CONTACTS=1` turns them on; see "Cloth contacts are opt-in".

## ADDED Requirements

### Requirement: Cloth contacts are opt-in
The level's cloths SHALL gather body and level contacts only when the cloth contacts switch is on (`EXA_CLOTH_CONTACTS=1`). A cloth's bound radius SHALL be its particles' half-extent in centimetres, scaled by its node. A cloth SHALL NOT meet the static meshes of its own scene node. A sleeping cloth SHALL take only awake bodies, and it SHALL skip the stage while its frame is unchanged and no body sphere reaches it.

#### Scenario: A falling cloth lands and sleeps
- **WHEN** a free cloth falls onto a level floor with the switch on and rests there
- **THEN** it gathers the floor's triangles, stays above the floor and falls asleep

#### Scenario: The cloth's own mesh
- **WHEN** the floor belongs to the cloth's own scene node
- **THEN** the cloth falls through it

### Requirement: Cloth stage counters
With `EXA_DEBUG_CLOTH=1`, the cloth stage SHALL log once a second (83 ticks) the per-tick means of its time and work, then one line per cloth for the last tick. The work counted SHALL include awake, sleeping and skipped cloths, body parts offered, spheres kept and swept, the sweep tests, static records and own-node skips, level pieces, triangles and particle-triangle tests. Without the flag, the stage SHALL take no timings.

#### Scenario: The level stage's tests
- **WHEN** two of three particles reach a level piece's box holding one triangle
- **THEN** the cloth's level-test count is 2
