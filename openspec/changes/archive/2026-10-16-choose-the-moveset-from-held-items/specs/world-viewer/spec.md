## MODIFIED Requirements

### Requirement: Scripted input and camera
`--press key@frame,...` SHALL press each key, or mouse button (`LMB`, `RMB`, `MMB`), at its frame counted from the end of the load and release it the frame after. In play mode, `--camera x,y,z,yaw,pitch` SHALL place a fixed camera there instead of the chase camera, and `--cursor x,y` SHALL hold the cursor at that window point (logical pixels).

#### Scenario: Scripted steps
- **WHEN** `exarust --pause-at-tick 300 --press .@500,.@510,.@520 character` runs
- **THEN** the simulation stops at tick 303

#### Scenario: Side view of a duel
- **WHEN** `exarust --opponent --camera 5.5,1.3,6.3,90,-6 play arena` runs
- **THEN** the camera stays at that pose, looking at the duel from its side

#### Scenario: Scripted punch
- **WHEN** `exarust --opponent --press Tab@100,LMB@200 play arena` runs
- **THEN** the player enters combat mode at frame 100 and throws a punch at frame 200
