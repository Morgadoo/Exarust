## ADDED Requirements

### Requirement: The AI steps on its feet
A fighter whose body has muscles SHALL take its combat footwork through its body's step fields and SHALL read its body's step (stepping, kind, age, landings since its start, queue) as the combat state's. Its character SHALL NOT be given a walk velocity for the step, and the fighter SHALL keep its stance clip while it steps.

#### Scenario: A step in the arena
- **WHEN** the arena opponent chooses a place one step away
- **THEN** its body's step starts toward that place, its character has no walk velocity and its feet walk it

### Requirement: The AI's step writes
A new step SHALL aim the body's step target at the place and start the step (stepping on, no balance step); a long step's second half SHALL re-aim the target and make the step long without a new start; a step chosen while one runs SHALL queue the place's offset on the body; a place of no step SHALL clear the body's queue; a queue the body took SHALL go back to 0 on the next combat tick.

#### Scenario: A long step's second half
- **WHEN** a long step is 200 ms old and the place is the step it is in
- **THEN** the body's step target moves to the second half's goal and the step is long, with no new start

#### Scenario: A body without muscles
- **WHEN** the fighter has no muscled body
- **THEN** it glides its interim step as before
