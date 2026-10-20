# player-movement Specification

## Purpose
Lets the player move a character through original levels with collision, as
the foundation of physically driven bodies and combat.

## Requirements

### Requirement: Level collision
The system SHALL turn the meshes of a level shown in play mode into static colliders, so characters stand on floors and are stopped by walls.

#### Scenario: Standing on the arena floor
- **WHEN** `exarust play arena` starts
- **THEN** the player stands on the floor instead of falling through it

### Requirement: Clips follow movement
The system SHALL play the walk clip whenever the player moves forward, walking or running, starting it from any looping clip, and SHALL end it in the idle clip when the player stops out of combat mode. Standing still out of combat mode, the steering SHALL start no other clip: the stance decides, so the combat idle stays until the player walks.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk and back to idle

#### Scenario: Combat idle after combat mode
- **WHEN** Tab turns combat mode off while the player stands in Cmbt - Idle with the Defence stance, and it then turns in place, walks and stops
- **THEN** it keeps Cmbt - Idle while standing and turning, plays the walk, and stops in idle

#### Scenario: Running plays the walk
- **WHEN** the player runs forward
- **THEN** the clip is the walk

### Requirement: Sprint latch
Shift with the cursor further than 3 m SHALL keep the sprint latch. Shift without that far cursor SHALL NOT keep it.

#### Scenario: Cursor beyond 3 m
- **WHEN** Shift is held and the cursor is further than 3 m
- **THEN** the sprint latch is kept

#### Scenario: Walk key
- **WHEN** Shift is held and the cursor is not that far
- **THEN** the sprint latch is not kept

### Requirement: Chase camera
The system SHALL keep the camera 20 m from the player's pelvis, drawn between its last two body ticks as its mesh is (the player's feet while it has no body), and SHALL zoom by field of view (zoom / 2000 × 45° × √(1.6 / aspect), six wheel stops from 800 to 2000). The pitch SHALL follow the zoom (0.8 to 0.925 rad below the horizon) unless the middle-mouse orbit drag raises it, up to 1.5 rad. The camera SHALL never turn by itself in exploration.

#### Scenario: Zoom
- **WHEN** the mouse wheel is turned
- **THEN** the field of view steps between 18° and 45° while the distance stays 20 m

#### Scenario: Walking bob
- **WHEN** the player walks
- **THEN** the camera rises and falls with the pelvis, centred on it in every frame

### Requirement: Original starts
The system SHALL start the player where the original does for a new game in the first story level, the arena hub and arena matches, facing as the original does; in an arena duel the opponent SHALL stand opposite, both facing the arena's centre. In a level without an arena ring (the practice arenas) the opponent SHALL stand 2.5 m in front of the player, facing it.

#### Scenario: Arena duel
- **WHEN** `exarust --opponent play arenao3` runs
- **THEN** the player stands 2 m from the arena centre along +Z facing it, and the opponent 2 m along −Z facing it

#### Scenario: Practice duel
- **WHEN** `exarust --opponent play arena` runs
- **THEN** the player stands at (0, 0, 5) facing −Z and the opponent at (0, 0, 2.5) facing it, so the duellist's blows meet the player's front and land

### Requirement: Tank controls
The system SHALL steer the player in exploration as the original does. W SHALL walk along the character's heading; A and D SHALL turn it by 0.025 rad per 83 Hz tick while moving, or by a quarter turn (at most once a second) when standing. S SHALL brake when faster than 1 cm per tick, then backpedal. Neither the camera nor the cursor SHALL change the heading.

#### Scenario: Walking forward
- **WHEN** W is held while the camera is orbited
- **THEN** the player keeps walking the same way

#### Scenario: Turning in place
- **WHEN** D is pressed while standing
- **THEN** the player turns a quarter to the right

### Requirement: Walk to the cursor
The system SHALL walk the player toward the cursor's point on the plane 80 cm above the player's ground while the right mouse button is held, running when that point is more than 3 m away. The point SHALL be found through the scene camera's own viewport, also when the scene is drawn supersampled into a larger image.

#### Scenario: Far cursor
- **WHEN** the right button is held over a point 5 m away
- **THEN** the player runs toward it

#### Scenario: Supersampled view
- **WHEN** the right button is held with the pointer just above the player on screen in the default supersampled view
- **THEN** the player walks up the screen toward that point

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab, and SHALL turn it on at the start of a practice or arena fight. In combat mode the heading SHALL turn to the cursor's direction once it is more than 30° off it, and follow it while the left button is held; the body SHALL turn toward the heading at the body-turn rate. W, S, A and D SHALL step along and across the cursor's direction; their walk SHALL give back the clip it replaced, and a walk from before combat mode SHALL end in the stance.

