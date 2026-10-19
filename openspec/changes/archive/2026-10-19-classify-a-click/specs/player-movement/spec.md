# Spec Delta

## ADDED Requirements

### Requirement: Cursor pick
A click SHALL name a free hinge as a door, a dead character as a corpse and a living one as a person. A loose prop lighter than 0.08 SHALL be ignored, and a heavy prop SHALL be ignored when strength is under 0.9. A usable item on a prop SHALL stay usable, and a usable item with no body SHALL be nothing. A point farther than 70 cm SHALL be turned toward first. A point SHALL be within reach at a height of 245 cm and 300 cm away, and a door at 300 cm high and 350 cm away.

#### Scenario: A door, a light prop, and the reach
- **WHEN** a free hinge, a blocked hinge, a prop of mass 0.07, a prop of mass 0.08, a usable prop, that usable prop with its body removed, a point 70 cm away and a point 70.1 cm away are read, and the reach is tried at 245 cm high and 300 cm away, then 0.1 cm farther, then as a door at 350 cm
- **THEN** the names are a door, nothing, nothing, a loose prop, usable, nothing; 70 cm does not turn and 70.1 cm does; the first reach holds, the farther one does not, and the door does
