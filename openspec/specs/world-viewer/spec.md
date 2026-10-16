# world-viewer Specification

## Purpose
Shows the original game's models and levels inside the Bevy runtime, as the
first step from decoded data to a playable world.

## Requirements

### Requirement: Model view
`exarust model <pack> <entry>` SHALL display every static mesh of the entry with its textures, framed by a camera that can fly around it.

#### Scenario: Prop
- **WHEN** `exarust model Objlib afr_cornflag_a01` runs
- **THEN** the textured plant is visible at its real size with alpha-masked leaves

### Requirement: Level view
`exarust level <name>` SHALL display the level's own scene meshes and every prop placed from the object library at its stored transform, with lighting and a free-fly camera.

#### Scenario: Story level
- **WHEN** `exarust level exanima01` runs
- **THEN** the level's props appear in their stored arrangement and the camera can move through the level

### Requirement: Shared resources
The viewer SHALL load each distinct model, material and texture once, however many times it is placed.

#### Scenario: Repeated props
- **WHEN** a level places the same object hundreds of times
- **THEN** its meshes and textures are decoded and uploaded once

### Requirement: Missing data is not fatal
The viewer SHALL show what it can when a prop's object, a material or a texture is missing, and log what was skipped.

#### Scenario: Unknown object
- **WHEN** a prop names an object absent from the installation
- **THEN** the level still loads and the missing name is logged once

### Requirement: Responsive load
The viewer SHALL build a level over frames, and SHALL run on the async compute pool the work that cannot be cut into short pieces, so the window keeps answering the desktop while a level loads.

#### Scenario: Large level
- **WHEN** `exarust play exanima01` loads
- **THEN** no frame of the load takes a second, and the desktop never reports the window as not responding

### Requirement: Complete first frame
Level and play modes SHALL end their load only when the voxel GI block around the starting view is built and warmed up and, in play mode, when the level's collision is built and every character is dressed. With GI turned off, the load SHALL end without waiting for it.

#### Scenario: First frame after the load
- **WHEN** `exarust play exanima01 --screenshot first.png --frames 1` runs
- **THEN** the screenshot shows the level lit by its GI and the player in braies, not the loading screen

#### Scenario: GI off
- **WHEN** `exarust play arenao1 --no-gi` runs
- **THEN** the load ends once the characters are dressed

### Requirement: Scripted runs after the load
`--frames` (before a screenshot) and `--shove` SHALL count frames from the end of the load, and `--autowalk` SHALL start walking then.

#### Scenario: Screenshot
- **WHEN** `exarust play exanima01 --screenshot shot.png --frames 60` runs
- **THEN** the screenshot shows the level 60 frames after the loading screen went

### Requirement: Window identity
The window SHALL carry the app id (X11 class) `exarust`, so the desktop names it in its dialogs.

#### Scenario: Wayland
- **WHEN** the window opens on a Wayland desktop
- **THEN** its toplevel's app id is `exarust`

### Requirement: Character creation scene
`exarust character` SHALL show the body in the original's character-creation scene: the `sgccflr01` floor and its two omni lights, with no sun and no ambient. Every surface SHALL fade to black by 2 m horizontal distance from the origin, over a black background. The camera SHALL be the original's: target (0, 0.6, 0) m, 4 m along (−2, 2.1, 3.6), 50° vertical field of view. A "Torch" or "1H & Torch" action SHALL show the body holding the lit torch in the hand that moveset raises.

#### Scenario: Default view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the body stands on cobblestones in a pool of white and warm light with black all around

#### Scenario: Torch moveset
- **WHEN** `exarust character Torch/Idle` runs
- **THEN** the body holds the burning torch raised in its right hand

#### Scenario: Torch beside a weapon
- **WHEN** `exarust character "1H & Torch/Idle"` runs
- **THEN** the body holds the burning torch raised in its left hand

### Requirement: Body overlay
`--debug-bodies`, or F3 in any view, SHALL draw every simulated body over its mesh between its last two ticks: each part's particles and tetrahedron; each link from its point on the part to its point on the parent, green within its slack (1 mm tolerance), yellow up to 1 cm past it, red beyond; pivots as crosses; a living body's muscle anchors, faint. F4 SHALL draw only the inspected body, and F5 SHALL inspect the body under the cursor.

