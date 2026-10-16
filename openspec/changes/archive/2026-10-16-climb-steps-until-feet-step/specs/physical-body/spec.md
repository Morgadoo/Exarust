## ADDED Requirements

### Requirement: Climbing with the controller
Until the feet step and the legs bend (Exarust's own), while a living body that has not fallen has its character controller on the ground, standing or moving, the target pelvis height SHALL read each foot's ground no lower than the controller's floor less 6 cm. A controller in the air or under a fallen body SHALL change nothing.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its controller floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps with its controller, stopping on the third
- **THEN** it keeps its balance and ends on the landing, its pelvis 72 cm higher; with the original's target alone it loses its balance

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` or `--at 7.6,-1.3,66.2` walks the player up the stairs to the door
- **THEN** it climbs to the landing without falling on the stairs and stands there

#### Scenario: Perched
- **WHEN** a body's grounded controller stands 12 cm over the floor under its feet, or is in the air
- **THEN** its target is 6 cm higher than the original's, or the original's

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its grounded controller 12 cm over it
- **THEN** its target is the original's every tick, as with its controller on the floor
