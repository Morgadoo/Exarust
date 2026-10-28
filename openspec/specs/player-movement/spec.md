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
The system SHALL play the walk clip whenever the player moves forward, walking or running: the body tick's locomotion SHALL start it in the tick it first moves the player, from any clip, and the steering SHALL end it in the idle clip when the player stops out of combat mode. A walk or a run SHALL blend into idle over 20 frames with the fast ease; a backpedal SHALL end at once. Standing still out of combat mode, the steering SHALL start no other clip: the stance decides.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk in the tick the body first moves, and back to idle

#### Scenario: A walk ends over twenty frames
- **WHEN** the player stops a walk or a run out of combat mode, and later a backpedal
- **THEN** idle starts with a fast-eased blend of 20 frames, and the backpedal ends at once

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
The system SHALL steer the player in exploration as the original does. W SHALL walk along the heading; A and D SHALL turn it 0.025 rad a tick while moving, or a quarter standing, once more than 1 s of the game clock has passed since any turn key. S SHALL backpedal, braking first above 1 cm a tick. A released walk SHALL go on to its walk's key segment 0 or 4, then stop softly 8·v + 20 cm ahead in three dimensions. Shift SHALL jog. Neither camera nor cursor SHALL change the heading.

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
The system SHALL toggle combat mode on Tab, and turn it on at a practice or arena fight's start. In combat mode the heading SHALL turn to the cursor once more than 30° off it, and follow it while the left button is held; the body SHALL turn toward it at the table's rate (300°/s standing). W, S, A and D SHALL step along and across the cursor's direction; their walk SHALL give back the clip it replaced; a walk or run from before combat mode SHALL stop softly and end in the stance as a walk ends.

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
A click SHALL name a free hinge as a door, a dead character as a corpse and a living one as a person. A loose prop whose mobility is under 0.08 SHALL be ignored unless it is tagged bulky, and a prop needing strength SHALL be ignored when strength is under 0.9, both compared in double precision. A usable item on a prop SHALL stay usable, and a usable item with no body SHALL be nothing. A point farther than 70 cm SHALL be turned toward first.

#### Scenario: A door, a light prop, and the reach
- **WHEN** a free hinge, a blocked hinge, a prop of mobility 0.07, a prop of mobility 0.08, a prop of mobility 0.080001, a bulky prop of mobility 0.07, a usable prop, that usable prop with its body removed, a point 70 cm away and a point 70.1 cm away are read
- **THEN** the names are a door, nothing, nothing, nothing, a loose prop, a loose prop, usable, nothing; 70 cm does not turn and 70.1 cm does

#### Scenario: Strength
- **WHEN** a prop needing strength is read with strength 0.89, 0.9 and 0.900001
- **THEN** the first two are nothing and the third is a loose prop

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
The player's look-ahead SHALL walk the navigation grid from the standing point toward the look-ahead goal (75 cm + 6·v², 70 cm when backing), as the original does. Blocked, the free walk SHALL end and the body walk to the clamped goal and brake there; a clamped goal within 100 cm SHALL also cut the run and the sprint latch. A soft stop SHALL pull its goal in when the squared distance in three dimensions is over (8·v + 20 cm)², then the line walk SHALL clamp that goal the same way.

#### Scenario: Walking into a crate
- **WHEN** the player walks toward a crate lower than its hip
- **THEN** the player brakes to a stop in front of it instead of pushing into it

#### Scenario: A raised soft-stop goal
- **WHEN** a soft stop at speed 0 has a goal 10 cm ahead and 30 cm up, then one exactly 20 cm ahead on the level, then one with a NaN component
- **THEN** the first is pulled to 20 cm along the facing at the standing height, and the other two stay

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
In combat mode a footwork step SHALL aim from the chest: target = chest + v·(r·√scale/|v|) cm, v the key sum with a side key weighing 0.8, r the step reach × the legs' extension (80 cm for a human), or with the long-step latch (1.5 − 0.3·encumbrance) × the reach, narrowed once, times the extension. The line walk SHALL clamp it from the chest's x and z at the ground, and the step SHALL be taken only when the clamped target is over 400 cm² from the chest in three dimensions.

