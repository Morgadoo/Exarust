# Spec Delta

## MODIFIED Requirements

### Requirement: Voxel light volume
The system SHALL build a level's voxel light volume as the original does. Each mesh's GI sample points mark a block of 20 cm voxels below the level's highest point. Every frame, light propagates through the open voxels from their eight diagonal neighbours, the level's base ambient, lights and voxel lights are injected, and the result is stored as 8-bit sRGB. Every write into the volume SHALL be encoded as the reference hardware (NVIDIA RTX 3090, driver 615.78) encodes a fragment into a `GL_SRGB8_ALPHA8` target: a linear value stores the number of the measured steps at or below it, not the nearest code.

#### Scenario: Block height
- **WHEN** a level is taller than 12.8 m
- **THEN** its block is 256 × 256 × 128 voxels instead of 256 × 256 × 64

#### Scenario: The foot of the curve
- **WHEN** a voxel's new light is 0.000125 (0.41 of the first code)
- **THEN** it stores code 1, where nearest rounding stores 0

#### Scenario: A code's own value
- **WHEN** a write's linear value is exactly an sRGB code's
- **THEN** it stores that code

## ADDED Requirements

### Requirement: GI debug split
With `EXA_GI_DEBUG=direct` lit world and terrain surfaces SHALL leave out their GI ambient and reflection, and with `EXA_GI_DEBUG=ambient` their direct lights, so the two frames add up to the full one in linear light; any other value SHALL show the raw GI sample.

#### Scenario: Direct share
- **WHEN** a level runs with `EXA_GI_DEBUG=direct`
- **THEN** a surface no light reaches directly is black
