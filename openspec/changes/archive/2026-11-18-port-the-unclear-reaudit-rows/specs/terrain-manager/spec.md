## ADDED Requirements

### Requirement: Composite pools follow the texture detail
The terrain's composite pools SHALL take their detail side from the game's texture-detail setting: 4096 below 3, 8192 from 3.

#### Scenario: A low texture detail
- **WHEN** the texture-detail setting is 2
- **THEN** the pools' detail side is 4096
