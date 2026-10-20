## ADDED Requirements

### Requirement: Terrain noise maps
The system SHALL build the three 480×480 terrain noise maps as the original's terrain renderer does: value noise of four octaves from the seeds 137, 476 and 7359, with the first map folded into its ridge once after its octaves.

#### Scenario: The first map
- **WHEN** the noise maps are built
- **THEN** the first map starts 137, 128, 117, 131 and its bytes sum to 40 696 892
- **AND** the second map starts 156, 161, 159, 167
