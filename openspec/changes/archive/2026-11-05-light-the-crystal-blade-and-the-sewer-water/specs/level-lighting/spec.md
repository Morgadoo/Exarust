# Spec Delta

## MODIFIED Requirements

### Requirement: Frame environment values
Each frame the scene environment SHALL publish its water level: −25 cm when its terrain wetness is above 0.4 in double precision, −100000 otherwise. The terrain wetness SHALL be the palette's header float a terrain load hands it, and the save load's reset SHALL set it to zero. The frame SHALL also publish the current rain times 0.3 as the wind, and the tint colour times the tint's fourth slot.

#### Scenario: Thresholds
- **WHEN** the terrain wetness is 0.39, then the single-precision 0.4
- **THEN** the water level is −100000, then −25; a rain of 0.5 gives a wind of 0.15 and a tint of (0.5, 0.25, 0.75, 2) gives (1, 0.5, 1.5)

#### Scenario: The sewer
- **WHEN** `exanima05_sw` loads its `UWSwrA1` terrain (wetness 0.5)
- **THEN** the scene's water level is −25 cm