#### Scenario: Footwork at a wall
- **WHEN** W is held in combat mode facing a wall that clamps the step's target 20 cm ahead of the chest on its level, then 21 cm ahead
- **THEN** the player takes no step and plays no walk, then it steps and plays the combat walk

#### Scenario: A diagonal weighs the side by 0.8
- **WHEN** W and A are held together facing +Z, on open ground, for a fresh press of an 80 cm step
- **THEN** the target lies 0.8 as far to the side as it lies forward, and a side key alone still aims a full step

#### Scenario: A raised clamp
- **WHEN** the line walk clamps a step 15 cm ahead and 15 cm up, then 15 cm ahead on the level
- **THEN** the first is stepped to and the second is not

#### Scenario: A held step is a long step
- **WHEN** a combat step's keys have already been held past 200 ms, on open ground, for a human step reach and for a reach of 72 cm at encumbrance 0.5
- **THEN** the target is 120 cm and 97.2 cm along the keys, and a fresh press of the same reach stays at 80 cm

#### Scenario: A shortened extension halves the step
- **WHEN** the legs' extension is 0.5, on open ground, for a human's fresh press and for the same keys held past 200 ms
- **THEN** the targets are 40 cm and 60 cm along the keys

#### Scenario: A crouch of 0.1 keeps the short step
- **WHEN** a combat step's keys have already been held past 200 ms and the crouch is 0.1, then the float just under 0.1
- **THEN** the first target stays one step reach and the second is the long step

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

### Requirement: Head look
While not helpless and on a clip under action 0x2C, a body's head SHALL turn toward its look (the smoothed cursor moved toward the head's physical forward by 1 − E², in the body's frame, its height halved) by a weight rising 0.01 a tick to 1 (0 otherwise): each angle moved by the weight toward the look's (yaw within ±1.2566 rad × 0.8, followed at most π/100 rad a tick; pitch −asin of the height; roll a tenth of the yaw).

#### Scenario: Looking aside
- **WHEN** the look is 0.5 rad aside of the frame at full weight
- **THEN** the head's yaw moves π/100 a tick and settles at 0.40024

#### Scenario: Ramping in
- **WHEN** a body starts looking
- **THEN** its weight is 0.01 after a tick and 1 by its 120th

### Requirement: Looking head anchors
While the head looks, its link muscles' anchors SHALL be placed relative to the thorax's physical transform rather than its pose.

#### Scenario: A turned pose
- **WHEN** the whole pose is turned 0.5 rad about the thorax and the head looks
- **THEN** the head's anchors take the turn, which they do not when it does not look

### Requirement: Anchors of a head that does not look
While the head is not looking, its link anchors SHALL be placed from its pose relative to its parent's pose before the torso twist. The same twist on both SHALL leave those anchors where the untwisted pose puts them.

#### Scenario: A twisted thorax
- **WHEN** the thorax is twisted 0.5 rad about the vertical and the head keeps the untwisted pose
- **THEN** the head's anchors match the untwisted pose, which they do not when the anchors read the twisted thorax

### Requirement: Reach under the cursor
A point under the cursor SHALL be within reach when the point the pick hit is at most 245 cm high in the world and its offset from the player's hip is at most 300 cm up or down and 300 cm across the ground; a door SHALL be measured from its own place with 300 cm of height and 350 cm across. A comparison with a NaN SHALL pass.

#### Scenario: The limits
- **WHEN** the reach is tried with the hit 245 cm high and 300 cm away, 245.1 cm high, 300.1 cm away, 300.1 cm below, 500 cm high right at the hip, as a door 300 cm high and 350 cm away, and with NaN values
- **THEN** the first holds, the next four do not, the door holds and the NaN values pass

### Requirement: When the reach is measured
The reach SHALL be measured each frame: outside combat mode while the right button is not walking to the cursor, and in combat mode only with Ctrl held or something held, whatever the right button does. A drag in hand SHALL be within reach. The player down, collapsing or dead SHALL let go of everything and be out of reach. Outside combat mode the cursor SHALL show the pointer within reach and the cross beyond it.

