## ADDED Requirements

### Requirement: The bar's casts run in the step
Behind the simulation bridge, a power bar Use SHALL run its cast entry, the domain's front end, the started power's class and a Mind cast's writes to both bodies' minds on the simulation's world, posted to the start of its next step, with the picks the frame read when the key was pressed. The frame SHALL NOT send its whole copy of the caster or of a mind for a cast.

#### Scenario: Three Mind casts in free mode
- **WHEN** the player casts Mask Mind, Calm and Enrage at a still opponent while the simulation runs free
- **THEN** the log counts no PowerCaster or MindBody among the frame values applied after a newer simulation write, and each cast logs its result and its mind sound

### Requirement: The bar's keys write the caster through the step queue
The bar's toggle, pick and step SHALL write the frame's copy of the caster at once, without a change mark, and queue the same write for the simulation's copy at the start of its next step.

#### Scenario: A scripted pick
- **WHEN** a debug cast opens the bar, picks a slot and closes it in one frame
- **THEN** the frame's bar shows the pick at once and the simulation's caster has it before the posted cast runs

### Requirement: The bar's results reach the frame after the step
The GUI's fail sound, the mind sounds and music, the spirit body's request and the end of a control that a posted bar cast asks for SHALL be sent back to the frame and run there after the step.

#### Scenario: A Mind cast's sound
- **WHEN** a posted Mind cast plays its sound
- **THEN** the frame logs the mind sound and plays it

### Requirement: Flame rows keep their frame state apart
A flame row's loop pitch SHALL be frame-only state. The frame's draw from the row's one random stream SHALL be made on its copy and the same draw queued for the simulation's copy. A faded row SHALL leave the scene by the frame's despawn alone. The frame SHALL NOT send a row as a whole copy.

#### Scenario: A burning blade in free mode
- **WHEN** the player holds a burning blade and casts Shock while the simulation runs free
- **THEN** the log counts no Flame among the frame values applied after a newer simulation write

### Requirement: Arcs keep their frame state apart
The arcs' light and hum flickers and their light points SHALL be frame-only state. The draw's write of each drawn tendril's first point to its origin SHALL be queued for the simulation's copy. The item abilities' end of a mace's arcs and their extinguish of a flame row SHALL go to the simulation's copy at the start of its next step.

#### Scenario: Shock and Shock Weapons in free mode
- **WHEN** the player casts Shock and Shock Weapons while the simulation runs free
- **THEN** the log counts no LiveArcs among the frame values applied after a newer simulation write, and lockstep traces match the build before the change

### Requirement: Small frame writes to simulation data go through the step queue
The fight start's engage of the duellists and the player's combat mode, the combat aim, the outfit's armour layers, the stay check's clear of the trailing-foot latch and the held moveset choice's switch of the animator's config SHALL write the frame's copy at once, without a change mark, and queue the same write for the simulation's copy at the start of its next step.

#### Scenario: The feature sweep in free mode
- **WHEN** the feature sweep runs free with the bridge's write log on
- **THEN** no Locomotor, Duellist, combat aim, armour or moveset config write is counted among the frame values applied after a newer simulation write

### Requirement: One stay check per lapsed request
A lapsed trailing-foot request SHALL run the stay check once, though a free-running simulation's steps computed before the latch's clear landed still carry the raised latch.

#### Scenario: The player stops in a stay zone
- **WHEN** the player's trailing foot plants in a stay zone while the simulation runs free
- **THEN** the zone's routine runs once for that request
