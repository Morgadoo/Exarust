## ADDED Requirements

### Requirement: Story level fog variants
A story level's load SHALL apply, after the indoor preset, the fog distance the story's level-enter hook gives its name: 1.4 for exanima04, 2.0 for exanima06 to exanima08, 1.8 for every other story level. Other indoor levels SHALL keep 2.0.

#### Scenario: Loading the flooded level
- **WHEN** exanima04 loads
- **THEN** its scene environment's fog distance is 1.4
