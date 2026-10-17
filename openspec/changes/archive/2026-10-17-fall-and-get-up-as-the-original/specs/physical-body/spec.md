## MODIFIED Requirements

### Requirement: Falling and getting up
In its posture stage, from last tick's balance, a living body under 0.6 (double) and not lying still SHALL steady itself upright (lean height over 0.8): the Defence stance (blend 40) still, Rebalance (20) moving; else stagger (below) or fall: Fall Forward when the lean is ahead, else Fall Backward, no blend, when it is sideways (|lean · forward| ≤ 0.4), fast (slow speed² ≥ 8), lost (≤ 0.1) or 80 cm off. Starts are eased fast; a fall leaves the controller.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2` without a hop, and stands where it fell

#### Scenario: From the lying body
- **WHEN** a get-up starts with a part turned 0.4 rad off its clip's pose
- **THEN** after its first tick that part's target turns as the part does

#### Scenario: Leans
- **WHEN** a body at balance 0.45 leans 30° ahead still, then 45° ahead, 45° back, 50° to its side, and 45° ahead with a slow speed² of 8
- **THEN** it takes the Defence stance, staggers with Get up, staggers with Get Up 2, falls backward, and falls forward

#### Scenario: Float thresholds
- **WHEN** the lean's height is the float 0.8, or its forward share the float 0.4 with the balance the float 0.1
- **THEN** the body steadies itself upright, or staggers, as the original's double compares give

#### Scenario: Dazed while falling
- **WHEN** a dazed body falls with its balance between 0.3 and 0.6
- **THEN** its fall's or Rebalance's clip starts again every tick after the daze's Defence stance, until it is down

### Requirement: Character tick stages
Each living body's character tick SHALL run nine stages in the original's order (upkeep, readiness, anchor, behaviour, posture, attack, movement, sampler, carry) and finish them before the next body's tick starts. Its work SHALL sit in them: red regeneration (upkeep); the condition, dying tick, falls, get-ups and fall check (posture); swing, health (attack); balance and ground (movement); the clip (sampler); targets and lift (carry). A body that dies in its posture stage SHALL skip the rest.

#### Scenario: One character after another
- **WHEN** two characters tick
- **THEN** the first runs all nine stages, in order, before the second runs its first

#### Scenario: Who ticks
- **WHEN** a tick runs with the player, a walking character with a controller, a standing body, a limp body and a body whose clips play by frames
- **THEN** the walking character and the standing body tick with their controllers (none for the standing body), the player last, and the limp body and the frame-played body do not tick

#### Scenario: Killed in its posture stage
- **WHEN** a collapsing body's tone reaches 0.25 in its posture stage beside a standing body
- **THEN** it is dead and limp at once and runs no attack, movement, sampler or carry stage, while the standing body runs them all

## ADDED Requirements

### Requirement: Staggers
A stagger SHALL start Get up (lean ahead) or Get Up 2 with a blend of 50 from the whole physical pose and write getting up; when the clip changes its clock SHALL be set to the key before Get up's fourth key (frame 55) or Get Up 2's third (40). Its first sample then drops the blend and the body SHALL hold the clip's first key until its clock reaches it.

#### Scenario: Get Up 2 jumps in
- **WHEN** a body staggers back at full tone
- **THEN** Get Up 2's clock reads −9.25 after its first tick (0.75 frames on it), its first key holds for 27 ticks, and its clip ends into the stance 144 ticks after the start

### Requirement: Getting up from the floor
A body lying still (balance under 0.3, limb motion under 25 cm a tick) SHALL get up once a foot gives support or 100 ticks after support was lost: Get up when its pelvis's front axis height is at most 0 (0.2 while Get up plays, −0.2 while Get Up 2 does), else Get Up 2; past a quarter of the clip an unchanged pick does nothing. It SHALL turn to pelvis → chest (reversed for Get Up 2) when over 18.2° off and start with a blend of 20.

#### Scenario: Waiting
- **WHEN** a body lies still without support for 99 ticks, then 100
- **THEN** it waits, then gets up

#### Scenario: Face up
- **WHEN** a supported body lies still on its back
- **THEN** Get Up 2 starts with a blend of 20 from its whole pose, its condition getting up, on its controller

#### Scenario: Rolling over
- **WHEN** a body 30 % through Get up rolls onto its back, or one 40 % through Get Up 2 rolls onto its front
- **THEN** the first changes to Get Up 2 and re-anchors its frame; the second keeps its clip

### Requirement: Rising energy
While getting up a body's energy SHALL gain 0.01 × (1 − progress) a tick beyond its recovery (by last tick's condition and progress), and its balance 0.05 a tick.

#### Scenario: Halfway
- **WHEN** a body is halfway through Get up at energy 0.5
- **THEN** its readiness stage brings it to 0.505 before the tick's own recovery

### Requirement: Steadying on the feet
A standing body at full tone playing a plain clip (no attack, combat idle, walk, run, kick, weapon switch or power) SHALL take the Defence stance (no blend) under balance 0.8 (double) with a fast chest speed² of at most 0.3, else Rebalance (blend 20); weak (tone under 0.85), the Defence stance. A Rebalance SHALL hold the posture switch off for 41 ticks off balance, 27 unsteady.

#### Scenario: Unsteady
- **WHEN** a standing body has balance 0.79 and a fast chest speed² of 0.29, then 0.31, then balance the float 0.8
- **THEN** it takes the Defence stance, then Rebalance, then neither

#### Scenario: Shoved
- **WHEN** `exarust --fixed-frames --shove 60 play arena` dazes the player
- **THEN** it Rebalances at balance 0.61 before it falls

### Requirement: Turning toward a fall
Under balance 0.8 (double) and not getting up, a body whose pelvis's front axis has turned more than 45° from its clip's SHALL turn its heading across the ground to chest → pelvis (front axis height over 0.4), pelvis → chest (under −0.4) or the front axis, when that is more than 37° off.

#### Scenario: Twisted
- **WHEN** a body's pelvis faces up while its clip's faces ahead, with its heading 90° off its fall
- **THEN** its heading turns to the direction from its chest to its pelvis
