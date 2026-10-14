## ADDED Requirements

### Requirement: Stored mip levels
The system SHALL decode and upload every mip level an `.rfi` stores, including for textures decoded on the CPU, so all textures keep their full resolution up close and filter smoothly at a distance.

#### Scenario: Height map in the distance
- **WHEN** a BC4 height map is decoded on the CPU
- **THEN** its texture carries every stored mip level, not only the base level
