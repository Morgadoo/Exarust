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
The system SHALL play the walk clip whenever the player moves forward, walking or running: the body tick's locomotion SHALL start it in the tick it first moves the player, from any clip, and the steering SHALL end it in the idle clip when the player stops out of combat mode. Standing still out of combat mode, the steering SHALL start no other clip: the stance decides, so the combat idle stays until the player walks.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk in the tick the body first moves, and back to idle

#### Scenario: Combat idle after combat mode
- **WHEN** Tab turns combat mode off while the player stands in Cmbt - Idle with the Defence stance, and it then turns in place, walks and stops
- **THEN** it keeps Cmbt - Idle while standing and turning, plays the walk from the tick W is decided, and stops in idle

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
The system SHALL steer the player in exploration as the original does. W SHALL walk along the heading; A and D SHALL turn it 0.025 rad a tick while moving, or a quarter standing, once more than 1 s of the game clock has passed since any turn key. S SHALL backpedal, braking first above 1 cm a tick. A released walk SHALL go on to its walk's key segment 0 or 4, then stop softly 8·v + 20 cm ahead. Shift SHALL jog. Neither camera nor cursor SHALL change the heading.

#### Scenario: Walking forward
- **WHEN** W is held while the camera is orbited
- **THEN** the player keeps walking the same way

#### Scenario: Turning in place
- **WHEN** D is pressed while standing
- **THEN** the player turns a quarter to the right

#### Scenario: Quarter turns a second apart
- **WHEN** D is pressed standing, again half a second later, and A two seconds after the first
- **THEN** the heading turns a quarter right, stays, and turns back a quarter left, the body following at 120°/s

#### Scenario: Releasing W mid-stride
- **WHEN** W is released while the player walks
- **THEN** it walks on until its walk reaches key segment 0 or 4, then brakes to a stop about 30 cm on

#### Scenario: W and Shift
- **WHEN** W and Shift are held
- **THEN** the player runs with the run bit set and the sprint latch spent at every movement decision

### Requirement: Walk to the cursor
The system SHALL walk the player toward the cursor's point on the plane 80 cm above its ground while the right button is held: the heading SHALL be whichever of the smoothed cursor direction and the facing is nearer the cursor's, turning first beyond 120°. Beyond 3 m it SHALL run, keeping Shift's sprint latch, and turn standing at 300°/s. The point SHALL be found through the scene camera's own viewport, also when supersampled.

#### Scenario: Far cursor
- **WHEN** the right button is held over a point 5 m away
- **THEN** the player runs toward it

#### Scenario: Supersampled view
- **WHEN** the right button is held with the pointer just above the player on screen in the default supersampled view
- **THEN** the player walks up the screen toward that point

#### Scenario: Far cursor and Shift
- **WHEN** the right button and Shift are held over a point 5 m away
- **THEN** the movement decisions see the run bit and the sprint latch both set

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab, and turn it on at a practice or arena fight's start. In combat mode the heading SHALL turn to the cursor once more than 30° off it, and follow it while the left button is held; the body SHALL turn toward it at the table's rate (300°/s standing). W, S, A and D SHALL step along and across the cursor's direction; their walk SHALL give back the clip it replaced; a walk from before combat mode SHALL stop softly and end in the stance.

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

### Requirement: Crouch
The system SHALL set the player's crouch amount to 1 while the Crouch binding is held and the body is not moving or is down, and SHALL lower it by 0.02 each tick while it is above 0. The body's bearing SHALL carry the amount. Above a crouch of 0.1 the locomotion clip clock SHALL run × (1 − 0.1 × crouch); above 0.5 the footwork's step length SHALL be × 0.7; a crouched attacker SHALL pick the first side attack.

#### Scenario: Crouch in place
- **WHEN** the player holds X while standing
- **THEN** the crouch amount is 1 and the body lowers its pelvis

