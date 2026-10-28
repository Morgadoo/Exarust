## ADDED Requirements

### Requirement: Unordered focus checks pass
Every Force and Displacement focus, facing and distance check SHALL refuse only on its ordered side, so a not-a-number SHALL pass it. The body gate's pace check SHALL send a not-a-number pace to the "not now" answer, and the displacement gates SHALL treat a not-a-number clip rate as not below one.

#### Scenario: A not-a-number focus
- **WHEN** a caster whose focus is not a number casts a wave, and the AI weighs a blast with focus, facing and distance not numbers
- **THEN** the wave starts and the blast advice is 1
- **AND** a not-a-number pace gives "not now" and a collapsing body with a not-a-number clip rate fails the displacement gate

### Requirement: Creator sound pitch in double
The shell and strikes start sound SHALL take its pitch as the draw widened to double times 0.4 plus 0.6, narrowed once; the barricade and throw pick sounds 0.4 and 0.8, the throw's lift 0.2 and 1.

#### Scenario: A rounding case
- **WHEN** the draw is 0.029
- **THEN** the start pitch is 0.61159998 and the barricade's 0.81159997

### Requirement: Effect material release callbacks
The force, barrier and aura materials SHALL register one release callback between them, and the tendril and shell materials another, each once. A callback SHALL free its unit's program handles (twelve, plus the GI table, for the Force unit; two for the Displacement unit) once, and SHALL NOT free the material shaders. A failed GI entry SHALL free nothing at shutdown.

#### Scenario: Releasing both units
- **WHEN** the tendril, shell, force and barrier shaders are made and every callback runs, then runs again
- **THEN** the first runs free 2 and 12 handles and the second free none, and the four shaders stay made
- **AND** a GI table with one made and one failed entry frees 12 + 2 handles

### Requirement: Active power creator records
The long door's creator SHALL hold the tick drain `tick ms × cost × 0.001` in single precision, the destination and the caster's forward axis (none on the AI cast); the constant displacement's and the shell's creators SHALL hold their cost; the summoning's SHALL hold none and add no node.

#### Scenario: Long door and constant
- **WHEN** a player's long door is created at cost 0.2 with 12 ms ticks, and a constant displacement at cost 0.35
- **THEN** the door's drain is 12 × 0.2 × 0.001 and its vector the caster's heading
- **AND** the constant holds 0.35

### Requirement: Constant displacement image end
The constant displacement's restore tick SHALL end its tendril image when a node's collisions return: every strand record marked 0x8, the image's fade zero and its node told 0x10. Its start SHALL ask for an unshared shell and play the hum silent at pitch 1.

#### Scenario: A blow passes and returns
- **WHEN** a passing blow phases a node at 1000 ms and the restore tick runs at 1501 ms
- **THEN** the node's collisions return and the image has ended with event 0x10