#### Scenario: A refused press and the cursor
- **WHEN** a press is refused outside combat mode over a point within reach while walking to the cursor
- **THEN** the cursor shows the cross

### Requirement: Left press
The hand SHALL act on the left button's drag start, picking at the pressed point. A drag start SHALL be ignored while combat mode keeps the button for attacks (unless Ctrl is held or something is held), while the point under the cursor is out of reach, while the player is down, or when the pick hits nothing. Each drag start SHALL begin from nothing held.

#### Scenario: A busy hand
- **WHEN** a corpse is pressed in combat mode without Ctrl, out of reach, and with the player down
- **THEN** each press is ignored

### Requirement: What a press takes
A prop or item the hand can move SHALL, unless the player locomotes, begin the object's hold (the standing quarter turns wait) and with Ctrl be dragged by the part the pick hit at the hit point, strength 1, in the tick's first two passes; without Ctrl it SHALL hold until the release, which ends the hold. A corpse SHALL be dragged by the limb the pick hit, strength 0.5, every pass. Anything else but a door, a wheel or a sim object SHALL be refused until the release.

#### Scenario: What a press starts
- **WHEN** a loose prop is pressed with Ctrl, without Ctrl and while locomoting, a corpse with and without Ctrl, a person, the level and a crank
- **THEN** the prop is dragged at strength 1 for passes 0 and 1 with the object held, lifted, and ignored; the corpse is dragged at 0.5 in every pass; the person, the level and the crank are refused

### Requirement: A refused press
A refused press SHALL show the cross outside combat mode and SHALL leave the movement request as it was until the release; the lift SHALL leave the request as it was too.

#### Scenario: The request is kept
- **WHEN** the hand holds a refusal, then a lift, then a drag
- **THEN** the first two keep the request and only the refusal shows the cross

### Requirement: Drag lets go
Each frame of a drag the grabbed point SHALL be let go once its squared distance from the player's hip, compared in double precision, is over 108 900 cm², when the held body is a character that lives, or when the left button is released.

#### Scenario: Letting go
- **WHEN** the grabbed point is 330 cm, 330.1 cm and (200, 200, 200) cm from the hip
- **THEN** 330 cm holds and the other two let go

### Requirement: Drag follows the cursor
Within reach, the drag's anchor SHALL be where the cursor's ray meets the level plane at the grab's height, kept within 200 cm of the hip across the ground (and then at the grab's height), and the body SHALL be woken to 256; out of reach the anchor SHALL stay on the grabbed point.

#### Scenario: Following
- **WHEN** the cursor's ray falls straight onto the 30 cm plane 1.5 m away, then 5 m away along (3, 4)
- **THEN** the anchor is at 1.5 m, then pulled to (1.2, 0.3, 1.6) m

### Requirement: Pick under the cursor
The pick SHALL look 40 m along the cursor's ray at the level and the props' meshes, naming a prop's body and part by its mesh, and at the characters' collision spheres, the nearest hit first. A static mesh built under a named scene node SHALL be named as that node, and other static geometry SHALL stay the level; the hand SHALL treat a scene node as the level.

#### Scenario: The nearest character
- **WHEN** a dead character's sphere 3 m ahead and a living one's 6 m ahead are on the ray, with nothing fixed
- **THEN** the pick hits the dead one at 2.8 m and names a corpse, and a ray beside them hits nothing

#### Scenario: A scene node's mesh
- **WHEN** a unit cube under a scene node sits at 1 m height and another cube with no node at 2 m along z, and rays fall onto each from above
- **THEN** the first collider names the node and the second names none

### Requirement: Idle camera turn
Each frame the player SHALL be idle when the pointer stayed within 2 pixels of where it was last seen on both axes, the condition word is 0, no foot steps, the hand holds nothing and no mouse button is held; a dead player SHALL always be idle. A busy frame SHALL note the pointer and the game clock. The test SHALL follow the hand's frame, and the frame's systems SHALL be ordered without a cycle.

