## ADDED Requirements

### Requirement: Fire particles step in the simulation
A fire's particle emitter SHALL step once a simulation step in the solver's object stage, after the bodies' bounds pass and before body–body detection, the emitters in a stable order, and SHALL meet the bodies in that step's object–body contacts.

#### Scenario: A medium fire under a sleeping body
- **WHEN** a 0.8 fire burns for 240 frames of 1/60 s under a body's part 35 cm above its node
- **THEN** its flame and its bed have each stepped 331 ticks and the part has taken whole doses of 1

### Requirement: Fire particles drawn from the step
The frame SHALL draw and light a fire from the last finished step's whole emitter, in lockstep and with the simulation free of the frame, and SHALL keep the light's peak, its full-update bit and the sound walk, with their random draws, as its own, never written back to the step.

#### Scenario: A torch with the simulation free of the frame
- **WHEN** a torch burns for 90 frames with the simulation on its own thread
- **THEN** the frame's emitter never goes back a tick and, once burning, never shows fewer than ten particles

### Requirement: Fire particle draw fraction
The frame SHALL draw a just-stepped emitter's particles at the original's render fraction, one plus the time since the step over the step (1 to 2), for both the position lerp and the age.

#### Scenario: Right after a step
- **WHEN** the frame draws with no time past the last step
- **THEN** each particle is drawn at its last tick's position

### Requirement: Fire body-contact bit
A fire's flame SHALL meet the bodies; its bed and a level portal's sparks SHALL not.

#### Scenario: A medium fire's bed
- **WHEN** a 0.8 fire is built
- **THEN** its flame has the body-contact bit and its bed has not

### Requirement: Fire pre-roll on both copies
The held load's settle ticks SHALL reach the simulation's copy of a fire as well as the frame's, so the step goes on from the settled flame once the hold ends.

#### Scenario: A held free load
- **WHEN** a torch settles while the simulation is held and the hold then ends
- **THEN** the frame never sees the flame's tick count below the settle count again

## MODIFIED Requirements

### Requirement: Carried fire
The system SHALL carry a held torch's fire with the hand as an emitter attached to the item's body: each tick its live particles move by half the hand's displacement and new particles inherit 0.3 of it. In the simulation the emitter's frame SHALL be its part's pose at the solver's object stage, tick by tick, and its body pass SHALL skip that part.

#### Scenario: Walking with a torch
- **WHEN** the player walks carrying a lit torch
- **THEN** the flame follows the hand with a short trail instead of being left behind

#### Scenario: A falling carried flame
- **WHEN** a body carrying a flame falls freely for half a second
- **THEN** the emitter's node is within a centimetre of the part's pose and the part takes no dose
