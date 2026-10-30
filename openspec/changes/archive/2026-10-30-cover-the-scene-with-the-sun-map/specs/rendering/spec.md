## MODIFIED Requirements

### Requirement: Fitted sun map in terrain levels
In a level with a terrain the system SHALL replace the sun's cascades for each active camera with one orthographic map fitted to that camera's view; every other sunlit level SHALL get one whole-scene map per active camera; every level SHALL keep two cascades over 60 m when the fit is turned off.

#### Scenario: A terrain level
- **WHEN** an outdoor level with a terrain is shown
- **THEN** the sun has one shadow cascade per camera, sampled out to 125 m

#### Scenario: A level without a terrain
- **WHEN** a sunlit level without a terrain is shown
- **THEN** the sun has one shadow cascade per camera covering the whole scene, sampled out to the scene's diameter

## ADDED Requirements

### Requirement: Whole-scene sun map
The whole-scene map SHALL be centred on the scene box's centre without a texel snap; its half extent and up-sun reach SHALL be the box's radius, raised to the scene's minimum extent (1 000 cm) only when the minimum is greater; its down-sun reach SHALL be the scene's radius when greater than the minimum and the minimum otherwise (a NaN radius giving the minimum); its softness SHALL be (c + 1)·(200 / half extent) with no floor. The fitted mode SHALL apply no minimum extent.

#### Scenario: A small scene
- **WHEN** the scene box's half extents are 100 cm each way and the radius passed down-sun is 10 cm
- **THEN** the half extent, up-sun and down-sun reaches are all 1 000 cm and the softness is 0.2

#### Scenario: A NaN far field
- **WHEN** the down-sun radius is NaN
- **THEN** the down-sun reach is the 1 000 cm minimum

#### Scenario: A small fitted view
- **WHEN** the fitted mode fits a view whose corners make a 173 cm sphere from the initial 3 000 cm
- **THEN** the half extent becomes 200 cm

### Requirement: Sun caster bias
The sun map's caster pass SHALL use the polygon factor (1/(h·0.001) + 1, in double, stored single) + (c'·20)/(h·0.01) in double, with one unit, and SHALL shift only the casters' depth by 0.01·w, where w is half of the smaller of (c' + 1)·500 / D and 0.3 (compared in double), D being the box's up-sun plus down-sun reach.

#### Scenario: The constructor's box
- **WHEN** h is 1 000 cm, c' is 0 and D is 16 000 cm
- **THEN** the factor is 2 and the depth shift is 1.5625e-4 of the engine's [−1, 1] range

### Requirement: Sun caster walk nodes
The caster walk SHALL skip nodes without the caster flag, keep the list's head without the volume test or a volume push, skip a later node whose box lies wholly beyond a volume face and otherwise push the volume's faces in place by the node's vector, and set a kept node's map-visible bit and clear a failed one's.

#### Scenario: A head far from the volume
- **WHEN** the list's head lies outside the caster volume and carries a vector
- **THEN** the head is kept, its outside parts do not draw and the volume does not move

### Requirement: Sun caster walk parts
For each flagged part of a kept node the caster walk SHALL set or clear the part's visible bit by the volume test, switch the cull face once per inside part, and draw each of its meshes whose draw flag is set, alpha-cut ones through the alpha-tested program.

#### Scenario: A part with a skipped mesh
- **WHEN** an inside front-culling part has a drawn plain mesh, a skipped mesh and a drawn alpha-cut mesh
- **THEN** two meshes draw, the face switch rides on the first, and the second goes through the alpha-tested program
