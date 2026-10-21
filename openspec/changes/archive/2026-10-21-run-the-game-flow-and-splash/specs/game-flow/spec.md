## ADDED Requirements

### Requirement: Mode switch
A switch to another game mode SHALL clear the reconstruction state, store the outgoing mode's player, and set the hook: the story's for story, none for the arena, the title and a scripted mode (whose enter hook runs); practice SHALL keep its hook, and entering it from outside the practice family SHALL restart the round clock.

#### Scenario: Into practice from the title
- **WHEN** the title switches to practice at 1000 ms
- **THEN** no hook is installed and the round clock reads 1000

### Requirement: Camera shots
A point shot SHALL put the camera at the point plus (700, 1180, 1100) cm, or (300, 1180, 1300) for the far shot, and warp the cursor to the client centre plus (120, −200) times the height over 1140, each rounded half to even; the story start SHALL put the camera 600, 600, 1300 cm off the player.

#### Scenario: The practice ring
- **WHEN** the ring's middle (0, 0, 500) is framed with the near shot in a 1600 × 1140 client
- **THEN** the camera stands at (700, 1180, 1600) and the cursor at (920, 370)

### Requirement: Story start
A new story SHALL start at `exanima01` with its background's role and flags, a starting light for the unknown background (turned −48°, 110°, 88°) and the villager (8°, −150°, 50°) and none for the knight and proctor, and spawn turns built as the engine's rows `(cz·cy, sz, −cz·sy)`, `(sx·sy − cx·sz·cy, cx·cz, sx·cy + cx·sz·sy)`, `(cx·sy + sx·sz·cy, −sx·cz, cx·cy − sx·sz·sy)`.

#### Scenario: A quarter turn
- **WHEN** the spawn turn of yaw 90° is built
- **THEN** its rows are (0, 0, −1), (0, 1, 0) and (1, 0, 0)

### Requirement: Level-exit save
Every level exit to a level the locale table names SHALL save without the death rule behind the transition and load the level at the exit's spawn point; an exit below level 7 SHALL also keep a checkpoint when the checkpoint setting is on (level 6 always) and the save holds neither `exanima04` nor the level.

#### Scenario: Past level 6
- **WHEN** a player leaves for level 7
- **THEN** the session is saved and no checkpoint is kept

### Requirement: Exit-trigger nudge
A continued story's player SHALL be pushed out of an exit trigger whose centre lies within (radius + 50)² by the squared three-dimensional distance, whose box lies within 50 by the squared overhang and whose object is an exit marker: along the trigger row of the axis with the smaller clearance by `(extent − local) + 75`, negated when the local offset is not positive.

#### Scenario: Eight centimetres past the box
- **WHEN** a player stands 7 cm past an exit trigger's box, then 8 cm past it
- **THEN** it is pushed out the first time and left alone the second

### Requirement: Practice round and death
The practice hook SHALL act only in the playing phase once a second has passed since the fight began: a dead player dies, otherwise the first dead opponent ends the round, and the eighth fight wins the ladder.

#### Scenario: A death inside the guard second
- **WHEN** the player is dead 999 ms after the fight began
- **THEN** nothing happens until the second has passed

### Requirement: Quit autosave
The application quit SHALL, in the paused phase, run the arena unit's pause callback; while playing it SHALL save a story with the death rule, save an arena from its in-game page or forfeit from its pause page, and write nothing for practice; it SHALL NOT return to the title.

#### Scenario: A stripped story player quits
- **WHEN** a story whose player was stripped quits the application
- **THEN** the save is written with the death rule

### Requirement: Story game events
The third domain-awakening code SHALL wake the mind domain, write its class bits and wake the force domain, after the swing sound; the second SHALL wake the mind domain before its class bits.

#### Scenario: The last awakening code
- **WHEN** the third awakening code arrives on the unknown background
- **THEN** the effects end with the swing, the mind domain, the class bits 0x8A0000 and the force domain

### Requirement: Splash screen
The splash SHALL draw a black backdrop each tick at its alpha, fade the first logo's grey level in from black over 600 ms after a one-second warm-up, show the second logo with its sound, fade it to black over 300 ms, hand over to the main menu, wait a second and fade the backdrop's alpha out over a second; its elapsed times SHALL be the signed difference of the frame time and the stamp clock, its fades computed in double and kept in single, rounded half to even to a byte.

#### Scenario: A tenth of the first fade
- **WHEN** 100 ms of the first logo's fade have passed
- **THEN** the logo is drawn at grey 42 and the backdrop stays opaque

### Requirement: Splash at start-up
`exarust --splash` SHALL show the splash at start-up, centred logos at `round(height / 135) · 0.125`, and open the main menu at the hand-over; without the flag nothing SHALL start it.

#### Scenario: A 1080-pixel client
- **WHEN** a 512 × 128 logo is drawn in a 1600 × 1080 client
- **THEN** it covers 544, 476 to 1056, 604
