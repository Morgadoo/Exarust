## MODIFIED Requirements

### Requirement: Scripted input and camera
`--press key@frame,...` SHALL press each key (`Ctrl`, `Shift` among them) or mouse button (`LMB`, `RMB`, `MMB`) at its frame counted from the end of the load and release it the frame after; `key@a-b` SHALL hold it from frame a to frame b. `--click frame,...` SHALL click the left mouse button the same way. In play mode, `--camera x,y,z,yaw,pitch` SHALL place a fixed camera there instead of the chase camera, and `--cursor x,y` SHALL hold the cursor at that window point (logical pixels).

#### Scenario: Scripted steps
- **WHEN** `exarust --pause-at-tick 300 --press .@500,.@510,.@520 character` runs
- **THEN** the simulation stops at tick 303

#### Scenario: Side view of a duel
- **WHEN** `exarust --opponent --camera 5.5,1.3,4.5,90,-6 play arena` runs
- **THEN** the camera stays at that pose, looking at the duel from its side

#### Scenario: Scripted punch
- **WHEN** `exarust --opponent --press LMB@200 play arena` runs
- **THEN** the player, in combat mode from the fight start, throws a punch at frame 200

#### Scenario: Scripted run to the cursor
- **WHEN** `exarust --press RMB@60-400 --cursor 800,120 play arena` runs
- **THEN** from frame 60 the player runs up the screen with two amber chevrons at that window position

#### Scenario: Scripted jog
- **WHEN** `exarust --press W@100-200,Shift@100-200 play arena` runs
- **THEN** the player jogs from frame 100
