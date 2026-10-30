## ADDED Requirements

### Requirement: Scan draw
The system SHALL draw a scan node's aura over a copy of its body's skinned meshes every frame until the scan ends, with `Fade` one less the shown fraction and `TexPos` zero, SHALL start a reconstruction device's scan at colour scale 3, and SHALL remove the copies when the scan node leaves.

#### Scenario: The flash
- **WHEN** a device scan's looping close runs
- **THEN** its colour scale is three times the fraction left and the copies go with the node at the end
