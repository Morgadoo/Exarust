# Spec Delta

## ADDED Requirements

### Requirement: Character follows its body
A living character SHALL have no capsule and no swept controller: each fixed step it SHALL go at once, across the ground, to where its body's frame origin puts the body root, and stand at the character's ground its body measured that tick. Without support it SHALL go down with its pelvis's drop that tick, never under that ground; a ground more than 3 m under the pelvis, or above it, SHALL be ignored. The carry SHALL leave the follow out; walls, props and bodies SHALL block the body's parts.

#### Scenario: Follow
- **WHEN** the frame's origin is 50 cm across the ground from the feet and the body measured its ground 18 cm up
- **THEN** the feet stand there, 18 cm up, after one step, through any collider between, and the body is not carried by that move

#### Scenario: Sewer
- **WHEN** `exarust grid` runs `play exanima05_sw` on the round 62 build and on this one side by side
- **THEN** the fixed tick no longer spends its time on character sweeps (about a tenth of the round 62 tick in the same grid)

### Requirement: Character without muscles
A character without muscles SHALL walk by its desired velocity and stand on the level under its feet, falling where there is none.

#### Scenario: Without muscles
- **WHEN** a character without muscles walks off a floor onto a 20 cm ledge and off its far edge
- **THEN** it stands on the floor, then on the ledge, then falls

## MODIFIED Requirements

### Requirement: Support
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle from 70 cm above it (from it, down or getting up), walking with the gait probe, else with a sphere of 5 cm + 10 cm × an anchored foot's share. Ground over half the leg under the pelvis is out of reach, but getting up. S SHALL be grounded × balance; a body that does not locomote and has no grounded foot SHALL lose its balance, getting up included, before the get-up's support near the ground.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** every row has the player's support and balance at 1 and its counter at 80

#### Scenario: No ground
- **WHEN** a standing body has no ground under its feet
- **THEN** its balance and support are 0 and it counts the ticks it is airborne; a moving body keeps its balance

#### Scenario: Out of reach
- **WHEN** a 65 cm block stands under one foot of a standing human
- **THEN** that foot has no ground, the other is grounded, and its support is 1

#### Scenario: Getting up near the ground
- **WHEN** a body getting up has no foot grounded and its pelvis 20 cm over the ground
- **THEN** it has support, at half its balance, which the feet routine has set to 0, so S is 0 until a foot is grounded

#### Scenario: Down
- **WHEN** a body lying or getting up has a shelf 40 cm over its ankle, and a 5 cm step's edge 7 cm beside its probe
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the floor, and a walking body the shelf and the step's edge (2.14 cm)

#### Scenario: Walking gait probe
- **WHEN** a walking body's clip progress puts its first foot at the gait bump's peak, at scale 1 and a speed of 0
- **THEN** that foot's sphere is 20 cm and its probe is 36 cm further along the facing than the 5 cm offset

### Requirement: Climbing with the controller
The lift SHALL read each foot's own ground. The character's floor SHALL NOT raise that ground, on level ground, on stairs or fallen.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its character floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps, stopping on the third
- **THEN** its feet stay on the floor and its pelvis stops short of the first riser, its balance stays at 1

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` walks the player up the stairs to the door
- **THEN** it climbs onto the landing at the door, its balance and support 1 throughout

#### Scenario: Perched
- **WHEN** a body's character stands 12 cm over the floor under its feet
- **THEN** its target is the original's

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its character 12 cm over it
- **THEN** its target is the original's every tick, as with its character on the floor

## REMOVED Requirements

### Requirement: Controller follows the frame
**Reason**: The original has no capsule; the character goes to its frame at once.
**Migration**: See "Character follows its body".