#### Scenario: Busy and idle frames
- **WHEN** the pointer moves 2 pixels, then 3 pixels, and the player walks, steps, holds an object or holds a button, and a dead player's pointer jumps
- **THEN** 2 pixels stays idle, 3 pixels and the busy cases are busy, and the dead player is idle

#### Scenario: The frame's order
- **WHEN** the frame's schedule of the player, the cursor, the hand, the idle turn, the hotkeys and the player light is built
- **THEN** it builds

### Requirement: Idle turn rate
Once the game clock is past the noted time plus 10 000 ms, an idle frame SHALL turn the camera's target yaw by 0.0001 rad for each millisecond since the last frame, in double precision then single, less 2π once past it, and the behaviour SHALL stop reading the cursor until the player is busy again.

#### Scenario: Ten seconds
- **WHEN** a busy frame at 1000 ms is followed by idle frames at 11 000, 11 001 and 12 001 ms
- **THEN** the yaw stays at 11 000 ms, turns 0.0001 rad at 11 001 ms and 0.1001 rad by 12 001 ms, and the cursor is no longer read

### Requirement: Hotkeys by state
A dead player's keys SHALL only end an open dialogue. With the power bar open a digit SHALL pick a power ('0' the tenth), the use key SHALL use the picked one and the arrows SHALL step through them; with a dialogue open '1' to '9' SHALL choose an answer and the Talk key SHALL end it; with neither the Talk key SHALL talk when talking is allowed.

#### Scenario: Dead, talking, the bar
- **WHEN** a dead player presses Tab, then with a dialogue open; a talking player presses 1, 9, 0 and T; the power bar is open and 0, 2, Q and the arrows are pressed
- **THEN** nothing, then the dialogue ends; answers 0 and 8, nothing, the end; powers 9 and 1, the use, a step forward and back

### Requirement: Play hotkeys
Any other key SHALL then be tested in this order: use the picked power (powers allowed), toggle combat mode, the inventory (allowed, and in the modes hiding it only while collapsing), the weapon swap (allowed), the held light, the skills and the journal (allowed and not hidden), the power tree (powers allowed and some), and the power bar (the player's or else the companion's powers).

#### Scenario: The play keys and their gates
- **WHEN** Tab, I, R, F, K, J, T, Q, P and C are pressed in play with no powers, then P and C with two powers, then C with a companion's power, then the window keys with no permission
- **THEN** combat, the inventory, the swap, the light, the skills, the journal, talking, the use and nothing twice; the tree and the bar; the bar; and nothing for each window key while Tab and F still act

### Requirement: Entering combat mode
Entering combat mode SHALL let go of what the hand holds and clear the request's backpedal, keys' heading and turn-about bits. Tab SHALL toggle a living player's combat mode in the running game.

#### Scenario: Letting go
- **WHEN** combat mode is entered with a drag in hand and the request carrying move, backpedal, keys' heading, turn-about and run, and Tab is pressed by a living and by a dead player
- **THEN** the drag is let go, the request keeps move and run, the living player's combat mode turns on and the dead one's does not

### Requirement: Player light switch
The player's own light SHALL burn while the player wears an item that sees in the dark, and SHALL go out while either hand holds an item with a held light's section; the switch SHALL be read whenever what the player wears or holds changes, making or removing the light.

#### Scenario: Helm and torch
- **WHEN** a night-vision helm is worn with empty hands, then with a torch in the off hand, and nothing is worn
- **THEN** the light burns, then goes out, and stays off

### Requirement: Player light frame
The light SHALL be an omni light of colour (1, 1, 1), radius 1200 cm, multiplier 2 and softness 4 that falls off with distance, made with intensity 0.01. Each frame while under 0.4 less the single-precision epsilon it SHALL gain 0.0005 per millisecond (double precision, then single), clamped to 0.4. It SHALL hang 80 cm above the head, or where a 10 cm ball swept up from the head first meets the level when that is nearer.