#### Scenario: Side step
- **WHEN** A is held in combat mode
- **THEN** the player steps to its left while still facing the cursor

#### Scenario: Small cursor move
- **WHEN** the cursor moves 20° to the side of the facing in combat mode
- **THEN** the player keeps its facing, and turns toward the cursor once it is more than 30° off

#### Scenario: Fight start
- **WHEN** `exarust --opponent play arenao1` shows the arena
- **THEN** the player is in combat mode without Tab, and Tab turns it off

#### Scenario: Stepping from the guard
- **WHEN** A is held and released while the player stands in the Defence stance
- **THEN** it plays the combat walk while A is held and the Defence stance again after

#### Scenario: Tab while walking
- **WHEN** Tab is pressed while the player walks out of combat mode, and the keys are released
- **THEN** it stands in its stance instead of walking on the spot

### Requirement: Carried torch
The system SHALL give the player the game's lit torch in play mode in story levels. It goes in the right hand with the "Torch" moveset, or in the left hand with "1H & Torch" when a weapon is held. Each hand places it by the original's grip table: an offset plus Euler angles applied about X, then Z, then Y of the hand. Practice and arena levels SHALL start the player without a torch. The torch's light SHALL also light its holder.

#### Scenario: Starting a level
- **WHEN** play mode starts in a dungeon
- **THEN** the player holds a burning torch raised in the right hand that lights and shadows the room

#### Scenario: Starting a story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the player holds a burning torch raised in the right hand, and it lights and shadows the room and the player's own skin

#### Scenario: Arena match
- **WHEN** `exarust play arenao1` runs
- **THEN** the player carries no torch, walks with the default moveset, and no torchlight falls on the body

#### Scenario: Exploring with the torch
- **WHEN** the player carrying the torch in the right hand stands out of combat mode
- **THEN** it holds the torch up beside its head as "Torch" Idle asks: the elbow bent about 123°, the upper arm about 40° out to the side, the wrist at shoulder height

#### Scenario: Torch in combat mode
- **WHEN** the player carrying the torch in the right hand attacks in combat mode with no target near
- **THEN** the attack ends in "Torch" Cmbt - Idle, the carry of the reference shots: the upper arm tucked and 32° forward, the elbow bent about 100°, the wrist 32 cm before the shoulder

### Requirement: Start outfit
The system SHALL start the player in play mode as the original's new character without a background kit: wearing only the underwear, with any apparel given on the command line put on over it. Apparel that covers the groin SHALL take the underwear's place.

#### Scenario: New game
- **WHEN** `exarust play exanima01` runs without `--outfit`
- **THEN** the player wears pale grey braies and nothing else

#### Scenario: Shirt given
- **WHEN** `exarust play exanima01 --outfit shirt_b` runs
- **THEN** the player wears the shirt with the braies beneath it

#### Scenario: Trousers given
- **WHEN** `exarust play exanima01 --outfit trouser_a,shirt_b` runs
- **THEN** the player wears the trousers and the shirt and no braies

### Requirement: Camera ticks
The camera SHALL tick every 17 ms of real time, a tick running once the frame passes its start, at most nine a frame with the rest of a backlog dropped. Each frame SHALL show the backward vector (renormalised), the zoom and a shake offset between the last two ticks, by how far the frame is into the last tick; the shake offset SHALL move the eye only, and only while the present offset, squared, exceeds 1e-5 cm².

#### Scenario: A stalled frame
- **WHEN** a frame comes a second after the last
- **THEN** the camera runs nine ticks and shows the last of them

#### Scenario: Between two ticks
- **WHEN** a frame falls halfway into a tick
- **THEN** the zoom shown is halfway between the two ticks' zooms

### Requirement: Camera shake
The camera SHALL keep a shake offset and the offset before its last tick. A kick SHALL add its vector to the offset and subtract it from the one before. Each camera tick SHALL carry the offset on: while its change from the one before, squared, exceeds 1e-7 cm² or its own square exceeds 1e-4 cm², the one before SHALL become the offset and the offset (offset + 0.96 × change) × 0.99; otherwise both SHALL be zero.

#### Scenario: A 1 cm kick
- **WHEN** the camera is kicked by 1 cm and ticks once
- **THEN** its shake offset is 2.8908 cm and the one before 1 cm

#### Scenario: The wobble dies away
- **WHEN** a 1 cm kick is left for 6 s of camera ticks
- **THEN** the offset has swung back through zero and is zero

