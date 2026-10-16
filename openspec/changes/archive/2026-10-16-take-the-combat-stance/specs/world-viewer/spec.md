## ADDED Requirements

### Requirement: Still opponent
`--still-opponent` SHALL spawn the play mode duellist as `--opponent` does, standing its ground: it SHALL face the player and take its stance, but neither walk nor strike.

#### Scenario: Stances at a distance
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs
- **THEN** the duellist stays 8 m from the player, facing it in the Defence stance

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
- **WHEN** `exarust --opponent --press LMB@200 play arena` runs
- **THEN** the player, in combat mode from the fight start, throws a punch at frame 200
