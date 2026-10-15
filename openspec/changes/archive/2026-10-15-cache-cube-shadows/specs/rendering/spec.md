## ADDED Requirements

### Requirement: Cached cube shadows
The system SHALL keep static casters' cube shadow maps across frames, as the original does. A face SHALL be rendered only when it is needed (it reaches into the camera's view) and dirty (its light moved, a caster linked to it moved or changed links, or the light took a new slot). It SHALL hold distance / (R·k_c), compared against 0.995·d / (R·k_r) on receivers.

#### Scenario: Still room
- **WHEN** nothing moves in view for a frame
- **THEN** no cached cube face is rendered that frame

#### Scenario: Flickering torch
- **WHEN** a torch's intensity flickers
- **THEN** its cached faces are not rendered again