### Requirement: Sprint thrust
While sprinting, the capsule's forward speed SHALL blend from the walk speed to the jog speed times the feral sprint thrust over the jog thrust, by the run blend. The feral sprint thrust SHALL be 472.2 at feral 0 and the jog thrust 350 at feral 1. Without sprinting the jog end SHALL stay 3.6 m/s.

#### Scenario: Full sprint
- **WHEN** the run blend is 1, the body is sprinting and the feral level is 0
- **THEN** the forward speed is 3.6 × 472.2 / 350 metres per second

#### Scenario: Feral sprint
- **WHEN** the run blend is 1, the body is sprinting and the feral level is 1
- **THEN** the forward speed is 3.6 metres per second

#### Scenario: Not sprinting
- **WHEN** the run blend is 1 and the body is not sprinting
- **THEN** the forward speed is 3.6 metres per second

### Requirement: Cursor pick
A click SHALL name a free hinge as a door, a dead character as a corpse and a living one as a person. A loose prop lighter than 0.08 SHALL be ignored, and a heavy prop SHALL be ignored when strength is under 0.9. A usable item on a prop SHALL stay usable, and a usable item with no body SHALL be nothing. A point farther than 70 cm SHALL be turned toward first. A point SHALL be within reach at a height of 245 cm and 300 cm away, and a door at 300 cm high and 350 cm away.

#### Scenario: A door, a light prop, and the reach
- **WHEN** a free hinge, a blocked hinge, a prop of mass 0.07, a prop of mass 0.08, a usable prop, that usable prop with its body removed, a point 70 cm away and a point 70.1 cm away are read, and the reach is tried at 245 cm high and 300 cm away, then 0.1 cm farther, then as a door at 350 cm
- **THEN** the names are a door, nothing, nothing, a loose prop, usable, nothing; 70 cm does not turn and 70.1 cm does; the first reach holds, the farther one does not, and the door does

### Requirement: Drag reach
A drag SHALL pull along the cursor direction by the nearest accepted hit, starting from 1000 cm. A hit at or beyond 1000 cm SHALL leave that reach. The grabbed box SHALL be refused when any placed corner is higher than 280 cm or lower than 1000 cm, and a corner on either limit SHALL be kept.

#### Scenario: A near hit and a high box
- **WHEN** hits at 1200 cm and 400 cm are taken along +X, and a box centred at 270 cm with 10 cm of half-height is tested, then the same centre with 20 cm of half-height
- **THEN** the pull is 400 cm along +X, the first box is held, and the second is refused

### Requirement: The sprint latch tightens the run cap
While the sprint latch is kept, the run blend SHALL stop at 1 minus 0.4 times encumbrance. The walk keys SHALL stop at 1 minus 0.3 times encumbrance. Encumbrance 0.5 with Shift and the cursor 4 m ahead SHALL reach 0.8 and keep the latch. Shift with the walk key and no cursor SHALL reach 0.85 and spend the latch.

#### Scenario: Cursor 4 m ahead, then the walk key
- **WHEN** a player of encumbrance 0.5 holds Shift with the right button and the cursor 4 m ahead for 40 steering ticks, then releases the button, presses the walk key, and is steered for 40 more ticks
- **THEN** the first tops the run blend at 0.8 and keeps the sprint latch, and the second tops it at 0.85 and spends the latch

### Requirement: Cursor Look
The system SHALL capture the mouse while the Cursor Look binding is held, hide the system and world cursors, turn the player's facing, heading and combat aim by 3.6 × Δx / window width radians for horizontal mouse motion, and turn the camera's target behind the new facing. Releasing the binding or losing window focus SHALL restore the pointer to where it was.

#### Scenario: Turn with the mouse
- **WHEN** Cursor Look is held and the mouse moves 800 logical pixels right across a 1600-pixel window
- **THEN** the player turns 1.8 radians right and the camera target follows behind it

#### Scenario: Hide the cursors
- **WHEN** Cursor Look is held
- **THEN** neither the system pointer nor the world cursor is drawn

#### Scenario: Release
- **WHEN** Cursor Look is released
- **THEN** the mouse is released and returned to its earlier position

### Requirement: Bound controls
The system SHALL read movement, primary attack, alternate attack, combat toggle, orbit camera and interact from the player's binding table rather than fixed keys.

#### Scenario: Rebound forward
- **WHEN** Forward is rebound from W to another key
- **THEN** W no longer walks the player and the new key does

#### Scenario: Either shift
- **WHEN** either shift key is held with default bindings
- **THEN** the player runs
