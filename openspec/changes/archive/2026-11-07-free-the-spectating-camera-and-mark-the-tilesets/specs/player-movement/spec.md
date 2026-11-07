## ADDED Requirements

### Requirement: Free camera
A camera that follows nothing SHALL run the free branch each tick: the movement keys make a flattened, normalised pan along the drawn right and forward axes; the terrain lifts the looked-at point toward a metre over the ground; the drift is the point's last move plus, with a pan, the pull ((shown zoom/2000 + 0.25)·6000/59² cm) along it; a drift past 1e-5 squared moves the point by 0.01^(1/59) of it, its height held to 0–200 cm. A frame SHALL draw the point between its last two ticks.

#### Scenario: Panning while spectating
- **WHEN** `play arena7 --spectate` runs and W is held for 25 frames
- **THEN** the camera's point moves forward along the view and coasts to a stop

#### Scenario: No pan
- **WHEN** no key is held and the point is still
- **THEN** the point stays where it is

### Requirement: Grid tileset probe mark
Loading a grid wall or floor set SHALL mark every node whose name does not start with the byte 0xE4 with 0x10000000 in its second flag word before the set is used, so the set's meshes that carry 0x200000 are wall-probe obstacles.

#### Scenario: A dungeon wall behind the player
- **WHEN** the player in exanima01 backs up to a wall
- **THEN** the wall probe climbs and the view tilts steeper

## MODIFIED Requirements

### Requirement: Camera wall probe
Each tick of a camera that follows a body SHALL end with a wall probe from 50 cm under the body's hip along the view: alternately the present step with a 30 cm ball (a hit climbs, to at most 4) and the step below with a 20 cm ball (a miss comes down), over 38.5 m; the tilt SHALL ease toward 0.15 rad steeper per step, scaled by the zoom spring's rate, added to the view's pitch (held to 1.54 rad). A free camera SHALL run no probe.

#### Scenario: Open space
- **WHEN** the camera ticks with nothing in the probe's way
- **THEN** the view's pitch is the camera's own

#### Scenario: A free camera
- **WHEN** the spectating camera follows nothing
- **THEN** no probe runs and the tilt holds

### Requirement: Wall probe obstacles
The wall probe SHALL meet only drawn meshes whose second flag word holds both 0x10000000 and 0x200000, that no simulated body owns and that the cut-away fade has not marked, when its segment meets the mesh's bounding sphere and then, for kind 0x5, a ball swept from the segment's far end back toward its start, otherwise the one-sided ray from the start.

#### Scenario: A wall facing the player
- **WHEN** the probe's ray meets the face of a level wall marked as an obstacle that turns toward the player
- **THEN** the probe reports a hit and the steps climb

### Requirement: Spectate session
The controls' spectate session SHALL, at its start, set the spectate flag and free the camera; while it runs the player SHALL not be steered, and a double click SHALL make the camera follow the character picked under the cursor, or free it when the pick is anything else or nothing; a freed camera SHALL start its free branch from where it last looked. Its end SHALL clear the flag and the follow. The play flag `--spectate` SHALL start it once the load is over.

#### Scenario: Following the opponent
- **WHEN** `play arena --opponent --spectate` runs and the opponent is double clicked
- **THEN** the camera follows the opponent and the player stands where it is

#### Scenario: A miss
- **WHEN** the double click picks the floor or nothing
- **THEN** the camera follows nothing and its free branch starts at its last point
