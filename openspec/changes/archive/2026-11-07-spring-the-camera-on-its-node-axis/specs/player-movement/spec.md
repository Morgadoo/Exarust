## ADDED Requirements

### Requirement: Camera heading
Each camera tick SHALL read the shown yaw as the engine's planar yaw of the backward vector; when it lies more than half a turn from the last shown yaw the target yaw SHALL turn a whole turn the same way.

#### Scenario: Turning across the seam
- **WHEN** the shown yaw is π − 0.1 and the target yaw π + 0.2
- **THEN** the shown yaw crosses to −π and the target becomes π + 0.2 − 2π

### Requirement: Camera backward spring
The camera node SHALL be turned to the pitch plus the wall tilt (held to 1.54 rad) and the target yaw led at most π/4 ahead of the shown yaw. The backward vector SHALL move by the retention times its last move plus the stiffness times its gap to the node's backward axis, normalised, only while that velocity's square exceeds 1e-8. A frame SHALL draw the vector between its last two ticks.

#### Scenario: Coming to rest
- **WHEN** the target yaw holds still
- **THEN** the backward vector comes to rest on the node's backward axis

### Requirement: Zoom spring
A camera tick SHALL snap the shown zoom to the target when they are within 0.001; otherwise the zoom spring SHALL move by the change toward the target at 5/59 less its last move, held to 5 a tick, carried by its last move and damped by 0.998, never under half the smallest zoom. A frame SHALL draw the field of view from the spring's last two values, and the stored shown zoom SHALL take the drawn one once they are 0.01 apart.

#### Scenario: A wheel notch out from 1300
- **WHEN** the zoom target moves from 1300 to 2000
- **THEN** the spring moves 4.99, 9.97, 14.94 and 19.90 in its first four ticks

### Requirement: Wall probe obstacles
The wall probe SHALL meet only drawn meshes whose second flag word holds both 0x10000000 and 0x200000 and that no simulated body owns, when its segment meets the mesh's bounding sphere and then, for kind 0x5, a ball swept from the segment's far end back toward its start, otherwise the one-sided ray from the start.

#### Scenario: A wall facing the player
- **WHEN** the probe's ray meets the face of a level wall marked as an obstacle that turns toward the player
- **THEN** the probe reports a hit and the steps climb

## MODIFIED Requirements

### Requirement: Camera wall probe
Each camera tick SHALL end with a wall probe from 50 cm under the followed point along the view: alternately the present step with a 30 cm ball (a hit climbs, to at most 4) and the step below with a 20 cm ball (a miss comes down), over 38.5 m; the tilt SHALL ease toward 0.15 rad steeper per step, its change scaled by the zoom spring's rate, and be added to the view's pitch, held to 1.54 rad. With no obstruction the steps and the tilt SHALL stay 0.

#### Scenario: Open space
- **WHEN** the camera ticks with nothing in the probe's way
- **THEN** the view's pitch is the camera's own

### Requirement: Game camera preset
The chase camera SHALL take its numbers from the game camera's preset record as the original's preset applies it: the backward spring's stiffness SHALL be the preset view over 59² (250/3481) and its retention the 59th root of the default 3.94e-13, the zoom SHALL start at the preset's first distance 1300 between its smallest and largest distances 800 and 2000, and the zoom spring's rate SHALL be 5/59 and its step 5.

#### Scenario: The spring constants
- **WHEN** the chase camera is built
- **THEN** its stiffness is 250/3481 in single precision and its retention 0.6162461

### Requirement: Camera shot placement
A scripted camera shot SHALL set the camera's target and shown yaw to the engine's planar yaw (its fast arc cosine) of the vector from the target to the eye, set the backward vector to that vector's direction and stop it, and clear the wall avoidance's steps and tilt. A shot with a distance SHALL set the shown zoom and both zoom spring values to it and the zoom to it held between the smallest and largest distances; a shot without one SHALL keep the zoom.

#### Scenario: The story start
- **WHEN** a shot puts the eye 600, 600, 1300 cm from the player with distance 1300
- **THEN** the yaw is the planar yaw of that offset and the zoom is 1300

#### Scenario: A distance past the stops
- **WHEN** a shot's distance is 5000
- **THEN** the zoom is 2000 and the shown zoom 5000
