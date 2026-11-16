## ADDED Requirements

### Requirement: Post sharpen by supersample level
The final pass's sharpen SHALL follow the settings' supersample level on every post camera: 0 at level 0, otherwise 1.5 × (0.075·level + 0.4) in double precision, stored in single (0.9375 at the 4× level 3).

#### Scenario: Changing the level
- **WHEN** the level is set to 1, 3 and 0
- **THEN** the sharpen is 0.7125, 0.9375 and 0
