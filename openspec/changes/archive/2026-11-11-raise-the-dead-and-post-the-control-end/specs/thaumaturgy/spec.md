## ADDED Requirements

### Requirement: Mind cast reads the target body
Before a Mind cast from the bar, the target's mind SHALL take the target body's condition word, its rules, its feral level as the decay and its yellow and red health, as the original's casts read them off the body.

#### Scenario: A dead target
- **WHEN** Raise Dead is cast at a dead body whose mind still holds a condition of 0
- **THEN** the cast reads the dead condition and starts (code 1)

### Requirement: Raise Dead raises the corpse
A started Raise Dead or Command Dead SHALL raise its target on the world that steps the bodies, at the start of the next step: a revival when asked (feral level 0.2 on the record and the character, the decayed rules bit, most energy `1 − decay/10`), then the revive the AI manager's rise runs (knocked out at once, muscled again, its death clock gone), then yellow 0.5 and red 1 over the revive's yellow. A body no longer dead SHALL not rise.

#### Scenario: Raised and up
- **WHEN** Raise Dead starts on a dead body without decay
- **THEN** the body is down with its muscles back, yellow 0.5 and red 1, feral 0.2, and wakes from the knock-out

### Requirement: Command Dead lays the body down
The end of a Command Dead caster SHALL start a collapse on its raised body, as a lethal wound starts one, so that the body dies again; a body collapsing or dead already, or without a clip player, SHALL be left as it is.

#### Scenario: Second cast
- **WHEN** Command Dead is cast again on the standing body the caster commands
- **THEN** the command ends and the body collapses and dies

### Requirement: Control end reaches the power walk
A cast while the player is in another body SHALL end the held body's Control Mind, or else the Spirit Form of the spirit that left it, by a command run on the world that walks the powers at its next step, not by a write on the frame's copy of the power list.

#### Scenario: Spirit Form in free mode
- **WHEN** the player casts from the spirit while the simulation runs free
- **THEN** the Spirit Form is told to end, ends and the player is home

### Requirement: Arc voxel light owner
An arc emitter's voxel light SHALL name its emitter without a child link, and SHALL be despawned when its emitter is gone.

#### Scenario: Emitter despawned
- **WHEN** an arc emitter holding a voxel light is despawned
- **THEN** its voxel light is despawned on the next frame
