# Spec Delta

## MODIFIED Requirements

### Requirement: GI cube shadows
The system SHALL shadow each point light's GI injection by its cached cube shadow, comparing the distance (in block-width units) times ((bias + 0.94)/far) × 5120, formed in double precision, plus 0.01 along the direction from the light to the voxel; far is the light's radius in centimetres when its range is on, else 3.4e38. It SHALL place the light at its position over the block's width on each axis with the height over the width.

#### Scenario: Shipped detail
- **WHEN** a ranged light of radius 2 m with no quality offset is injected
- **THEN** its cube depth scale is ((0.025 + 0.94)/200) × 5120 = 24.704

#### Scenario: Light off the block's middle
- **WHEN** a light stands 6 m along world Z from a 51.2 m block's corner
- **THEN** its depth coordinate is 6 / 51.2

## ADDED Requirements

### Requirement: GI cube shadow bias
The system SHALL take a light's GI cube bias as the shadow detail 5 plus the light's quality offset, capped at 6, times 0.005 in double precision, or 0.025 when that sum is above 5.

#### Scenario: Lower offset
- **WHEN** a light's quality offset is −1
- **THEN** its bias is 0.02

### Requirement: GI slice-centre sampling
The system SHALL read the GI light and occupancy textures for voxel layer L at the slice centre (L + 0.5)/depth and SHALL measure light distances with the height (slice + 0.5)/width.

#### Scenario: Bottom layer
- **WHEN** layer 0 of a 64-slice block is propagated
- **THEN** it samples r = 0.5/64

### Requirement: GI surface sample offset
The system SHALL sample the GI volume for a surface half a voxel above its height, and for terrain and grass at the voxel's bottom, with the per-object offset's X and Z ((grid/2 − block cell) − 4) × 30/256 + 0.5 and Y (below + 0.5)/depth, or below/depth when the half voxel is dropped.

#### Scenario: Synthetic block
- **WHEN** the synthetic grid's block at cells (3, 3) is 64 slices deep with 34 below the world's zero
- **THEN** its offset is (0.1484375, 0.5390625, 0.1484375), and its Y is 0.53125 for terrain
