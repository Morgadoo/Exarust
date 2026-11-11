## ADDED Requirements

### Requirement: The step ticks standing bodies' clips
A body whose clips neither its body tick nor the frames advance SHALL tick its clip once at the end of each fixed step, at a standing body's pace times its race's base, and keep the sample as its pose. The frame SHALL place such a body's parts from the kept pose and SHALL NOT write its clip player.

#### Scenario: A story level in free mode
- **WHEN** exanima01 runs a story session in free mode with the bridge's write log on for Animator, and the player is felled at frame 250
- **THEN** the log counts no Animator among the frame values applied after a newer simulation write

### Requirement: Presentation figures keep the frame clock
The character creator's figure, the inventory's doll and the character view's figure SHALL advance their clips by the body clock counted from the frames' time, as they did before, and the step SHALL leave them alone.

#### Scenario: The step meets a presentation figure
- **WHEN** a fixed step runs while a presentation figure stands
- **THEN** the figure's clip clock does not move in that step

### Requirement: The player's lesson is taken in the step
The lesson that a struck character's reaction offers the player SHALL raise the player's practice lesson in the post-step of the step that dealt the blow, after the hit reactions. The frame SHALL NOT send the player's practice as a whole copy.

#### Scenario: Fist strikes in free mode
- **WHEN** the player strikes an opponent with its fists in an arena while the simulation runs free
- **THEN** the log counts no PlayerPractice among the frame values applied after a newer simulation write, and the practice pays

### Requirement: The arena-fell drop is posted
The `--arena-fell` debug tool SHALL post the opponents' death and empty health to the start of the next fixed step.

#### Scenario: Felled arena opponents
- **WHEN** an arena match runs with `--arena-fell 250` in free mode
- **THEN** the round is won and the log counts no Fighter among the frame values applied after a newer simulation write

### Requirement: Equal frame values are not counted as undoing the step
A frame value of data both sides write that is equal to the simulation's newer value SHALL NOT be counted among the frame values applied after a newer simulation write. With the write log on, the simulation side SHALL log what an applied value undid, with the entity and the two steps.

#### Scenario: A frame copy equal to the simulation's
- **WHEN** a frame value based on an older step arrives equal to the simulation's newer value
- **THEN** it is applied and not counted