#### Scenario: Rising and hanging
- **WHEN** a 16 ms frame passes from 0.01, a frame from 0.399, and the head is 150 cm up with no ceiling, then with a ceiling the ball meets 35 cm up
- **THEN** the intensity is 0.018, then 0.4; the light hangs at 230 cm, then at 185 cm

### Requirement: Drag start
The drag SHALL start once the pointer, the left button held, has moved from the pressed point by more than 2 pixels on either axis, or, within the first 190 ms after the press, by more than 2 × ((200 − t) / 20) pixels with t the milliseconds since the press and the division an integer one.

#### Scenario: The allowance
- **WHEN** the pointer moves 20 then 21 pixels at once, 2 pixels by 2 then 3 pixels after 500 ms, and 1 pixel after 185 ms
- **THEN** 20 does not start the drag and 21 does; 2 does not and 3 does; 1 pixel after 185 ms does

### Requirement: Click turn
A left release before the drag starts SHALL be a click; outside combat mode a click SHALL turn the player's heading to the cursor's last direction when the cursor's last distance is over 70 cm.

#### Scenario: Turning
- **WHEN** clicks are made with the cursor 70 cm and 70.1 cm away
- **THEN** the first does not turn the player and the second does

### Requirement: Combat walk to the cursor
In combat mode, while the right button is held, the player SHALL ask the walk to the cursor however near it is, the cursor showing its chevrons. With the cursor 50 cm or more away the request SHALL keep only its sprint latch and its move, take the cursor's direction as heading and look, run when the cursor is beyond 3 m or turn the sprint latch into the run, and walk free; the footwork SHALL give way, and the release SHALL end the walk.

#### Scenario: Walking, running and the sprint latch
- **WHEN** the button is held with the cursor 2 m ahead, then 4 m to the side, then 2 m ahead with the sprint latch, then 20 cm away, then released
- **THEN** the body walks ahead with the move bit alone, runs to the side, runs ahead with the latch cleared, asks the walk without moving, and asks nothing

### Requirement: Combat walk look-ahead
While no foot steps, the combat walk SHALL cast the look-ahead 6·v² + 45 cm along the facing; a hit SHALL stop the free walk and, within 100 cm, the run and the sprint.

#### Scenario: Open ground
- **WHEN** the combat walk's look-ahead meets nothing
- **THEN** the walk stays free

### Requirement: Gamepad aim ease
A gamepad aim whose x² + z² is over 0.01 SHALL place the aim point 250·|aim| + 50 cm out along it, and the aim cursor SHALL move (0.4·AimSpeed + 0.1)² of the way toward it each behaviour tick (in double precision, then single), starting from the mouse's point; combat mode SHALL aim along it.

#### Scenario: The shipped AimSpeed
- **WHEN** AimSpeed is 0.29 and the point is 100 cm away
- **THEN** the cursor moves 4.6656 cm in a tick

### Requirement: Mechanism drag holds the hand
A drag start on a door, a wheel or cloth SHALL hold the hand until the release without keeping the movement request; a grabbed box SHALL be refused when a placed corner is higher than 280 cm or lower than −1000 cm, an unordered height passing.

#### Scenario: A door and a box
- **WHEN** the hand holds a door's drag, and boxes with corners at 280 cm, 281 cm, −1001 cm and NaN are tested
- **THEN** the request is not kept, and the boxes at 280 cm and NaN are taken, the others refused

### Requirement: Idle camera gates
A screen that blocks the controls SHALL turn the camera once the clock is ten seconds past the stamp whatever the pointer and the buttons do; the active view not being the game's (the in-game menu) SHALL only restamp the clock; a dialogue or the power bar SHALL keep the player busy.

#### Scenario: Blocked, a window and a dialogue
- **WHEN** at 20 000 ms a blocked frame comes with the pointer moved and a button held, a dialogue frame comes, and a window frame comes
- **THEN** the blocked frame turns the yaw by 2 rad and stops reading the cursor, the dialogue restamps the clock, and the window restamps it without touching the last frame's time

