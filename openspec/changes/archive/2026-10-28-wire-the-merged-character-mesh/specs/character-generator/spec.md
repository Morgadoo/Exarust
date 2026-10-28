## ADDED Requirements

### Requirement: Running merged character mesh
The game SHALL draw each dressed character whose body model numbers at least fifteen slots as one skinned mesh merged from the dressed-mesh plan, with one sub-mesh per material run, and SHALL hide the body nodes it replaces and the body slots a garment covers. Node frames SHALL be composed in the file's row-vector convention, so a child one metre along x under a parent turned a quarter about z sits one metre along y.

#### Scenario: Frames
- **WHEN** a transform is turned into a file frame and composed under a parent turned a quarter about z
- **THEN** the round trip gives the same transform and the child's origin and axes match the Bevy hierarchy's

#### Scenario: Runs
- **WHEN** two built parts with different materials are merged and sliced
- **THEN** there are two runs of three vertices each, with rebased indices, their own joints and tangents

### Requirement: Running skin composite
The merged skin run SHALL take the character's composited colour, surface and normal maps, generated once per distinct look (stem, tone, side, layers and beard; not the look seed) and read from the race maps at the target's own coordinates. Until the composite is ready, the run SHALL draw the race maps plus tone.

#### Scenario: Cache key
- **WHEN** two jobs differ only in tone, side, a layer amount or a beard
- **THEN** their keys differ, and an equal job repeats its key
