## ADDED Requirements

### Requirement: Outdoor sky
The system SHALL render an open sky over levels lit by the outdoor preset, scattering the level's sunlight, and no sky over other levels.

#### Scenario: Outdoor arena
- **WHEN** `exarust level arenao3` runs
- **THEN** a daylight sky appears above the horizon instead of a flat background