### Requirement: Behaviour aim lock
The aim height SHALL be a target point's within the pixel radius 80 cm make at its head's depth and blend by (d − r)/r within twice it; without a target a picked point above the plane's and under the ground plus 200 cm SHALL replace the aim point.

#### Scenario: The lock
- **WHEN** a bone is 15 px from the cursor with a 10 px radius
- **THEN** the aim height is halfway between the plane's and the point's

### Requirement: Player contact list
The contact list SHALL hold every character within 12 m, up to twelve, with closeness 1 − d·0.00083333335 capped at 0.99 and facing weight 0.75·(0.5 − 0.5·facing·dir) + 0.25·(0.5 − 0.5·cursor·dir)², each scoring half of both; a new character SHALL replace the lowest it outscores.

#### Scenario: Ahead and beside
- **WHEN** a character stands 100 cm ahead of the body and the cursor, and another ahead of the body with the cursor turned aside
- **THEN** the first's closeness is 0.9166667 with no facing weight, and the second's facing weight is 0.0625

### Requirement: Strongest engaged enemy
The strongest engaged enemy SHALL be the largest threat among the living characters within reach plus 70 cm, in front of the heading by more than 0.3 of their distance, whose own contact score for the player is over 2.

#### Scenario: An engaged and a calm enemy
- **WHEN** an enemy with threat 0.7 and a score of 2.5 for the player stands 100 cm ahead of a player with 60 cm of reach, and one with a score of 2
- **THEN** the first is found with threat 0.7 and the second is not

### Requirement: Attack timing
After a swing's tick the wind-up step SHALL be 1 − 0.05·√E (1 at no energy), at least 0.1, over the weapon's rate times 0.03, at least 0.01; the next attack SHALL come round(1000/(80·step)) ms on, at most the swing's length; the recovery SHALL end round((2f + 1)·(round(100·rate) + 100)) ms on, with f = (3 − 2b)·b², b = balance/0.7, while a swing's leg mode is on and the balance at most 0.7, else 1.

#### Scenario: Full energy and a poor balance
- **WHEN** a fighter of energy 1 swings a weapon of rate 0.1 at balance 1, and one of energy 0.5 at balance 0.5 in leg mode 5
- **THEN** the first attacks again 44 ms on and recovers 330 ms on; the second recovers 286 ms on

### Requirement: Item use values
A pour SHALL warm the character by 0.2, or 0.1 for a liquid of type 50, in double precision; a cleaning kit SHALL take a dirt byte over 0x20 to 0x20; a repair kit SHALL lower a wear byte by 255 over (rating + 5)/10 (at least 1), never under zero, and leave an unworn item; a kit's value SHALL move 0.0005 of its gap a millisecond, rounded to the nearest (ties to even), the timed list dropping an arrived entry by moving the last into its place.

#### Scenario: Kits over time
- **WHEN** a wear of 100 is repaired with a rating of 25, and a timed value runs from 100 to 0 for 1000 ms and 2000 ms
- **THEN** the target is 15, and the value is 50, then 0 and done

### Requirement: Camera wall probe
Each camera tick SHALL end with a wall probe from 50 cm under the followed point along the view: alternately the present step with a 30 cm ball (a hit climbs, to at most 4) and the step below with a 20 cm ball (a miss comes down), over 38.5 m; the tilt SHALL ease toward 0.15 rad steeper per step and be added to the view's pitch, held to 1.54 rad. With no obstruction the steps and the tilt SHALL stay 0.

#### Scenario: Open space
- **WHEN** the camera ticks with nothing in the probe's way
- **THEN** the view's pitch is the camera's own

### Requirement: Long step latch
The footwork's keys SHALL form a key code in which opposing keys cancel; the cursor walk is its own code. The clock SHALL be stamped when the code changes, and the long-step latch SHALL be set when the code is held, the stamp plus 200 ms is below the clock and the crouch widened to double is below 0.1. Only a code change or a release SHALL clear it; a later crouch or a blocked step SHALL not. The interim footwork SHALL walk only while a step is taken.

