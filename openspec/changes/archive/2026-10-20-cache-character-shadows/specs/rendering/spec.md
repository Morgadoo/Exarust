# Spec Delta

## MODIFIED Requirements

### Requirement: Cached cube shadows
The system SHALL keep every caster's cube shadow maps across frames, characters' skinned meshes included, as the original does. A face SHALL be rendered only when it is needed (it reaches into the camera's view) and dirty: every face when its light moves or takes a new slot, and, for a caster linked, unlinked or moved, the faces its bounding sphere touched and touches. A face SHALL draw only the casters whose sphere touches it. A face the renderer could not draw SHALL stay dirty. It SHALL hold distance / (R·k_c), compared against 0.995·d / (R·k_r) on receivers.

#### Scenario: Still room
- **WHEN** nothing moves in view for a frame
- **THEN** no cached cube face is rendered that frame

#### Scenario: Flickering torch
- **WHEN** a torch's intensity flickers
- **THEN** its cached faces are not rendered again

#### Scenario: Light on the first frame
- **WHEN** a light takes its slot on the first frame, before the renderer can draw cube faces
- **THEN** its faces are drawn on a later frame instead of staying empty and wholly shadowed

#### Scenario: Character beside a wall torch
- **WHEN** an animated character stands in one face of a wall torch's cube
- **THEN** that face is rendered again each frame with the character posed, and the torch's other faces stay cached
