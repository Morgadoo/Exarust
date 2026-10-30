## ADDED Requirements

### Requirement: Held bodies float in the held environment
A body a Kinesis picks or a Throw grabs SHALL take the held environment (no gravity; per-second retentions 0.125 above 40 cm/s, 0.0125 above 20, 0.025 at rest) in place of its own until the force lets it go, under a force shell of colour 2.8 and fade-plane scale 0.5.

#### Scenario: A kinesis pick and its end
- **WHEN** a running kinesis takes a prop and then ends
- **THEN** the prop is held in the held environment under a shell of colour 2.8 while it runs, and has its own environment back with its shell closing after

#### Scenario: A throw from a kinesis
- **WHEN** a throw takes the body a kinesis holds
- **THEN** the throw holds the kinesis's shell and the body stays in the held environment

### Requirement: Held bodies pass from kinesis to throw
A throw from a kinesis SHALL take over the kinesis's shell and held environment; a throw SHALL close the shell when it drops and give the environment back 200 ms later, or both when it ends first.

#### Scenario: The throw's drop
- **WHEN** a throw's shell drops 800 ms after its lift
- **THEN** the shell closes and the body leaves the held environment 200 ms later

### Requirement: Strikes arm the held weapons
While Force Strikes runs, each item its caster holds SHALL carry an aura (at most four) of colour 2.8, fade-plane scale 1, impact gain 1.5 and pitch 0.8 and ripple scale 3, drawn with the strikes program; an item put away SHALL lose its aura, and the power's end SHALL close every aura.

#### Scenario: A weapon put away
- **WHEN** a strikes caster lets go of its weapon
- **THEN** the weapon's aura closes and leaves the power's list
