# Spec Delta

## ADDED Requirements

### Requirement: Hinged door
A hinged door's creak SHALL smooth the handle velocity by half and stop when the squared speed is at most 0.015. Otherwise the volume SHALL be 0.075 times the square root of the speed and the pitch SHALL be 0.022 times the speed plus 0.7, at most 1. A shut door's pull count SHALL rise by 1 up to 20 and then fall by 4 while it is held, not latched, and short of closed, and it SHALL unlatch when the count goes below 0. Opening SHALL shove the handle 3 cm along the door's facing.

#### Scenario: Creak and the seventh pull
- **WHEN** a still hinge is given a handle velocity of 1 along X, and a shut unlatched door whose count is 20 is pulled for seven ticks, and the door is opened along +Z
- **THEN** the creak pitch is 0.711, the counts after the first six pulls are 16, 13, 10, 7, 4 and 1, the seventh pull unlatches, and the shove is 3 cm along +Z
