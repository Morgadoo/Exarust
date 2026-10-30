# Spec Delta

## ADDED Requirements

### Requirement: Smoke pass draws
With supersampling on, the smoke draw SHALL repeat in each pass once per sub-view the pass draws (the level's first-pass count, at most four), each with that sub-view's viewport and matrix; the program cache SHALL keep one program per key and a failed build as −1, skipping the mesh.

#### Scenario: Eight sub-views
- **WHEN** the supersampling level draws eight sub-views
- **THEN** each of its two passes draws a smoke mesh four times

### Requirement: Material environment cube
A record whose material keeps its reflection layer SHALL bind that layer's texture as its environment cube when it loads as a six-face cube, the scene's cube name sharing the scene's image; otherwise the level's cube SHALL stay.

#### Scenario: A 2-D reflection texture
- **WHEN** a record's reflection layer names a texture that is not a cube
- **THEN** the material keeps the level's cube

### Requirement: Character icon renderer set-up
The character icon renderer SHALL use a private view with a 30° field of view and a 10 cm near plane, a render target twice the icon's side with an sRGB colour buffer (with alpha for a cut-out) and a depth buffer, and an RGBA8 destination image with flags 0x20000000 at the icon's size; a new size SHALL resize the image, target and view only.

#### Scenario: A 64-pixel opaque icon
- **WHEN** a 64-pixel item icon renderer is created
- **THEN** its target is 128 × 128 with an sRGB8 colour buffer and its destination 64 × 64 RGBA8