#### Scenario: The latch holds until the keys change
- **WHEN** W is pressed and held with steps of 80 and 120 cm, then the crouch goes to 0.5, then A and D are added, then W and A are held, then every key is released
- **THEN** the target is 80 cm through 16 ticks after the press and 120 cm from the 17th, stays 120 cm after the crouch and with W, A and D, the hold restarts at 0 with W and A, and the release clears the latch

### Requirement: Click on what is under the cursor
A click outside combat mode SHALL name what it hits as the original's click handler does: a placed object with no usable section SHALL show the item; a lock SHALL open its container when open, and otherwise, with the key and its reward due, pay 100 experience, or 1000 for the lock subtype 3; usable kinds 0 and 1 SHALL open the container and any other kind SHALL show the item. A click on the player's own body SHALL show the health and focus bars for 5000 ms.

#### Scenario: Own body
- **WHEN** the player clicks its own body outside combat mode at game time 1000 ms
- **THEN** the HUD bars are held shown until 6000 ms

#### Scenario: Lock subtype 3
- **WHEN** a closed lock of subtype 3 is clicked with its key and its reward due
- **THEN** the click pays 1000 experience

### Requirement: Double-click interaction
A double click SHALL be a second press of the same button within the double-click time (500 ms) and under 2 pixels of the first; it SHALL interact only outside combat mode or with Ctrl held. On a placed object the hand-held item (kind 1, unmarked) SHALL end the interaction; every other branch SHALL go on to open a container section. The player's own body SHALL open the inventory.

#### Scenario: Consumable in a container
- **WHEN** an unmarked consumable of kind 0 subtype 1 with a container section is double-clicked at 1000 ms
- **THEN** it is consumed at 3100 ms and its container opens

### Requirement: Item use marks and timed values
A repair SHALL mark the item or its container entry with 0xFF0000 and both pours with 0xC80000; the elixir's sound SHALL play only with the potion. The timed list SHALL write a section only while its byte lowers or the entry arrives, and SHALL skip the weapon section in a frame where the usable section exists and is not written.

#### Scenario: Usable section not lowered
- **WHEN** a repair entry's value is 20 and the usable section's byte is 10 while the weapon section's is 90
- **THEN** nothing is written that frame

### Requirement: Pick-only geometry
A model's `_null` groups SHALL be built as static colliders that only the cursor's pick meets; the character controller, the bodies' level contacts, sweeps and the other rays SHALL pass through them.

#### Scenario: A pick-only box
- **WHEN** a pick-only unit cube under a scene node sits at 1 m height and rays and a 10 cm ball fall onto it from 3 m
- **THEN** the cursor pick meets it 1.5 m down and names the node, and the plain ray, the filtered pick, the sweep and the level pieces find nothing

### Requirement: Item cursor turn input
The system SHALL turn a lifted item from the pad's raw move stick when its squared length is over 0.1 (compared in double precision), its forward part about the camera's axis and its sideways part about the world's up axis, and otherwise from the move keys, Forward over Back and Left over Right, each giving the turn speed; the speed SHALL be the ramp factor times the last frame's milliseconds times 0.0025, multiplied in double precision; mouse motion SHALL NOT turn the item.

#### Scenario: Forward stick
- **WHEN** the move stick is fully forward on a 100 ms frame at the base factor
- **THEN** the item turns 0.0625 rad about the camera's axis and none about the up axis

#### Scenario: Stick in the dead zone
- **WHEN** the move stick is 0.3 long and no key is held
- **THEN** the item does not turn

### Requirement: Item cursor placement
The system SHALL place a lifted item at the cursor ray's level hit, the held item left out of the pick, when the hit's height is not under −100000 cm, else where the ray crosses that height unless the ray climbs, and SHALL place it whether or not its box stays within 280 cm up and −1000 cm down.

#### Scenario: Box out of range
- **WHEN** the placement point puts the item's box above 280 cm
- **THEN** the item is still placed there and the box test answers no