#### Scenario: Ease out
- **WHEN** the player releases X at a crouch of 1
- **THEN** the amount falls by 0.02 a tick and reaches 0 within 50 ticks

#### Scenario: Not while walking
- **WHEN** the player presses X while walking
- **THEN** the crouch amount does not rise

#### Scenario: Slower crouched walk
- **WHEN** the player walks at a crouch of 1
- **THEN** the walk clip's clock runs at 0.9 of its rate

### Requirement: Grid look-ahead
The player's look-ahead SHALL walk the navigation grid from the standing point toward the look-ahead goal (75 cm + 6·v², 70 cm when backing), as the original does. Blocked, the free walk SHALL end and the body walk to the clamped goal and brake there; a clamped goal within 100 cm SHALL also cut the run and the sprint latch. A soft stop's goal SHALL be clamped the same way.

#### Scenario: Walking into a crate
- **WHEN** the player walks toward a crate lower than its hip
- **THEN** the player brakes to a stop in front of it instead of pushing into it

### Requirement: Locomotion request
The player SHALL move through a locomotion request (heading, look, flag word, moving, free walk, goal) that its body tick decides each tick: a free walk never brakes; toward a goal a walking body SHALL brake within its stop distance (50/200/250/50 cm, blended by the run; the sprint row's only with the sprint latch, a stance level above 0.9 and over 3.5 cm a tick) and stop within 25 cm at 0.5 cm a tick or less; a running body never brakes. A body under its most energy SHALL not walk.

#### Scenario: Walking into a wall
- **WHEN** the player walks straight at a wall
- **THEN** the look-ahead ends its free walk and it brakes to a stop before the wall

#### Scenario: Dazed
- **WHEN** a blow leaves the player under its most energy while W is held
- **THEN** it stands until its energy is full again, then walks

#### Scenario: The sprint row's stop distance
- **WHEN** a body with the sprint latch moves at 3.6 cm a tick at a half run blend, on a stance level of 1, then of 0.9
- **THEN** its stop distance is 150 cm, then the jog's 125 cm

#### Scenario: Stopping resets the columns
- **WHEN** a body whose stop distance was 140 cm stops and moves again
- **THEN** its gait columns are the walk row's, its stop distance 50 cm

### Requirement: Body turn
The body SHALL turn toward its heading by the original's table: (1 − 0.1·v)·240°/s in free walk (v in cm a tick), 120°/s for the exploration's slow turn, the swing's law while attacking, else (1 − 0.6·enc)·300°/s/√scale; ×0.6 crouched, eased over the last 0.6 rad, ×0.9 moving by the keys, the step ÷√scale; held while helpless; the heading error and the turned yaw each brought once within ±π. The alignment of its pelvis with its heading SHALL be kept each tick.

#### Scenario: A standing quarter turn
- **WHEN** D turns the standing player's heading a quarter
- **THEN** its body turns 120°/s until the last 0.6 rad, then eases onto the heading

#### Scenario: Turning in combat mode
- **WHEN** the cursor is moved beyond 30° of the standing player's facing in combat mode
- **THEN** its body turns toward it at 300°/s

#### Scenario: Across the half turn
- **WHEN** a body at yaw 3.1 rad turns toward a heading of −2.5 rad
- **THEN** it turns the 0.683 rad way and its yaw wraps under −3

### Requirement: Capsule executes the decision
Until the body walks by its carry, its capsule SHALL execute the movement decision: along the heading at 1.4 m/s walking up to 3.6 m/s at a full run blend (rising 0.05 a tick to the encumbrance's cap, falling 0.015), 1.22 m/s backpedalling, slowing while braking by the original's brake drive and enough to stop on its goal. Without a navigation grid the look-ahead SHALL clamp the goal a capsule radius and a 30 cm cell short of a wall at hip height.

#### Scenario: Soft stop
- **WHEN** W is released at a walk
- **THEN** the capsule slows from 1.687 cm a tick and stops within 3 cm of the goal 8·v + 20 cm ahead

### Requirement: Spawn snap
On the player's third behaviour tick with a navigation grid, unless it is down, getting up or dead, the player SHALL be moved across the ground onto the grid's nearest walkable point, once.

#### Scenario: The third tick
- **WHEN** the player's behaviour ticks three times with a grid, then again
- **THEN** nothing moves it on the first two, the third moves it across the ground to the nearest walkable point, and none after does

#### Scenario: Down on the third tick
- **WHEN** the player is down on its third behaviour tick
- **THEN** it is not moved, then or later

### Requirement: Footwork step targets
In combat mode a footwork step's target SHALL be a step's length along its keys (the step reach × √scale, 80 cm for a human), clamped by the grid's line walk from where the player stands; a target within 20 cm SHALL not be stepped to. Until the original's steps replace it, the interim footwork SHALL walk only while its step's target lies farther.

#### Scenario: Footwork at a wall
- **WHEN** W is held in combat mode facing a wall that clamps the step's target 20 cm ahead, then 21 cm ahead
- **THEN** the player takes no step and plays no walk, then it steps and plays the combat walk

### Requirement: Camera yaw wrap
Cursor Look SHALL keep the camera's target yaw within half a turn of the camera's present yaw, adding or subtracting a whole turn when the difference, compared in double precision, exceeds π.

#### Scenario: Wrapping the target
- **WHEN** the camera has orbited to a yaw of 7 rad and Cursor Look asks for a target yaw of 0.25 rad
- **THEN** the target becomes 0.25 + 2π

### Requirement: Camera pitch and orbit rate
The pitch floor SHALL be 0.8 rad below zoom 1000 and 0.000125 rad steeper per unit above, the zoom less 1000 in single precision and the rest in double. The camera SHALL hold the asked pitch, or the floor when that is steeper. The orbit SHALL turn 4.8 rad over the screen width, divided in double precision.

#### Scenario: Floor and held pitch
- **WHEN** the zoom is 1000.5 and the asked pitch is 0.5 rad
- **THEN** the floor is 0.8000625 rad and the camera holds it

### Requirement: Free zoom
A free zoom SHALL move 0.2 % of its value per wheel unit (120 to a notch), worked in double precision, but at least 0.5 % of its range, and SHALL stay in its range.

#### Scenario: A wheel notch
- **WHEN** a free zoom of 5000 in a 10–10000 range turns one notch in
- **THEN** it becomes 3800

### Requirement: Camera placement
A placement without a distance SHALL measure the eye from the looked-at point before its height clamp, scale the vector by its reciprocal length, and take the engine's planar yaw and arc sine of it.

#### Scenario: A clamped point
- **WHEN** the point (1, 5, 2) is clamped to a height of 10 and the eye is at (1, 20, 7)
- **THEN** the backward vector is (0, 15, 5) over its length and the yaw is 0

### Requirement: Camera preset
A preset SHALL fill its defaults: 280 and 16000 over 59², a retention of 3.94e-13 per second whose 59th root is 0.6162, pitch bounds of ±1.5 and distances of 10 to 10000. The stepped camera SHALL take speeds of 5 and 5/59; any other camera 15 × its largest distance / 59² and 6.6/59.

#### Scenario: The default preset
- **WHEN** a preset with every field zero is applied
- **THEN** its retention root is 0.6162 and its view scale 280 / 3481

### Requirement: Camera kick offset
A kick SHALL add its direction, normalised, to a small random offset and scale the sum to its size; a direction shorter than √0.1 SHALL be replaced by the camera's right axis times the sign of a random draw, lifted by a second draw.

#### Scenario: A degenerate direction
- **WHEN** a zero direction is kicked and the fourth draw is below a half
- **THEN** the basis is the camera's left

### Requirement: Cut-away segment
The cut-away segment SHALL start twice the camera's largest distance beyond the looked-at point along the backward vector and run back towards the point for twice that distance less 1 m.

#### Scenario: The segment
- **WHEN** the looked-at point is at height 10 m, the backward vector is +Z and the largest distance 20 m
- **THEN** the segment starts at z = 40 m and ends at z = 1 m

### Requirement: Mesh pick
The mesh pick SHALL visit nodes whose class matches 0x89 without the no-pick class bit and whose layers meet the view's. The segment, shortened by each hit, SHALL meet the node's bounding sphere and then its box, and each mesh SHALL be asked for a face within the best distance so far, keeping the nearest. With no hit the pick SHALL record where the ray meets the ground plane.

#### Scenario: A hit shortens the segment
- **WHEN** a mesh is met 1 m along and a later node's sphere lies 3 m along with a radius of 0.5 m
- **THEN** the later node is not asked

### Requirement: Kinesis pick
The kinesis pick SHALL need the node's kinesis option bit, look 40 m, grow each sphere by the caller's height and ask the mesh from where the ray crosses the grown sphere's top.

#### Scenario: Kinesis from above
- **WHEN** a ray from 10 m up looks straight down at a kinesis node of radius 1 m over a floor at height 0, with a height of 2 m
- **THEN** the pick hits at 10 m

### Requirement: Run blend
A moving body's run blend SHALL follow the original's locomotion: running, with support × √alignment × the stance level, then + 0.05 (double precision) to 1 − 0.3·enc (1 − 0.4·enc with the sprint latch); otherwise − 0.015, or braking or stopping at most f·(2 − f), f = max(0.2·(speed − 2), 0), never under 0; then − 0.005 per cm a tick of vertical travel, never under 0. A standing body's run blend SHALL fall 0.02 a tick, never under 0, and be 0 while it gets up.

#### Scenario: Running to the cap
- **WHEN** a human runs aligned on level ground from a blend of 0 for 20 ticks, then at encumbrance 0.5, then with the latch
- **THEN** its blend is 1, then 0.85, then 0.8

#### Scenario: A misaligned run
- **WHEN** a body runs with support at an alignment of 0.81 from a blend of 0.5, then without support
- **THEN** its blend stays 0.5, then rises to 0.55

#### Scenario: Braking and climbing
- **WHEN** a walking body at a blend of 0.85 walks a tick, brakes at 3 cm a tick, brakes at 2, then walks at 0.5 rising 2 cm a tick
- **THEN** its blend is 0.835, 0.36, 0, then 0.475

#### Scenario: Standing
- **WHEN** a standing body's blend is 0.5, and a body getting up's is 0.7
- **THEN** the first is 0.48 a tick later, the second 0

### Requirement: Gait table
A body's gait table SHALL be its race's rows scaled per tick as the original's loader scales them: thrust and clock rate × 1/83, push × (1/83)², the built-in stops 0.05, 0.2, 0.25 and 0.05 thousand cm, the backpedal's thrust and clock negated. A feral character's sprint row SHALL be (1 − f)·sprint + f·jog.

#### Scenario: A half-feral sprint
- **WHEN** a character of feral level 0.5 is given its table
- **THEN** its sprint row's stop distance is 225 cm

### Requirement: Gait columns
A moving body's gait columns SHALL be the backpedal row backwards; otherwise the walk row blended by the run blend toward the jog row, or toward the sprint row with the latch, a stance level above 0.9 and over 3.5 cm a tick; the clock by √run outside the run leg mode, × (1 − 0.1·enc) with the latch, × the character's clock factor, × (1 − 0.1·crouch) beyond a crouch of 0.1; thrust and push × the stance level. A body that stops SHALL take the walk row's columns.

#### Scenario: A human's rows
- **WHEN** a human's gait table is scaled
- **THEN** its walk thrust is 1.07108, its push 0.079837, its clocks 0.3253, 0.2, 0.16265 and −0.33735, and its stops 50, 200, 250 and 50 cm

#### Scenario: Half a run
- **WHEN** a human moves at a run blend of 0.5, walking, then running
- **THEN** its clock rate is 0.23670, then 0.26265, and its stop distance 125 cm

#### Scenario: The latch on the jog row
- **WHEN** a human runs at a full blend with the latch at encumbrance 0.5 on a stance level of 0.9
- **THEN** its clock rate is the jog's 0.2 blended at its cap of 0.8, × 0.95

### Requirement: Walk start
When locomotion finds another clip playing, the walk SHALL start from where the body is: more than 600 ms of game time after the last locomotion at key 2 ahead, 4 backwards, 5 (8 at a run blend of 0.5 or less) with the heading more than 1 rad on the yaw's negative side, none beyond 1 rad on its positive side, its clock at the key before; within 600 ms at the kept clock. The boost ramp SHALL be 0 at a start and rise 0.5/83 a tick to 1 while the walk plays, × the alignment.

#### Scenario: From idle
- **WHEN** a body that has not walked starts moving ahead from Idle
- **THEN** the walk plays from frame 10 with its capture meant for key 2

#### Scenario: Within 600 ms
- **WHEN** it stops and moves again 600 ms after it last walked, and again 612 ms after
- **THEN** the walk resumes at its kept clock, then starts at key 2 again

### Requirement: Locomotion drive
A moving body SHALL compute the original's drive each tick: braking −0.2·tone × its root's displacement; otherwise the heading × amount·thrust·tone·P − 0.4·tone × the displacement, P = (1.1 − 0.6·|0.5 − frac(2·progress + 1)|) × (2 − alignment²), boosted by (boost − 8·the speed along the heading) × the ramp under the row's boost speed. Its speed SHALL be its displacement along the heading. Until the body walks by its carry the drive SHALL be kept, not applied.

#### Scenario: A walking human's drive
- **WHEN** a human walks aligned at full tone, standing still at the cycle's start, at 1.687 cm a tick, and a quarter into the cycle
- **THEN** its drive along the heading is 0.85687, 0.18207 and 1.17819 cm a tick²

#### Scenario: Braking
- **WHEN** a braking body moved (0.5, 0, 2) cm last tick
- **THEN** its drive is (−0.1, 0, −0.4)

### Requirement: Stance facing
Each turn SHALL keep the stance facing: the heading while it is within a quarter turn of the body's yaw, else the yaw a quarter turn toward the heading. A turn toward a fall SHALL set it with the yaw.

#### Scenario: A reversal
- **WHEN** a body facing 0 rad turns toward a heading of 2 rad
- **THEN** its stance facing is a quarter turn, π/2

### Requirement: Torso twist
Each body tick a body not helpless and over 0.85 tone SHALL twist its torso toward its look: the look's yaw in last tick's frame (by the original's arc cosine), clamped to ±1 rad, followed 0.05 rad a tick standing, 0.02 locomoting, or the weapon's handling × 0.12 × the swing strength attacking; its direction lowered by the crouch (0.3 of it attacking), its height × 0.75 (−height² casting a power with a low one). Otherwise the twist SHALL be the identity.

#### Scenario: Looking aside
- **WHEN** a standing human looks 0.5 rad off its yaw for 11 ticks
- **THEN** its twist's yaw is 0.05 after one tick and 0.500297 after eleven

#### Scenario: Crouched
- **WHEN** a standing human crouches fully looking ahead
- **THEN** its twist looks along (0, −0.6, 0.8)

#### Scenario: Down
- **WHEN** the body is down
- **THEN** the twist is the identity

### Requirement: Spine takes the twist
The sampler SHALL turn the midline chain (from the root part, its first child within 3 cm of the midline, recursively, but the head) by the twist, the part under the root halfway back (its forward midway, its up square to it), each part keeping its place.

#### Scenario: Lumbar and thorax
- **WHEN** the twist turns 0.6 rad about the vertical
- **THEN** the thorax turns 0.6 rad, the lumbar 0.3, and both stay where the pose puts them
