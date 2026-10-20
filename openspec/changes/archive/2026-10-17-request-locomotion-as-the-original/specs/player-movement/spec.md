## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Locomotion request
Every steered character SHALL move through a locomotion request (heading, look, flag word, moving, free walk, goal) that its body tick decides each tick: a free walk never brakes; toward a goal a walking body SHALL brake within its gait's stop distance (50/200/250/50 cm, blended by the run) and stop within 25 cm at 0.5 cm a tick or less; a running body never brakes. A body under its most energy SHALL not walk.

#### Scenario: Walking into a wall
- **WHEN** the player walks straight at a wall
- **THEN** the look-ahead ends its free walk and it brakes to a stop before the wall

#### Scenario: Dazed
- **WHEN** a blow leaves the player under its most energy while W is held
- **THEN** it stands until its energy is full again, then walks

### Requirement: Body turn
The body SHALL turn toward its heading by the original's table: (1 − 0.1·v)·240°/s in free walk (v in cm a tick), 120°/s for the exploration's slow turn, the swing's law while attacking, else (1 − 0.6·enc)·300°/s/√scale; ×0.6 crouched, eased over the last 0.6 rad, ×0.9 moving by the keys, the step ÷√scale; held while helpless. The alignment of its pelvis with its heading SHALL be kept each tick.

#### Scenario: A standing quarter turn
- **WHEN** D turns the standing player's heading a quarter
- **THEN** its body turns 120°/s until the last 0.6 rad, then eases onto the heading

#### Scenario: Turning in combat mode
- **WHEN** the cursor is moved beyond 30° of the standing player's facing in combat mode
- **THEN** its body turns toward it at 300°/s

### Requirement: Capsule executes the decision
Until the body walks by its carry, its capsule SHALL execute the movement decision: along the heading at 1.4 m/s walking up to 3.6 m/s at a full run blend (rising 0.05 a tick to the encumbrance's cap, falling 0.015), 1.22 m/s backpedalling, slowing while braking by the original's brake drive and enough to stop on its goal. The look-ahead SHALL clamp the goal a capsule radius and a 30 cm cell short of a wall at hip height.

#### Scenario: Soft stop
- **WHEN** W is released at a walk
- **THEN** the capsule slows from 1.687 cm a tick and stops within 3 cm of the goal 8·v + 20 cm ahead

### Requirement: Duellist through the request
The play mode duellist SHALL approach through the locomotion request: farther than 1.15 m from its target its goal SHALL lie 1.5 m toward the target, out of free walk, its capsule walking 1.2 m/s; nearer it SHALL stop. Its body SHALL turn toward its target at the table's rate, by the swing's law while attacking.

#### Scenario: Duellist approach
- **WHEN** `exarust --opponent play arenao1` starts the fight
- **THEN** the duellist walks in toward the player, stops within 1.15 m and strikes
