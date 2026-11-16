## ADDED Requirements

### Requirement: Checked engine routines at their callers
A port whose original calls one of the engine's checked routines (the fast length or distance, a point routine, the relative frame, the rigid inverse, the weighted push or the in-place negation) SHALL take that routine's engine form rather than glam's equivalent.

#### Scenario: The door press divides by the fast length
- **WHEN** a hinged leaf is pressed with its hit 3 m from the node
- **THEN** the grip point is 1 m from the origin to within the fast length's error, not glam's exact length

### Requirement: The original's negation where it negates in place
A port SHALL negate with `0 − v` where the original calls its negation routine, so a zero component comes out as positive zero.

#### Scenario: A zero push
- **WHEN** a placed pair's correction has a zero component
- **THEN** the negated correction written to the other array holds positive zero there
