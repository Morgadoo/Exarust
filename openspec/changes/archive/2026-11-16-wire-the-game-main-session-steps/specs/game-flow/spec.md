## MODIFIED Requirements

### Requirement: Session bookkeeping
Leaving a session SHALL stop the hook and frame task, sweep the game screens, end every conversation, close the prelude, and run the leave hook of the arena or of a registered scripted mode. It SHALL then close the save package, hide the end menu and turn the in-game GUI off before the world goes. Back to the title SHALL show the main menu's title logo and leave the GUI scale alone.

#### Scenario: Back to the title
- **WHEN** a story returns to the title
- **THEN** the title logo shows and neither the help flag nor the GUI scale is written

#### Scenario: A registered word without the scripted bit
- **WHEN** a session of a registered word without the `0x1000` bit begins and then leaves
- **THEN** its begin hook runs and its leave hook does not

### Requirement: Practice next fight reload
The practice ladder's next fight SHALL reload its arena (`arena`, or `arenae` for the expert ladder) with no scene filter, then equip the player's prepared kit, before the heal and the placement.

#### Scenario: Expert next fight
- **WHEN** the expert ladder's fight 7 advances
- **THEN** the steps load `arenae` with a teardown and the next step equips the prepared kit

### Requirement: Practice spawn point
The practice start and each next fight SHALL place the player at x 0, z 500 cm with the spawn table's half turn over the origin, with no floor probe: the pelvis at the fresh pose's pelvis target plus 5 cm at the start, at its stand height at each next fight.

#### Scenario: A practice start
- **WHEN** a practice start runs
- **THEN** the player is placed at (0, 0, 500) cm with the pelvis target plus 5 cm as its frame height

#### Scenario: A next fight
- **WHEN** the next practice fight starts
- **THEN** the placement at (0, 0, 5) m carries the stand height as its frame height across the arena's load

### Requirement: Death and knock-out block the controls
The flow SHALL block the player's controls at the knock-out (after its tip), at the death (after its tip and the save's fate, before the end menu), and at the practice ladder's last win in the practice mode (after its tip and settings write), and SHALL release them at the revive after the wake is held.

#### Scenario: Knock-out steps
- **WHEN** the story player is knocked out
- **THEN** the steps sweep the game screens, take the controls, show tip 2 and block the controls, in that order

### Requirement: Session ends release the button
The death's, the knock-out's and the level exit's steps SHALL end the GUI's held left button before the game screens are swept.

#### Scenario: The exit's steps
- **WHEN** a dead player leaves a level
- **THEN** the first step ends the held button and the second sweeps every game screen and the shown power bar

### Requirement: Story respawn portal
The story respawn SHALL load `exanima05` with the session-start arrival word, so the level's reconstruction device is the player's object. Once the level is in, the player's pelvis SHALL stand at that object's frame 15 cm up, with no floor probe, which the flow records as the respawn portal; without one, at the identity frame 15 cm up.

#### Scenario: The story respawn in exanima05
- **WHEN** the death screen's respawn loads `exanima05`
- **THEN** the player stands 15 cm over the level's one reconstruction device and that device is the recorded portal

#### Scenario: The frame's height is the pelvis's
- **WHEN** the respawn's placement is put in the simulation
- **THEN** the body's living placement takes the frame's own height for the pelvis

## ADDED Requirements

### Requirement: Practice role
The practice start SHALL clear the player's roles and give it role 16 (Unaligned) with the parents it inherits.

#### Scenario: A surviving player
- **WHEN** the practice start's steps reach a player that holds the Player role
- **THEN** its roles are role 16 alone

### Requirement: Session dialogue end
Leaving a session and the story end menu's exit SHALL end every conversation: live ones with their topic recorded, all released, a pending narration dropped and the reply panel closed.

#### Scenario: An open reply panel
- **WHEN** a story session is left with the reply panel open
- **THEN** the panel is closed and no narration is pending

### Requirement: First wake-up after the load
A new story of the unknown background SHALL start watching its first wake-up once its level is in, before the start's motion; every load's GUI reset SHALL stop watching it.

#### Scenario: The new story's load
- **WHEN** a new story's load runs its GUI reset and its level comes in
- **THEN** the first wake-up is not watched until the start's later steps run, and is watched after them

#### Scenario: A door during the wake-up
- **WHEN** a level change is asked while the first wake-up is watched
- **THEN** it is no longer watched

### Requirement: Camera shot pointer warp
A camera shot SHALL warp the pointer to the client point it framed while the window has the focus, and leave the pointer alone otherwise.

#### Scenario: A focused practice shot
- **WHEN** the practice start's shot runs in a focused 1600 by 1140 window at scale 2
- **THEN** the pointer is warped to the shot's point in logical pixels

### Requirement: In-game GUI across a session
Each session start and continue SHALL turn the in-game GUI on once its level is in. The death, the knock-out, the level exit and the story exit SHALL leave it on; only the session's leave SHALL turn it off.

#### Scenario: A practice session
- **WHEN** a practice start's level is in, the player dies and the session is left
- **THEN** the in-game GUI is on after the start and the death and off after the leave

### Requirement: Session begin and continue flags
A session's begin and continue SHALL clear the end menu's no-row-selected flag. A story continue SHALL clear only the first wake-up's animating flag.

#### Scenario: A story continue
- **WHEN** a story save is continued while a get-up is pending
- **THEN** the end menu's no-row-selected flag and the animating flag are clear and the get-up stays pending