### Requirement: Item cursor release
The system SHALL leave a plainly released lifted item where the cursor put it, SHALL give it to a GUI item slot it is over, and SHALL put a lift cancelled by the movement request back at its saved frames unless the world hover bit is set.

#### Scenario: Plain release
- **WHEN** the left button is released over the level with no hover word
- **THEN** the item stays where it was placed

#### Scenario: Cancelled lift
- **WHEN** the lift is cancelled with no hover word
- **THEN** the item goes back to the frames saved at the lift

### Requirement: Ground pick
The ground pick SHALL look 40 m along the unit screen ray, grow each pickable node's bounding sphere by 5 cm when the caller accepts the node and by 1 mm otherwise, and ask the node's mesh from where the ray crosses the grown sphere's top for the rest of the reach; a crossing that is not positive SHALL become zero (the mesh asked from the eye with the whole reach). Each answering node SHALL shorten the reach and become the pick; with none the pick SHALL record the ground plane.

#### Scenario: An eye under the grown top
- **WHEN** an eye 0.3 m above a floor looks straight down inside an accepted node of radius 0.5 m centred on the floor
- **THEN** the ground pick hits at 0.3 m

#### Scenario: A taken node from above
- **WHEN** a ray from 10 m up looks straight down at an accepted node of radius 0.5 m about a floor at height 0
- **THEN** the pick hits face 0 at 10 m

### Requirement: Stepped wheel zoom
The stepped camera's wheel SHALL read its zoom as a stop level (the zoom less its least over a fifth of its range, rounded to the nearest with ties to even), subtract the wheel's units over 120, hold the level between 0 and 5 by the processor's max then min, and write the level back over the stop. The chase camera's wheel SHALL use it with one notch per scroll line and 24 pixels per notch.

#### Scenario: One notch in from the third stop
- **WHEN** a zoom of 1280 in an 800–2000 range takes 120 wheel units
- **THEN** it becomes 1040

### Requirement: Zoom slide and pitch offset
A vertical zoom slide SHALL clamp `dy · 0.001 + base` to 0–1, square it and multiply it into the range above the least zoom. A non-zero pitch amplitude SHALL give an offset `exp(3 · ln(1 − (zoom − min)/(max − min))) · amplitude` with the engine's fast logarithm and exponential, which steepens the pitch only.

#### Scenario: Halfway slide
- **WHEN** a slide of 500 pixels from base 0 runs over an 800–2000 range
- **THEN** the zoom is 1100

### Requirement: Free-camera drag
An absolute drag SHALL start from the shown yaw and minus the engine's arc sine of the view's backward height, move both by the rate times the sensitivity times the pixel delta, hold the pitch's negation below the low bound first and above the high bound second (a NaN passing), and on release snap a quarter turn from the shown yaw in the moved direction when the rounded count of quarter turns has non-zero low 32 bits and is positive.

#### Scenario: A released drag
- **WHEN** a drag is released 0.9 rad from a shown yaw of 0
- **THEN** the yaw becomes π/2

### Requirement: Editor grab drag
The grab drag SHALL keep the grabbed node's ground position at the target's height, reach the node's half extent plus the zoom, turn by half the rate per pixel, raise the grabbed point by `dy · zoom / (width · aspect / 2)` and put the looked-at point at the raised point plus the camera axis at pitch zero times the reach.

#### Scenario: A raised grab
- **WHEN** a grab with zoom 10 in a 1600-pixel viewport of aspect 1.6 moves 100 pixels down
- **THEN** the point and the looked-at point rise by 100 · 10 / 1280

### Requirement: Preset editor scan
Under its editor flag a preset SHALL scan the scene for nodes of class 0xA9, each replacing the default 800 by 0 by 1000 target box with its centre and its three half extents, a 0xAE marker raising the extra flag; a first distance below the least distance SHALL be raised to it, and a first distance that is not positive, or is NaN, SHALL derive the placement's distance.

#### Scenario: The last 0xA9 node wins
- **WHEN** the scene holds two 0xA9 nodes, the last with half extents (8, 9, 10) and the marker
- **THEN** the target box takes (8, 9, 10) and the flag is raised