#### Scenario: Standing duel
- **WHEN** `exarust --debug-bodies --opponent play arena` runs and both fighters stand
- **THEN** both are drawn over their meshes and their links are green

#### Scenario: Limp body
- **WHEN** `exarust --debug-bodies --limp character` runs
- **THEN** the falling body's parts and links are drawn on it as it falls and once it sleeps

#### Scenario: Off
- **WHEN** neither the flag nor F3 turned the overlay on
- **THEN** nothing is drawn and no measure is taken

### Requirement: Joint readout
With the overlay, the window SHALL show, for the inspected body (the player's unless another is picked), each joint's turn from its rest, its swing and twist about the bone axis, its distance from the clip's turn, the share of its range its slack links use, its pivot's gap and its worst link past its slack, each row coloured as that link; and the body's sleep state, support, balance, energy and tone, and the simulation's tick, speed and pause.

#### Scenario: A blow opens an elbow
- **WHEN** a hit makes the player's elbow pivot open more than 1 cm
- **THEN** the forearm's row is red and reads how far the elbow is from the clip's turn

### Requirement: Simulation controls
P SHALL pause and resume the fixed step (character controllers, character ticks and the body solver), `.` SHALL run exactly one tick while paused, and `[` and `]` SHALL step its speed through ×0.1, ×0.25, ×0.5, ×1 and ×2; `--sim-speed` SHALL set the starting speed. While paused or slowed, rendering SHALL go on and the cameras SHALL move on real time.

#### Scenario: One step
- **WHEN** the simulation is paused at tick 300 and `.` is pressed three times
- **THEN** ticks 301, 302 and 303 run and the simulation stays paused at 303

#### Scenario: Orbit while paused
- **WHEN** the simulation is paused in play mode and the middle mouse button drags
- **THEN** the camera orbits the frozen bodies

### Requirement: Scripted pauses
`--pause-at-tick <n>` SHALL pause the simulation at the end of body tick n, `--pause-on-stretch <cm>` at the end of the tick in which a body's link first goes that far past its slack, logging the body, part and link, and `--pause-after-jet <n>` at the end of the nth tick after the tick that first finds a blood jet started. With any of them, `--screenshot` SHALL count its frames from the pause.

#### Scenario: Freeze a fold
- **WHEN** `exarust --debug-bodies --opponent --pause-on-stretch 1 --screenshot s.png --frames 30 play arena` runs
- **THEN** `s.png` shows the duel stopped at the tick a link first opened 1 cm past its slack, with that part's row red

#### Scenario: Freeze a jet
- **WHEN** `exarust --opponent --weapon ub05ssworda --click 100,125,150,175,200 --pause-after-jet 12 --screenshot j.png --frames 20 play arena1` runs until a blow bleeds
- **THEN** `j.png` shows the duel stopped 12 ticks after that blow, with the jet's droplets flying off the struck part

### Requirement: Scripted input and camera
`--press key@frame,...` SHALL press each key (`Ctrl` among them) or mouse button (`LMB`, `RMB`, `MMB`) at its frame counted from the end of the load and release it the frame after; `key@a-b` SHALL hold it from frame a and release it at frame b. `--click frame,...` SHALL click the left mouse button the same way. In play mode, `--camera x,y,z,yaw,pitch` SHALL place a fixed camera there instead of the chase camera, and `--cursor x,y` SHALL hold the cursor at that window point (logical pixels).

#### Scenario: Scripted steps
- **WHEN** `exarust --pause-at-tick 300 --press .@500,.@510,.@520 character` runs
- **THEN** the simulation stops at tick 303

#### Scenario: Side view of a duel
- **WHEN** `exarust --opponent --camera 5.5,1.3,6.3,90,-6 play arena` runs
- **THEN** the camera stays at that pose, looking at the duel from its side

#### Scenario: Scripted punch
- **WHEN** `exarust --opponent --press LMB@200 play arena` runs
- **THEN** the player, in combat mode from the fight start, throws a punch at frame 200

#### Scenario: Scripted run to the cursor
- **WHEN** `exarust --press RMB@60-400 --cursor 800,120 play arena` runs
- **THEN** from frame 60 the player runs up the screen with two amber chevrons at that window position

### Requirement: Still opponent
`--still-opponent` SHALL spawn the play mode duellist as `--opponent` does, standing its ground: it SHALL face the player and take its stance, but neither walk nor strike.

#### Scenario: Stances at a distance
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs
- **THEN** the duellist stays 8 m from the player, facing it in the Defence stance

#### Scenario: Unattended attacks
- **WHEN** `exarust --opponent --weapon ub05ssworda --click 100,125,150 play arena1` runs
- **THEN** the player, in combat mode from the fight start, swings its sword at those frames

### Requirement: Debug wounds
`--debug-wound bone,radius_cm[,dx,dy,dz]`, repeatable, SHALL paint a test wound on the player's body or the character view's body once it is dressed and bound: at its body part's front (the part's visible vertex furthest along the body's facing), of that radius, smeared along d (cm, in the body's frame, +Y up, +Z forward). A bone SHALL be given by number (1–18) or by part name (`chest`, `luarm`, …).

