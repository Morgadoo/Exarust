## ADDED Requirements

### Requirement: Torch light reach
The system SHALL widen a small fire's light radius by 1.25 when its size, compared in double precision, is above 0.2 (up to 0.35), as the original does, so a torch (size 0.2 stored in 32 bits) reaches 838.5 cm.

#### Scenario: Wall torch
- **WHEN** a torch of size 0.2 burns
- **THEN** its light's radius is 838.5 cm
