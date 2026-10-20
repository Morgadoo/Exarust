## ADDED Requirements

### Requirement: Head look direction
The head tracking SHALL take the look-at minus the eye, reuse the previous unit direction when the squared xz distance is under 2,500, and in the band under 10,000 replace the vertical component with (10000 − d²)·1e-4 before normalizing.

#### Scenario: Distant and close targets
- **WHEN** the target is far away and off to the right
- **THEN** the direction comes from the normalized difference
- **WHEN** the target is under 50 cm away
- **THEN** the previous direction is used unchanged

### Requirement: Head angles
Yaw SHALL chase the approximate xz atan2 by at most π/100 a tick and stay within ±π/4 of the body's facing. Pitch SHALL chase the floored negative asin by at most π/100 a tick, with a −0.6 floor.

#### Scenario: Slew and body clamp
- **WHEN** the target is 45° to the right
- **THEN** the yaw moves one π/100 step
- **WHEN** the yaw would pass 45° from the body's facing
- **THEN** it is held at π/4

### Requirement: Head look result
The result SHALL be the unit direction (cos p·sin y, −sin p, cos p·cos y), the point 250 cm along it, and the flat unit direction.

#### Scenario: A step ahead
- **WHEN** the yaw is one step and the pitch is zero
- **THEN** the point is 250 cm along that direction and the flat direction keeps y at 0
