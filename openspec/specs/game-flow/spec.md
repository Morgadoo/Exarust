# game-flow Specification

## Purpose
The game's top-level flow: the mode switch, the session starters and hooks, death, level exits and their saves, the quit autosave, the story scripts' game events and the start-up splash.

## Requirements

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

### Requirement: Story start record
Before the level loads, a new story SHALL clear the player's role list and add roles 7, 1 and the background's own; the unknown background SHALL carry item 0x10c with the third word 0x0080_0020 and get the power word 0x510000 and value 4.0; the knight and proctor SHALL hold item 0x1f in the off hand and carry it with flags 0x100 and the default third word 0x0004_0028.

#### Scenario: The knight's gear
- **WHEN** a knight's story starts
- **THEN** its roles are 7, 1 and 0x1b, its off hand holds item 0x1f and its carried list gains item 0x1f with flags 0x100

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

### Requirement: Menu page rows
The title's CONTINUE SHALL be live only when the story's save manager has a save and the start page's only when the arena's has one; the in-game page's first row SHALL read NEW GAME in practice and CONTINUE otherwise; the arena pause page's last row SHALL read EXIT when the session word has its value-4 bit and FORFEIT otherwise.

#### Scenario: Only an arena save
- **WHEN** only the arena's manager has a save
- **THEN** the title's CONTINUE is greyed out and the start page's is live

### Requirement: Get-up writes
While a villager's get-up holds, the story hook SHALL write animator flags 0x600, blend 1 and move and turn 0.49; once 500 ms have passed it SHALL write flags 0x20, blend 0.4, turn 0.4 and the clip clock at the motion's value less 1490 ms. A revival SHALL set flag 0x40 and push the clip clock 2000 ms on.

#### Scenario: The get-up starts
- **WHEN** a get-up begun at 10 000 ms is ticked at 10 501 ms
- **THEN** flags 0x20, blend 0.4 and turn 0.4 are written and the get-up ends

### Requirement: Session bookkeeping
Leaving a session SHALL stop the hook and frame task, close the game view, the dialogue and the prelude, run a scripted mode's end hook, then close the save package and hide the end menu before the world goes; back to the title SHALL show the main menu's title logo and leave the GUI scale alone; a session's begin and continue SHALL clear the end menu's no-row-selected flag; a story continue SHALL clear only the first wake-up's animating flag.

#### Scenario: Back to the title
- **WHEN** a story returns to the title
- **THEN** the title logo shows and neither the help flag nor the GUI scale is written

### Requirement: Applied flow steps
The running game SHALL apply the flow's menu, end-menu, title-logo, game-view, controls, health, music and quit steps, and a level load's request SHALL hide the menu form and close the end menu while the loaded level stays. The session begin SHALL NOT hide the menu form itself.

#### Scenario: Death and exit
- **WHEN** a reconstructable story player dies, EXIT is confirmed, and 100 ms pass
- **THEN** the end menu shows CONTINUE and EXIT with the first row selected, the exit runs behind the transition, and the title page is shown in menu mode

#### Scenario: Begin
- **WHEN** a session begins around a starter that loads exanima01
- **THEN** the steps are the load, the session task and the end menu's selected flag, with no menu hide

### Requirement: Practice end-menu rows
In practice mode the end menu's first row SHALL leave to the menu form behind the transition on menu 2 and start the next fight on any other menu, and its second row SHALL return to the title behind the transition.

#### Scenario: Practice restart
- **WHEN** RESTART is confirmed on practice's menu 2 at 1000 ms and 100 ms pass
- **THEN** the prelude closes, the transition starts, and the session leaves to the menu form with its theme

### Requirement: Story and arena end-menu rows
In story mode the first row SHALL continue on the reconstructed menu and otherwise exit the story, as the second row does. In arena mode the first row SHALL run the arena's continue and the second return to the title behind the transition. Any other row or mode SHALL do nothing and keep the menu open.

#### Scenario: Story one-row menu
- **WHEN** the story's one-row EXIT menu's row is confirmed
- **THEN** the story exit runs

#### Scenario: Arena third row
- **WHEN** the arena end menu's third row is confirmed
- **THEN** nothing happens and the menu stays open

### Requirement: Launched level session
A level launched directly SHALL run as a practice-mode session without the practice round hook, and a title page the splash put up SHALL stay over it until closed.

#### Scenario: Corpses in a launched level
- **WHEN** `play exanima01` runs 900 frames
- **THEN** no end menu is shown

### Requirement: In-process level change
A level change SHALL despawn every level-owned root with its subtree, leave the physics world with no body or collider of the old level, drop the level's GI, navigation, room, wall, terrain-ground, sun-map, sky and fog state, and keep the player entity and the session state; the destination SHALL load through the same staged level load as a start-up load. A teardown step in the same batch as a load step SHALL make one change, and a placement step SHALL be carried across the load in metres.

#### Scenario: Teardown leaves nothing of the level
- **WHEN** a level root with a child, a placed character, a player holding a capsule handle and one physics body with a collider are present and a teardown is requested
- **THEN** no level entity or child remains, the player remains without its capsule handle, and the physics world holds 0 bodies and 0 colliders

#### Scenario: The player survives the load
- **WHEN** a running `exanima01` changes to `arenahub`, then `arena1`, then back to `exanima01`
- **THEN** each level holds one player, the physics world holds only that level's bodies and colliders, and `exanima01` holds the same counts on its return as on its first load within a few entities

#### Scenario: Load, teardown and placement in one batch
- **WHEN** the steps teardown, load `arenahub` at spawn 2 and place the player at (100, 200, 300) cm facing 90 degrees are applied
- **THEN** one change to `arenahub` at spawn 2 is requested and the player placement is (1, 2, 3) m with a yaw of pi/2

### Requirement: Practice next fight reload
The practice ladder's next fight SHALL reload its arena (`arena`, or `arenae` for the expert ladder) and run the practice scene filter right after the load, before the heal and the placement.

#### Scenario: Expert next fight
- **WHEN** the expert ladder's fight 7 advances
- **THEN** the steps load `arenae` with a teardown and the next step is the practice scene filter
