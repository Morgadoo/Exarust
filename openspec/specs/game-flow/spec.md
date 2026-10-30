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
- **WHEN** a reconstructable story player dies, EXIT is confirmed, and the transition screen reaches black
- **THEN** the end menu shows CONTINUE and EXIT with the first row selected, the exit runs behind the transition, and the title page is shown in menu mode

#### Scenario: Begin
- **WHEN** a session begins around a starter that loads exanima01
- **THEN** the steps are the load, the session task and the end menu's selected flag, with no menu hide

### Requirement: Practice end-menu rows
In practice mode the end menu's first row SHALL leave to the menu form behind the transition on menu 2 and start the next fight on any other menu, and its second row SHALL return to the title behind the transition.

#### Scenario: Practice restart
- **WHEN** RESTART is confirmed on practice's menu 2 and the transition screen reaches black
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

### Requirement: Transition screen
Every flow transition SHALL open the transition screen with its action and run the action once the screen passes black: a 600 ms fade, or 2000 ms with a 1000 ms hold when the level exit's fade bit set the long hold before the open; then two frames, a 500 ms hold, a 600 ms fade back in and the close. The screen SHALL run every frame on the real clock whatever covers the game, and an action that changes the level SHALL keep it black until the change is done.

#### Scenario: Action at black
- **WHEN** a revival's transition starts at 1000 ms on the real clock
- **THEN** the alpha is 128 at 1300 ms, 255 with no action at 1600 ms, the revival runs at 1601 ms, the screen stays black to 2130 ms, is at 128 at 2431 ms and closes at 2731 ms

#### Scenario: Long hold from the fade bit
- **WHEN** a level exit with the fade bit opens the screen at 0 ms
- **THEN** the alpha is 128 at 1000 ms, nothing loads at 2000 ms and the level loads at 2001 ms

#### Scenario: Held during the load
- **WHEN** a named-locale exit reaches black at 601 ms and its level loads until 5000 ms
- **THEN** the screen stays at 255 through the load and the 500 ms hold after it, and is at 128 300 ms into the fade in

### Requirement: Transition music fade
With the caller's music fade on, the main track's gain SHALL fall as (1 - t) times its gain, stop at black with the audio fade at 0, and the audio fade SHALL rise with the fade in to 1 at the close; the world fade SHALL reset to 1 at black. The quits, the respawn, RESTART and the next fight into the last fight SHALL fade it; the revival and the class cancel SHALL NOT; a level exit SHALL fade it when its destination's music differs.

#### Scenario: Music fade
- **WHEN** the story CONTINUE after death opens the screen with the music at 0.5
- **THEN** the music is 0.25 at 300 ms, stops at black with the audio fade at 0, and the audio fade is 0.5 halfway through the fade in and 1 at the close

#### Scenario: Next fight into the boss
- **WHEN** NEXT FIGHT is confirmed after fight 6 and after fight 7
- **THEN** only the second fades the music

### Requirement: Session frame phase
The session frame task SHALL run the dialogue and the hook while no main menu covers the game in every phase but the transition's (phase word 0).

#### Scenario: Ended session
- **WHEN** an ended session with no hook runs a frame
- **THEN** only the dialogue ticks

### Requirement: Story start hand-over
A new story SHALL hand the loader the level-start point (8200, 25, 3920), the view forward along +z looking at (8200, 0, 4000) and a start record of the background's start motion followed by two zero words; a practice start's view SHALL be forward along −z looking at (0, 20, 500).

#### Scenario: A knight's start record
- **WHEN** a knight's story starts
- **THEN** its start record is the waking motion, 0, 0 and its spawn point is (8200, 25, 3920)

### Requirement: First wake-up end
While the first wake-up animation runs, the story hook SHALL end it once the player's animator word no longer holds 0x20, whatever the condition word holds.

#### Scenario: A waking condition with a clear animator word
- **WHEN** the condition holds 0x20 and the animator word does not
- **THEN** the first wake-up animation ends

### Requirement: Respawn and revive closes
A revival SHALL close the prelude before its save; the story respawn SHALL hide the end menu and close the prelude after its save and before the world goes, and SHALL NOT take the player's controls away.

#### Scenario: The story respawn
- **WHEN** the death screen's respawn runs
- **THEN** the end menu hides and the prelude closes, and no controls-off step is made

