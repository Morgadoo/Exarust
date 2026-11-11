## ADDED Requirements

### Requirement: Parked dormant bodies
The body tick SHALL park the body of a dormant character (a lying sleeper or a resting statue) that the renderer did not draw in the last frame, unless it is parked already; a body never noted counts as drawn.

#### Scenario: A statue out of view
- **WHEN** none of a resting statue's spheres was inside a world camera's frustum in the last frame
- **THEN** its body is parked in the next tick and stays where it lies, with no velocity

### Requirement: Parked body tick
A parked body SHALL be left out of the solver: it is not integrated and no pair, link or contact stage meets it. Each tick it SHALL roll its particle sets (the newest becomes the previous one), so it keeps no velocity, and take one off its countdown while that is at least 1; once the countdown is under 1 nothing more is done to it. A wake SHALL NOT unpark it.

#### Scenario: A parked body is woken
- **WHEN** a parked body takes a wake to 256
- **THEN** its countdown rises but it stays out of the solver

### Requirement: Parked bodies come back when drawn
Before each step, the AI manager's frame SHALL unpark every parked body of a character in the scene that the renderer drew in the last frame, before its rise; a character out of the scene stays parked. The golem wake SHALL unpark the golem it wakes after its wake. An unparked body SHALL be simulated again from the next tick by its own sleep state, from rest.

#### Scenario: The camera turns to a parked statue
- **WHEN** a parked statue is drawn again
- **THEN** the next step unparks it and it is simulated again

#### Scenario: The alarm wakes a parked golem
- **WHEN** the alarm wakes a statue golem that is parked out of view
- **THEN** its body is woken to 256 and unparked in the same callback
