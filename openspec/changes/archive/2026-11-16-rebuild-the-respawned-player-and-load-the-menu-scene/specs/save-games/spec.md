## ADDED Requirements

### Requirement: Respawn rebuild
The death screen's CONTINUE SHALL reset the player record's identity (its race byte cleared) and post its rebuild request with no AI start into the world scene. The respawn SHALL, after its save, make the next player spawn from the record that save wrote as the player's and remove the surviving player root.

#### Scenario: A stripped player continues
- **WHEN** a stripped player's CONTINUE and respawn run with a current story slot
- **THEN** the save holds the stripped record as the player, the old player root is gone and the next spawn's record holds no gear while the saved level keeps the corpse

### Requirement: Menu scene
The title after the splash and every return to the title SHALL load `menuscene` with no player before the menu form shows, with the view at (0, 5, 7) m looking at (0, 0, −2) m and a 60° field, and a surviving player's body SHALL stay out of the scene until a level of play is loaded.

#### Scenario: Back to the title
- **WHEN** a story session quits to the title
- **THEN** the menu scene's load is asked for after the world's teardown, the player's body is out of the scene and a new level's load ends the menu view

### Requirement: Transition save
A level exit that keeps a checkpoint SHALL save and copy the save over the checkpoint, overwriting an existing one, only when the current slot's save holds neither an `exanima04` section nor one for the destination; otherwise it SHALL save plainly. The sixth level's clock SHALL be kept only when the copy runs.

#### Scenario: A level entered again
- **WHEN** the exit's destination already has a section in the save
- **THEN** the save is written and the checkpoint is left as it was

### Requirement: Exit level availability
A story exit zone or journey SHALL do nothing when its level is neither in the current slot's save nor in the game archive.

#### Scenario: A missing level
- **WHEN** an exit names a level the archive lacks and the save does not hold
- **THEN** no step is queued
