## MODIFIED Requirements

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

## ADDED Requirements

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
