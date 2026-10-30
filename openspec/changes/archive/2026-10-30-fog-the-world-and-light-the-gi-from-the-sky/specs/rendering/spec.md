# Spec Delta

## ADDED Requirements

### Requirement: GI sky light injection
The system SHALL inject the GI manager's sky light into every drawn GI sector's group ahead of its point lights: the environment's record fog colour as the frame leaves it (lightning included) times 3 × 0.3 × 0.01, shining straight down, times the voxel's openness and its sky visibility. It SHALL be left out when the colour times 3 has a third component of at most 1.19e−7. The scene's sun light SHALL NOT be injected into the GI.

#### Scenario: Sky light in a group
- **WHEN** a drawn sector lists five point lights under a sky
- **THEN** its group's first light is the sky light and it counts towards the twelve

#### Scenario: Indoors
- **WHEN** an indoor level, whose record fog colour is zero, is lit
- **THEN** its GI has no sky light

### Requirement: GI sky visibility
A voxel SHALL see the GI sky light when it is at or above its column's sky floor: the slice above the column's highest voxel with an occupancy byte of 10 or more, or slice 0 for a column without one.

#### Scenario: Under a roof
- **WHEN** a column has a roof voxel at slice 2 above an open voxel
- **THEN** the sky light reaches slices 3 and up of that column and not the voxel under the roof

## REMOVED Requirements

### Requirement: GI sun injection
**Reason**: The original's GI directional light is the manager's sky light (the record fog colour times 3, straight down, with its own map); the scene sun never reaches the volume.
**Migration**: See GI sky light injection.
