# Spec Delta

## ADDED Requirements

### Requirement: Leg bend
A leg SHALL bend onto its bend target. The knee SHALL come from the law of cosines on the two bones: 30° when the cosine is above 0.866, and 180° when it is below −1. The correction from the clip SHALL advance at most 0.02 rad a tick (0.2 while stepping) plus its stored rate, never past the target. The rate SHALL become half that sum, within ±0.1 rad. Muscles SHALL follow the correction on the next tick, not the tick that computed it.

#### Scenario: Folded to 40 cm
- **WHEN** both bones are 40 cm and the hip-to-target distance is 40 cm
- **THEN** the unsmoothed knee angle is 60°

#### Scenario: Folded too sharp
- **WHEN** both bones are 40 cm and the hip-to-target distance is 20 cm
- **THEN** the unsmoothed knee angle is 30°

#### Scenario: Next tick
- **WHEN** the stored correction and its rate are 0 and the new correction is past 0.02 rad
- **THEN** the pose the muscles follow on that tick is unchanged, and the next tick's correction is 0.02 rad

### Requirement: Thigh turn
While a leg is stepping, its thigh SHALL turn by 75% of the angle from its rest direction to the direction from the hip to the bend target, taken in the character frame. While it is not stepping, that extra turn SHALL lose 10% a tick.

#### Scenario: Three quarters of the way
- **WHEN** that angle is 40° and the leg is stepping
- **THEN** the thigh's extra turn is 30°

#### Scenario: Decay
- **WHEN** the leg is not stepping and the extra turn is 30°
- **THEN** the next tick it is 27°

### Requirement: Step lift
An anchored stepping foot SHALL lift by −(1 − crouch) · f · (1.5 − the stance factor). s SHALL be 0.4 times the horizontal distance to the anchor and no more than 30 cm. r SHALL be D²/(3s)², D the distance to the anchor. f SHALL be ((2 − r)·r·3 − 2)·s when D is at most 3s, and 0.5·s otherwise. An unanchored foot's lift SHALL become 0.8 of itself each tick, and an anchored foot that is not stepping 0.9 of itself.

#### Scenario: A human step of 40 cm
- **WHEN** the horizontal distance and D are both 40 cm, the crouch is 0 and the stance factor is 0.5
- **THEN** the lift is −11.5185 cm

#### Scenario: Decay of a free foot
- **WHEN** an unanchored foot's lift is 10 cm
- **THEN** the next tick it is 8 cm

### Requirement: Bend target
An anchored foot's bend target SHALL be the midpoint of the foot and its anchor, at the anchor's height. An unanchored foot's bend target SHALL be its anchor, lowered by twice the lift term stored before clamping. Standing at the target height, not stepping, that term is 1.125 cm, so the drop is 2.25 cm.

#### Scenario: Anchored midpoint
- **WHEN** the foot is 20 cm from its anchor and the foot is anchored
- **THEN** the bend target is 10 cm from each, at the anchor's height

#### Scenario: Unanchored drop
- **WHEN** the foot is not anchored and the body stands at its target height, not stepping
- **THEN** the bend target is the anchor lowered by 2.25 cm

## MODIFIED Requirements

### Requirement: Climbing with the controller
The lift SHALL read each foot's own ground. The character controller's floor SHALL NOT raise that ground, on level ground, on stairs, perched or fallen.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its controller floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps with its controller, stopping on the third
- **THEN** it keeps its balance at or above 0.9 and its feet end on the landing, and the controller's floor does not raise the pelvis

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` or `--at 7.6,-1.3,66.2` walks the player up the stairs to the door
- **THEN** the four-step synthetic walk is this change's stair gate; a fall on that exanima01 walk is the missing gait probe and is recorded, and the controller floor is not restored

#### Scenario: Perched
- **WHEN** a body's grounded controller stands 12 cm over the floor under its feet, or is in the air
- **THEN** its target is the original's in both cases

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its grounded controller 12 cm over it
- **THEN** its target is the original's every tick, as with its controller on the floor
