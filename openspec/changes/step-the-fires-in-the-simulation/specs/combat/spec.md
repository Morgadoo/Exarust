## ADDED Requirements

### Requirement: Fires burn the characters in them
A fire's flame particles that meet a character's part SHALL add their doses to the character's burn in the simulation step, behind the simulation's bridge as in one world.

#### Scenario: Standing in a medium fire
- **WHEN** a character's lumbar part rests 35 cm above a 0.8 fire's node for 240 frames in lockstep behind the bridge
- **THEN** the frame's copy of its burn strength has grown by whole doses and never went down
