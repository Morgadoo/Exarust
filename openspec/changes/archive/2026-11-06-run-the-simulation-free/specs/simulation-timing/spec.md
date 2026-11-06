## ADDED Requirements

### Requirement: Simulation-only data stays in the simulation
Data only the simulation's systems touch SHALL move to the simulation's world once and SHALL not be handed back to the frame between steps; a frame that inserts it again (a level load) SHALL replace the simulation's, and one that removes it SHALL have it removed there before its other changes apply.

#### Scenario: A level change
- **WHEN** the frame removes the moving meshes' record and later inserts a new one
- **THEN** the simulation drops its record before the frame's other changes of that packet and takes the new one, with no copy back to the frame

### Requirement: Free-running simulation
With the free mode on, the simulation SHALL step on its own 12 ms real clock, dropping its backlog when more than four steps late, SHALL not step while the frame holds it (the loading screen, a menu, the splash, the character creator, the end menu, a paused game clock) and SHALL leave the held time out of its clock.

#### Scenario: Held and released
- **WHEN** the frame holds the simulation for a while and then releases it
- **THEN** no step runs while held, steps follow at about 83 a second after the release with consecutive ticks, and the published time does not count the held time

### Requirement: Packets between the free simulation and the frame
With the free mode on, the simulation SHALL take every packet the frame sent since its last step, in order, before each step, and SHALL send after each step a packet stamped with the step's tick and time; the frame SHALL not wait for a step and SHALL apply every step packet that arrived, in order.

#### Scenario: The frame's calls
- **WHEN** the frame posts calls over several frames
- **THEN** the simulation runs them in post order before its next steps, and their answers reach the frame in the same order

### Requirement: Drawing at the original's render fraction
With the free mode on, the frame SHALL draw the simulated bodies at 1 + (frame clock − the last step's published time) / step, at most 2 and at least −2, as the moment one step behind the clock: 1 the step before the last, 2 the last, a fraction below 1 held at the step before the last; and SHALL keep the fraction while the simulation is held.

#### Scenario: A frame half a step after the last step
- **WHEN** a frame is drawn 6 ms after the last published step's time
- **THEN** the bodies are drawn at fraction 1.5, halfway between the last two steps

### Requirement: Drawn copies keep the simulation's newer value
With the free mode on, a value the frame writes only to draw it (transforms, the drawn controller) SHALL not replace a value the simulation wrote in a step the frame had not seen when it wrote; other values both sides write SHALL take the frame's write, as in lockstep. In lockstep every step SHALL stay identical to the frame-driven step.

#### Scenario: The drawn controller
- **WHEN** the frame draws a character's controller and the simulation has stepped it since
- **THEN** the simulation keeps its own controller and position
