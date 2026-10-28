## MODIFIED Requirements

### Requirement: Leg bend
A leg SHALL bend onto its bend target. The knee SHALL be the law of cosines on the two bones: 30° over the double 0.866, 180° at or below −1, else the engine's arc cosine. The correction from the clip SHALL advance at most 0.02 rad a tick (0.2 while stepping) plus its stored rate, never past the target. The rate SHALL become half that sum, within ±0.1 rad. Muscles SHALL follow the correction on the next tick, not the tick that computed it. Lying down it SHALL release instead.

#### Scenario: Folded to 40 cm
- **WHEN** both bones are 40 cm and the hip-to-target distance is 40 cm
- **THEN** the unsmoothed knee angle is 60°

#### Scenario: Folded too sharp
- **WHEN** both bones are 40 cm and the hip-to-target distance is 20 cm
- **THEN** the unsmoothed knee angle is 30°

#### Scenario: Next tick
- **WHEN** the stored correction and its rate are 0 and the new correction is past 0.02 rad
- **THEN** the pose the muscles follow on that tick is unchanged, and the next tick's correction is 0.02 rad

#### Scenario: Lying down
- **WHEN** the body is down, dormant or dead, not getting up, and a stored knee of 0.2 rad and a hip of −0.2 rad have a zero rate
- **THEN** the next stored knee is 0.18 rad and the hip is −0.18 rad, each rate half of that 0.02 rad step, and the bend target, the step lift and the thigh's extra turn are unchanged

## ADDED Requirements

### Requirement: Locomotion leg IK gate
While a body locomotes, each leg SHALL act only when the clip's hip height over its ankle exceeds the physical hip's height over the anchor (plus the leg's lag), or the comparison is unordered, or the foot touches and the body does not run. A leg that does not act, or whose target has no length, SHALL ease its corrections toward 0 and show the identity. Both corrections SHALL ease at 0.04 × the gait speed (cm a tick) + 0.01, in double.

#### Scenario: A running foot over low ground
- **WHEN** a running body's touching foot stands where the clip's leg is shorter than the hip's height over the anchor
- **THEN** the leg does not act and shows the identity while its stored angles ease toward 0

### Requirement: Locomotion leg IK angles
An acting leg's target SHALL run from the physical hip across the ground to the contact and down to the anchor, its squared length capped at the whole leg's. Its knee correction SHALL be the clip triangle's knee less the target's. Its hip correction SHALL be the angle between the clip's leg and the target leg, negated unless the clip's foot lies ahead of the character frame's origin, less the target triangle's hip angle, plus the clip triangle's.

#### Scenario: A short clip leg
- **WHEN** a planted leg whose hip stands 1 m over its anchor plays a clip whose ankle is 30 cm under its hip, the rate 0.01
- **THEN** the knee correction moves 0.01 rad toward −2.533 and the hip correction 0.01 rad toward +1.266

### Requirement: Jolt
Each tick a body on its feet SHALL take its jolt: the chest's fast velocity less its slow one, across the ground, less a decaying term that first loses 15 %. Past 2.5·scale cm a tick in the walking condition, 125·scale in any other, a thousand times either without support, its step SHALL end; once per episode its standing position SHALL move to the feet's centre plus 100 × the jolt, at most 100·scale cm away, and the behaviour SHALL be told while it locomotes.

#### Scenario: A walking jolt
- **WHEN** a walking body's fast chest velocity is 3 cm a tick over its slow one with its feet's centre at (100, 0, 200) cm
- **THEN** its step ends and its standing position moves to (200, 0, 200) cm

### Requirement: Jolt latch
Every jolt past the threshold SHALL hold the episode's latch for 80 ticks; a latched jolt SHALL still end the step and move nothing. The latch SHALL clear on the quiet tick after the countdown reaches 0. A body off its feet SHALL take no jolt and lose its latch.

#### Scenario: Clearing the latch
- **WHEN** a jolt sets the latch and the chest then stays quiet
- **THEN** the latch clears on the 81st quiet tick

### Requirement: Walking footstep sound
A walking foot's plant on a body whose race has a footstep set SHALL sound that set at the character's place plus 3·speed + 4 cm along its facing, with pitch (0.2·p + 0.9)(0.05·speed + 0.85) and gain (0.5·g + 0.5)((0.08·speed)² + 0.01), p and g the engine's unit draw taken in that order, the draw's share in single and the rest in double.

#### Scenario: A walk's step
- **WHEN** a foot plants at 4.3 cm a tick with draws 0 and 0
- **THEN** the pitch is 0.9585 and the gain 0.064168

### Requirement: Travel phase
Each post-step a body SHALL add the single-precision root of its pelvis's squared travel (cm²) times the double 0.0002 to its travel phase when that squared travel is over the engine's epsilon, and a phase over 1 SHALL lose 1, the phase before the tick too.

#### Scenario: A metre of travel
- **WHEN** the pelvis travels 100 cm in a tick from phase 0
- **THEN** the phase is 0.02
