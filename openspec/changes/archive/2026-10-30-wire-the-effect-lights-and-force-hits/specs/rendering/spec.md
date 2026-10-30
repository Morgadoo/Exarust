# Spec Delta

## MODIFIED Requirements

### Requirement: GI shape injection
The system SHALL add every registered GI shape's fragment body into the light step, per voxel, over the shape's clipped column box and slice range, after the point-light groups and before the voxel lights. The shell bodies SHALL read the voxel's world centimetres through the shape's frame; the line and multi-point bodies SHALL measure in the volume's units (the block's width is 1), positions scaled into them, attenuation unchanged. A shape SHALL carry up to 16 points.

#### Scenario: Cuboid in the block
- **WHEN** a cuboid shape lies inside the camera's block
- **THEN** the voxels of its clipped box take its colour falloff each step and the voxels outside it take none

#### Scenario: A blast's point in a 51.2 m block
- **WHEN** a multi-point shape of attenuation 5000 has a point at 512 cm in a block 51.2 m wide
- **THEN** the point is uploaded at 0.1 and lights voxels within √(1/5000) of the width, about 72 cm
