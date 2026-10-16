## MODIFIED Requirements

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose: its link muscles turn every joint, and a pose drive pulls the particles of the pelvis, lumbar, thorax, head and feet only, each toward its place in the pose with damping relative to the pose's own motion, at 0.084 per tick (the standing pelvis twice that) times tone² times energy⁴. Thighs, shins, arms and hands SHALL have no pose drive. Gravity is compensated by tone times energy.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with every joint within 15° of the clip's angle, but for the tick its loop wraps, and no limb flails

#### Scenario: Limbs by their muscles
- **WHEN** a living body's arms and legs are aimed away from where they are
- **THEN** the pose drive leaves their particles where they are

### Requirement: Link strength and smoothing
Each link SHALL keep a strength: 1, except a lower leg's first link with over 10 cm of slack, its leg limit, at 0.2 when a living body is built, then following its leg's gait, and 1 once the body goes limp; the femur's first such link SHALL be its leg limit. Each link SHALL keep the smoothing (1 − (r² − 25)·0.001)², at most 0.99 and 0 within √0.1 cm, r being its point's distance from the part's origin in cm.

#### Scenario: Human leg
- **WHEN** a living human body is built
- **THEN** each lower leg's 12.66 cm link pulls at 0.2, each femur's 14.88 cm link is its leg limit, and the femur's point 12.49 cm from its origin has smoothing 0.755

#### Scenario: Gone limp
- **WHEN** the body goes limp
- **THEN** each lower leg's limit link pulls at full strength

## ADDED Requirements

### Requirement: Link muscles
In a living body each slack link SHALL pull its point toward its anchor, where the clip's joint angle puts it on the parent: e = anchor − point, plus the gap's change over the tick × D·k/(0.5|e|² + 1.4), D = (E − 0.5)² (0 below E = 0.5), k = 50 moving, else 40 / stance factor, ×0.6 on the pelvis and legs; then e/(1 + c|e|), c = 0.1, in cm; times strength × part factor × √E·T, the excess beyond the slack added at full weight.

#### Scenario: Elbow swing
- **WHEN** a living arm's clip swings the elbow 40° over 10 ticks and holds it
- **THEN** the elbow is within 5° of the clip 20 ticks after the swing ends, without overshooting it

#### Scenario: Idle and walking damping
- **WHEN** an alert human stands
- **THEN** its link muscles damp with gain 14.3 (8.6 on the pelvis and legs) and pull at √E·T = 0.5, and walking they damp with gain 8.9

#### Scenario: Dazed
- **WHEN** a body's energy falls to a quarter
- **THEN** its link muscles pull at half strength and are not damped

#### Scenario: Momentum
- **WHEN** one pass of a link muscle moves a part and its parent
- **THEN** the body's momentum is unchanged

### Requirement: Muscle gains
Each tick the character tick SHALL set: the stance factor s, the race's standing (human 0.5), rising 0.01 a tick toward 0.7 + 0.3·run while moving; T = tone·s; part factors 0.5, a foot half its lower leg's, the head a quarter of the thorax's, a thigh 0.25 standing and 0.2 moving, shoulders, arms and hands 1.25 × arm stiffness; each leg limit link's strength s ← 0.9s + 0.1t, t = 0.5·S² standing, 0.4 moving.

#### Scenario: Walking firms the body
- **WHEN** a standing human starts walking
- **THEN** its stance factor rises from 0.5 to 0.7 in 20 ticks, reaches 1 running, keeps it while it moves on, and is 0.5 again when it stops

#### Scenario: Leg limit
- **WHEN** a living human stands for one tick
- **THEN** its leg limit links pull at 0.23, settling at 0.5, at 0.4 while it moves and toward 0 without support

### Requirement: Arm stiffness
The arms' stiffness SHALL be 0.5 off balance (falling, lying or getting up), 0.2 at tone under 0.85 or in the walk and run cycles, and otherwise x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), never under min(0.4·|turn left in rad|, 0.4). An arm part's anchor relaxation SHALL be capped at 0.6 − (x − 0.1)·0.8.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

### Requirement: Anchor relaxation
Each tick a part with link muscles SHALL move its relaxation λ toward 0.6 − 32|Δa|² (cm², the most one of its anchors moved since the last tick) × the race's arm relaxation, capped by the part: up by 0.015 a tick, down at once, and not at all for a negative target. A supported lower leg and its thigh SHALL instead take clamp(0.6 − 0.001·| |hip − foot contact|² − (2r)² |, 0, 0.2), r the contact's distance from the knee, rising 0.01.

#### Scenario: Held pose
- **WHEN** an idle arm's clip holds still for 40 ticks
- **THEN** its relaxation rises to its cap, 0.36

#### Scenario: Fast and slow motion
- **WHEN** an arm's anchor then moves 0.84 cm in a tick, and next 0.1 cm
- **THEN** its relaxation stays at 0.36, then falls to 0.28

#### Scenario: Standing leg
- **WHEN** a human stands with its hip 92 cm from its foot contact, 49 cm from the knee
- **THEN** its leg's relaxation is 0

### Requirement: Support until balance
Until balance and the feet are measured, a living body's support S SHALL be 1 while it has not fallen and its character controller stands on the ground (a body without a controller counts as grounded), else 0. The body trace SHALL write S, the stance factor and the tone target.

#### Scenario: Standing and fallen
- **WHEN** `exarust --trace t.csv --shove 60 play arena` runs
- **THEN** the player's support is 1 while it stands, 0 from its fall until it gets up, and while it stands its stance factor and tone target are 0.5