### Requirement: Death and continue details
The death path SHALL show the alive tip when the health is at least 0.01 in double precision or unordered; the death screen's CONTINUE SHALL write the player back with record flags 0x100; the next practice fight SHALL clear the player's condition word.

#### Scenario: A NaN health
- **WHEN** a story player dies with a NaN health
- **THEN** tip 2 shows and the save is neither dropped nor the player stripped

### Requirement: Sixth-level stamp
A level exit that makes the sixth level's checkpoint SHALL also stamp the level time; other checkpoints SHALL NOT.

#### Scenario: The third level's checkpoint
- **WHEN** a level-3 exit makes its checkpoint
- **THEN** no level-time stamp is made

### Requirement: Launch unlock words
The boot SHALL read the expert unlock from settings path [1, 0x6431ac01] and the background unlock from [1, 0x6431ac02], leaving a word unchanged when its entry is missing.

#### Scenario: A missing entry
- **WHEN** the store has neither entry and the expert ladder is already unlocked
- **THEN** it stays unlocked

### Requirement: Combat-mode setter
The practice start SHALL turn the player's combat mode on right after it attaches the controls, each next practice fight SHALL turn it on without touching the controls, and the story revive SHALL turn it off before it raises the body.

#### Scenario: The next fight
- **WHEN** the next practice fight starts
- **THEN** combat mode goes on and no controls step is made

### Requirement: Lie-down to wake
A new story's unknown background SHALL lie down to wake 3000 ms after its first motion, and the story respawn SHALL lie down with no delay after it attaches the controls and before its get-up motion; a lie-down SHALL write the condition 0x200, replace the flags' wake nibble with 0x30 and set the wake clock to the game clock plus the delay.

#### Scenario: A lie-down at 1200 ms
- **WHEN** a player with flags 0x600 lies down with a 3000 ms delay at game clock 1200 ms
- **THEN** its condition is 0x200, its flags 0x630 and its wake clock 4200 ms

### Requirement: Story revive body
The story revive SHALL raise a dead player body (yellow health half the red, laid down at once), then set the held bit 0x40 and push the wake clock 2000 ms on, then enable the controls; a living body SHALL NOT be raised.

#### Scenario: A revive at 1200 ms
- **WHEN** a dead player with red health 0.5 is revived at game clock 1200 ms
- **THEN** its yellow health is 0.25, its flags 0x70 and its wake clock 3200 ms

### Requirement: Live wake tick
A body carrying a knock-out SHALL run the wake tick each character tick while its flags hold 0x20, and SHALL clear the wake nibble once its readiness reaches 1 unless it is held.

#### Scenario: A body ready to stand
- **WHEN** a knocked-out body's readiness reaches 1
- **THEN** its flags no longer hold 0x20 and its stance factor is the race's

### Requirement: Expert unlock write
In the practice mode, a novice-ladder win SHALL show tip 5 and, when the expert unlock was not already stored or written, write the 4-byte value 1 under settings path [1, 0x6431ac01] once and flush the store; an expert-ladder win SHALL show tip 6 and write nothing; outside the practice mode neither tip nor write SHALL be made.

#### Scenario: Two novice wins
- **WHEN** a practice player wins the novice ladder twice with no stored unlock
- **THEN** the first win writes [1, 0, 0, 0] under [1, 0x6431ac01] and the second writes nothing

#### Scenario: An arena-mode round end
- **WHEN** the practice round's last fight ends while the mode is the arena
- **THEN** no tip 5 and no settings write are made

### Requirement: Flow action switch
Each motion the flow asks for (the story start's, the delayed wake's, the respawn's and the practice start's and next fight's) SHALL start the numbered action on the player's animator unless the player is already in that action, looping when its clip loops and otherwise playing once back into the combat or idle action.

#### Scenario: A body already in the action
- **WHEN** the practice motion is asked for while the player's animator already runs that action
- **THEN** the animator is left as it is

### Requirement: Practice spawn point
The practice start and each next fight SHALL place the player at x 0, z 500 cm with the spawn table's half turn, replacing only the vertical with the body's height.

#### Scenario: A practice start
- **WHEN** a practice start runs with a standing height of 92 cm
- **THEN** the player is placed at (0, 97, 500)
