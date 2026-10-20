## ADDED Requirements

### Requirement: Far head tracking
The far head variant SHALL use a 150 cm look-ahead and no pitch floor, keeping the same look bands, π/100 slew and ±π/4 yaw clamp.

#### Scenario: A far point and a steep look
- **WHEN** the target lies straight ahead a kilometre away
- **THEN** the published point is 150 cm along the direction
- **WHEN** the target lies far above and the pitch starts at −1.02
- **THEN** the pitch stays below −1, unfloored
