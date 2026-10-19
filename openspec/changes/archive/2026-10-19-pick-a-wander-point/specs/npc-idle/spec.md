# Spec Delta

## ADDED Requirements

### Requirement: Wander point
A walker with no room SHALL not pick. A pick SHALL be one exit, drawn by the roll, and a second draw only when that exit leads back to the room the walker came from. The goal SHALL be a quarter of the way from the exit toward the room point, at height 0, with a stop of 20 cm and a radius of 150 cm, ignoring height. A second draw of the same exit SHALL still be kept.

#### Scenario: A quarter of the way, unless the exit leads back
- **WHEN** a walker with no room is asked to pick, a walker draws the only exit from the origin toward a room point 100 cm ahead, a walker draws an exit 100 cm deep toward a room point at the origin, a walker whose first exit leads back draws the next exit 200 cm ahead, and a walker whose only exit leads back draws again
- **THEN** the walker with no room picks nothing and the roll stays, the ahead goal is 25 cm forward at height 0 with stop 20 cm and radius 150 cm after one step of the roll, the deep goal is 75 cm deep at height 0, the retry uses the second exit at 150 cm and steps the roll twice, and the only exit is kept after two steps
