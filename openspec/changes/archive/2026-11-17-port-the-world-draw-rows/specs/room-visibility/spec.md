## ADDED Requirements

### Requirement: Wall probe over seen sectors
The camera's wall probe SHALL ask an unhidden obstacle mesh when the sector it stands in was seen by the main view at the last classification, whatever its own frustum test gave; a mesh outside the live scene SHALL always be asked.

#### Scenario: A wall behind the eye
- **WHEN** a mesh of the active room stands behind the camera
- **THEN** it is not drawn but its sector is seen, so the probe asks it
