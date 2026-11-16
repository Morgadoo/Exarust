## MODIFIED Requirements

### Requirement: Gamepad aim ease
Outside the controls' capture, a gamepad aim whose x² + z² is over 0.01 (compared in double precision) SHALL place the aim point 250·|aim| + 50 cm out along it, 80 cm over the ground, and the behaviour's screen cursor SHALL move (0.4·AimSpeed + 0.1)² of the way toward the point's screen projection each behaviour tick (in double precision, then single), starting from the pointer; the GUI's pointer SHALL be the eased point rounded.

#### Scenario: The shipped AimSpeed
- **WHEN** AimSpeed is 0.29 and the projected point is 100 px from the screen cursor
- **THEN** the cursor moves 4.6656 px in a tick

### Requirement: Behaviour aim lock
The aim height SHALL be a target point's within the pixel radius 80 cm make at its pelvis's depth and blend by (d − r)/r within twice it, the points read from the target's pelvis toward its head and its two lower legs; without a target a picked point above the plane's and under the player's pelvis plus 200 cm SHALL replace the aim point.

#### Scenario: The lock
- **WHEN** a bone is 15 px from the cursor with a 10 px radius
- **THEN** the aim height is halfway between the plane's and the point's

### Requirement: Camera follow step
When the camera follows the body, a camera yaw more than 0.1π from the body's yaw (the gap wrapped into ±π in double precision) SHALL become the body's yaw put back 0.1π on the camera's side, wrapped into ±π; within 0.1π it SHALL stay. The step SHALL wait while the controls hold the mouse and, in combat mode, while Interact is held.

#### Scenario: Camera one radian off
- **WHEN** the camera's yaw is 1.0 and the body's 0.0
- **THEN** the camera's yaw becomes 0.1π

#### Scenario: The capture holds
- **WHEN** the camera is one radian off while the controls hold the mouse
- **THEN** the camera's yaw stays 1.0

## ADDED Requirements

### Requirement: Screen cursor let go
A gamepad aim under the single-precision epsilon SHALL let the screen cursor go and warp the pointer to its eased point; the controls' capture SHALL let it go and warp the pointer to its drawn point.

#### Scenario: Stick released
- **WHEN** the stick returns to the centre while the pad drives the screen cursor
- **THEN** the pad no longer drives it

### Requirement: Screen cursor interpolation
Each simulation step's post-step SHALL move the eased point into the screen cursor's pair (the previous taking the current), and each frame SHALL draw the pair at the render fraction; while the pad drives it the world cursor SHALL be drawn at the drawn point rounded, ties to even.

#### Scenario: A quarter of a step
- **WHEN** the pair holds (10, 10) then (20, 30) and the render fraction is 0.25
- **THEN** the drawn point is (12.5, 15) and the world cursor is drawn at (12, 15)

### Requirement: Pad latch in the capture
While the controls hold the mouse, the aim stick's raw pair with a squared length over 0.01 (double precision) SHALL latch until the capture ends; latched, Cursor Look SHALL turn the cursor follower's target by −0.04·x each behaviour tick (double precision) and the orbit SHALL turn the camera by the pair as the camera stick does.

#### Scenario: Cursor Look latched
- **WHEN** the follower's target is 0.5 and the latched pair's x is 0.5
- **THEN** the target becomes 0.48

### Requirement: Cursor read outside an Overhead Swing
The behaviour's cursor read (the steering and the combat facing) SHALL take the plain point on the aim plane, and the aim point with its lock and raise only while the body plays Overhead Swing (action 12); the powers SHALL aim with the aim point.

#### Scenario: A raised point
- **WHEN** the aim point lies ahead and the plane point to the side
- **THEN** the cursor direction is the side's, and the aim point's during an Overhead Swing

### Requirement: Behaviour halt
While the body is shock-staggered (state flag 0x100) or the block pair suspends the behaviour, its tick SHALL stop after the contact list: no practice, steering, attack request or stance.

#### Scenario: A stagger
- **WHEN** W is held while the body is staggered
- **THEN** no walk is requested

### Requirement: Request drops
The movement request SHALL be dropped, after S's backwards flags, while the body plays Head Pain (action 45) and while the hand holds the kinesis pick's drag; Head Pain SHALL keep the combat mode under the right button.

#### Scenario: Head Pain
- **WHEN** W is held while the body plays Head Pain
- **THEN** no walk is requested

### Requirement: Kinesis pick drag
A press the running kinesis takes SHALL hold the hand in the GUI callback's drag until the left button's release, whose handler clears it.

#### Scenario: Release
- **WHEN** the left button is released during the kinesis pick's drag
- **THEN** the hand holds nothing

### Requirement: Placement rests the cursor follower
A placement of the player's body on the ground or at a frame SHALL rest the cursor follower at the placed raw cursor direction, the cursor's distance kept.

#### Scenario: Placed facing
- **WHEN** the body is placed facing −1.2 rad with the cursor 2.5 m away
- **THEN** the follower rests at −1.2 rad and the distance stays 2.5 m

### Requirement: Death lets the desk go
The player's death SHALL let the desk's GUI capture go.

#### Scenario: A form held
- **WHEN** the player dies while a form holds the desk's capture
- **THEN** the capture is free

### Requirement: Parry keeps the stance
In a parry's clinch the player's stance SHALL stay as it is.

#### Scenario: A close opponent
- **WHEN** an opponent comes within 3.5 m while the player parries in the Defence stance
- **THEN** the stance stays Defence

### Requirement: Capture answer
The controls' capture SHALL answer that it took the mouse while it holds, starts, ends or pins the pointer; the desk SHALL skip that frame's mouse-move processing.

#### Scenario: Holding
- **WHEN** the capture holds
- **THEN** the desk's forms take no pointer move that frame

### Requirement: Pad holds the capture
While the pad drives the screen cursor or its stick latched, a held capture SHALL start or stay without the mouse's work (no centring, no turn); its release SHALL set the screen cursor to the warp point.

#### Scenario: Driven cursor
- **WHEN** the capture is held while the pad drives the screen cursor
- **THEN** the pointer is not moved and the capture stays
