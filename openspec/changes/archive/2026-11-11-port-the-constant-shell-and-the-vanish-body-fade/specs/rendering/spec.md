## ADDED Requirements

### Requirement: Fire particle textures
Fires up to size 0.35 (torches) SHALL draw their particles with `FirePrt02`, and larger fires and their beds with `FirePrt01`, as the original's fire builder names `_FirePrt02` and `_FirePrt01`.

#### Scenario: A torch's texture
- **WHEN** a wall torch of size 0.2 is drawn
- **THEN** its particles sample `FirePrt02`

### Requirement: Dithered cube casters
A caster carrying a dither SHALL drop the texels of its cube faces where `fract(3.555713358·x + 0.309269245·y)` of the face's pixel position is below the dither, as the original's dithered caster program does.

#### Scenario: A body thinning out
- **WHEN** a vanishing body's dither is 1
- **THEN** it casts nothing into the cube faces

## MODIFIED Requirements

### Requirement: Dirty cube faces
Every face of a light's cube SHALL go out of date when the light moves or takes a new slot. A caster linked, unlinked or moved SHALL put out of date the faces its bounding sphere touched and the faces it touches. A caster's dither coming, changing or going SHALL put its faces out of date.

#### Scenario: Character beside a wall torch
- **WHEN** an animated character stands in one face of a wall torch's cube
- **THEN** that face is rendered again each frame with the character posed, and the torch's other faces stay cached
