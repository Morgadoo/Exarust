## MODIFIED Requirements

### Requirement: Scripted input and camera
`--press key@frame,...` SHALL press each key at its frame counted from the end of the load and release it the frame after, and `--click frame,...` SHALL click the left mouse button the same way. In play mode, `--camera x,y,z,yaw,pitch` SHALL place a fixed camera there instead of the chase camera.

#### Scenario: Scripted steps
- **WHEN** `exarust --pause-at-tick 300 --press .@500,.@510,.@520 character` runs
- **THEN** the simulation stops at tick 303

#### Scenario: Side view of a duel
- **WHEN** `exarust --opponent --camera 5.5,1.3,6.3,90,-6 play arena` runs
- **THEN** the camera stays at that pose, looking at the duel from its side

#### Scenario: Unattended attacks
- **WHEN** `exarust --opponent --weapon ub05ssworda --press Tab@30 --click 100,125,150 play arena1` runs
- **THEN** the player enters combat mode and swings its sword at those frames
