## MODIFIED Requirements

### Requirement: Shadow casters
The system SHALL cast shadows as the original does. Surfaces with opacity ≤ 0.8, or flagged by the material record, cast no shadows. Surfaces cast through the alpha-tested caster have their shadows cut; other surfaces cast whole.

#### Scenario: Cobwebs in torchlight
- **WHEN** a torch lights a wall hung with cobwebs
- **THEN** the cobwebs' shadows follow their strands instead of darkening the wall in solid sheets

### Requirement: Cached cube shadows
The system SHALL keep static casters' cube shadow maps across frames, as the original does. A face SHALL be rendered only when it is needed (it reaches into the camera's view) and dirty (its light moved, a caster linked to it moved or changed links, or the light took a new slot). A face the renderer could not draw SHALL stay dirty. It SHALL hold distance / (R·k_c), compared against 0.995·d / (R·k_r) on receivers.

#### Scenario: Still room
- **WHEN** nothing moves in view for a frame
- **THEN** no cached cube face is rendered that frame

#### Scenario: Flickering torch
- **WHEN** a torch's intensity flickers
- **THEN** its cached faces are not rendered again

#### Scenario: Light on the first frame
- **WHEN** a light takes its slot on the first frame, before the renderer can draw cube faces
- **THEN** its faces are drawn on a later frame instead of staying empty and wholly shadowed

## ADDED Requirements

### Requirement: Alpha-tested casters
The system SHALL cast a surface through the alpha-tested caster when its material is alpha-tested, or when it has the opacity flag (set by the record or by a loaded opacity map) and an opacity above 0.75 whatever its blending, as the original does. The alpha-tested caster SHALL draw both sides of the surface and SHALL drop every texel whose cut texture (the opacity map, otherwise the diffuse map) has an alpha below ½, without the material's opacity.

#### Scenario: Blended cobwebs at the menu
- **WHEN** the main menu scene is shown with shadows at the original's camera, its alpha-blended cobwebs (opacity 1, with an opacity map) hanging around both wall torches
- **THEN** each torch lights its wall and the floor around it as in the original, broken only by the shadows of props such as the desk, the shelves and the skeleton

#### Scenario: Faint opacity-mapped surface
- **WHEN** a surface has an opacity layer and an opacity of 0.75 or less
- **THEN** its shadow is not cut by the map

#### Scenario: Opacity flag without an opacity layer
- **WHEN** a blended material's record sets the opacity flag but has no opacity layer, with an opacity above 0.75
- **THEN** its shadow is cut by its diffuse map's alpha and cast from both sides
