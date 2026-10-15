## ADDED Requirements

### Requirement: GI around the camera
The system SHALL keep the voxel GI block around the point 20 m in front of the camera, rebuilding it when that point moves more than two sectors, so surfaces in view take their ambient from the GI wherever the camera is in a large level.

#### Scenario: Far corner of a large level
- **WHEN** the camera looks at a room far from a large level's centre
- **THEN** the room's ambient comes from the GI volume, not the environment cube
