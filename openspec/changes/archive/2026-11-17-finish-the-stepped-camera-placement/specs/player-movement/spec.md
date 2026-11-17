## MODIFIED Requirements

### Requirement: Camera shot placement
A scripted camera shot SHALL set the camera's target and shown yaw to the engine's planar yaw (its fast arc cosine) of the vector from the target to the eye, set the backward vector to that vector's direction and stop it, and clear the wall avoidance's steps and tilt. The shot's distance, or the camera's current distance without one, SHALL become the shown zoom and both zoom spring values, and the zoom SHALL be it raised to the smallest distance.

#### Scenario: The story start
- **WHEN** a shot puts the eye 600, 600, 1300 cm from the player with distance 1300
- **THEN** the yaw is the planar yaw of that offset, the shown zoom is 1300 and the zoom 1280

#### Scenario: A distance past the stops
- **WHEN** a shot's distance is 5000
- **THEN** the zoom is 2000 and the shown zoom 5000

### Requirement: Game camera preset
The chase camera SHALL take its numbers from the game camera's preset record as the original's preset applies it: the backward spring's stiffness SHALL be the preset view over 59² (250/3481) and its retention the 59th root of the default 3.94e-13, the zoom SHALL start from the preset's first distance 1300 (on the stepped camera its nearest stop) between its smallest and largest distances 800 and 2000, and the zoom spring's rate SHALL be 5/59 and its step 5.

#### Scenario: The spring constants
- **WHEN** the chase camera is built
- **THEN** its stiffness is 250/3481 in single precision and its retention 0.6162461

## ADDED Requirements

### Requirement: Stepped placement tail
On the stepped camera a placement SHALL lower the zoom to the largest distance, move it to its nearest stop, hold the pitch at the steeper of the asked pitch and that stop's floor, and set the pitch target and the shown pitch to the placement's own pitch held between that and the cap. The target SHALL stay until the wheel, a zoom step or an orbit sets it again.

#### Scenario: Between the stops
- **WHEN** a placement on the stepped camera takes a distance of 1300
- **THEN** the zoom is 1280 and the pitch at least the floor of 1280

#### Scenario: A steep shot
- **WHEN** a shot looks straight down with an asked pitch of 0
- **THEN** the pitch is 1.5 and stays there until a handler runs

### Requirement: Camera starts on a stop
A new chase camera SHALL run the placement with its record's distance, so the Character record's 1300 gives a zoom of 1280 and a field of view of 29.93° at 1600 × 1080, and the pitch floor SHALL follow the target zoom.

#### Scenario: A new camera
- **WHEN** the chase camera is built
- **THEN** its zoom is 1280 and its shown zoom 1300

### Requirement: Free camera pan keys
The free camera SHALL pan with the keys bound to the forward, back, left and right movement slots.

#### Scenario: A rebound forward key
- **WHEN** forward is bound to I while spectating with a free camera
- **THEN** I pans the camera and W does not