#### Scenario: Chest and arm
- **WHEN** `exarust --debug-wound chest,20 --debug-wound luarm,12 character` runs
- **THEN** dark red blood with speckled edges and a wet sheen covers the front of the chest and of the left upper arm

#### Scenario: Smeared down the belly
- **WHEN** `exarust --debug-wound belly,15,0,-40,0 character` runs
- **THEN** the blood runs from the belly down over the braies

#### Scenario: Bad value
- **WHEN** `--debug-wound chest` is given without a radius
- **THEN** exarust prints the expected form and exits with code 2

### Requirement: Scripted health
In play mode `--health yellow,red` SHALL give the player that starting yellow and red health, and `--heartbeat` SHALL let the low-health heartbeat play in any level.

#### Scenario: Retail values
- **WHEN** `exarust --health 0.176,0.226 --press Tab@50 play arena` runs
- **THEN** from frame 50 the health bar shows the yellow, brown and red shares of the retail reference

#### Scenario: Heartbeat in the arena
- **WHEN** `exarust --heartbeat --opponent play arena` runs and the duellist's blow leaves the player's yellow health below 0.05
- **THEN** the heartbeat's border pulses for 7 s

### Requirement: Navigation grid overlay
`--debug-nav`, or F6 in any view, SHALL draw the level's navigation grid around where the camera looks: each floor cell within 8 m as a square at its floor's height, the lower layer green through yellow as its clearance rises to 180 and red beyond, the upper layer cyan through blue and magenta beyond.

#### Scenario: Arena floor
- **WHEN** `exarust --debug-nav play arena` runs
- **THEN** the floor around the player is covered in green squares, cut by the walls

### Requirement: Debug weapon blood
`--weapon-blood a` SHALL give the held weapon of the player's body, or of the character view's body, blood a (0 to 1) from when it is dressed, so its stain shows without a fight. A value that is not a number from 0 to 1 SHALL print the expected form and exit with code 2.

#### Scenario: Soaked sword
- **WHEN** `exarust --weapon ub05ssworda --weapon-blood 1 character` runs
- **THEN** the short sword is stained from its tip to below its guard

#### Scenario: Bad value
- **WHEN** `--weapon-blood 1.5` is given
- **THEN** exarust prints the expected form and exits with code 2

### Requirement: Debug blood jets
`--debug-spray bone,R[,dx,dy,dz]`, repeatable, SHALL fire a blood jet from the player's body or the character view's body every second of game time once it is dressed and bound: at its body part's front (as a debug wound's), as a bleeding blow of size R (over 0) whose striking point swung along d (body frame, +Y up, +Z forward; 0,0,−1 by default) would. A bone SHALL be given as for `--debug-wound`; a bad value SHALL print the expected form and exit with code 2.

#### Scenario: Chest jet
- **WHEN** `exarust --debug-spray chest,1 character` runs
- **THEN** every second 36 droplets spray out of the chest's front and fall

#### Scenario: Bad value
- **WHEN** `--debug-spray chest` is given without a size
- **THEN** exarust prints the expected form and exits with code 2
