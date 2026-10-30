## ADDED Requirements

### Requirement: Run-scene defaults
The scene editor model SHALL copy the original's seven-word default camera record into the task when the run options name none, and its listener step SHALL copy the camera's two vectors into the listener and, while the listener's flag word carries 0x10, set the listener's point to `camera + (followed − camera)·0.5`.

#### Scenario: A followed object
- **WHEN** the camera is at the origin, the followed object at (8, 4, −2) and the listener flag carries 0x10
- **THEN** the listener's point is (4, 2, −1)
