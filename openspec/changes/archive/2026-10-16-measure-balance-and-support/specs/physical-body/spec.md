## MODIFIED Requirements

### Requirement: Balance
Each tick a living body SHALL score its lean: q = u_y⁸ standing, u_y⁴ moving, plus 0.05 × the race's balance bonus, at most 1, u being the unit offset from its support point to its chest. While q ≤ 0.99 an instability counter SHALL grow by 10 − 10q rounded half to even; else it falls by 5 to no less than 80 standing still, or by 10 to 0. At 100 balance SHALL be q; below, it recovers by (5 − v)·0.01 a tick at a slow chest speed v under 5 cm/tick.

#### Scenario: Standing still
- **WHEN** an alert body stands upright on the ground
- **THEN** it keeps its balance indefinitely: balance 1, its counter at 80 from its first tick

#### Scenario: Lean
- **WHEN** a human's chest leans 44 cm off its support point, 130 cm above it
- **THEN** it scores 0.698, and under 0.6 only past 52.2 cm

#### Scenario: Second wobble
- **WHEN** a lean scoring 0.55 comes to a fresh body, and to one that has stood steady
- **THEN** the fresh body loses its balance after 25 ticks, the other after 5

#### Scenario: Rounding
- **WHEN** the score is 0.95, 0.75 or 0.55
- **THEN** the counter grows by 0, 2 or 4

### Requirement: Falling and getting up
The system SHALL make a standing body whose balance was below 0.6 last tick fall: it loses its controller's support, its tone drops to 0.5, it plays Fall Forward when its lean points ahead of it and Fall Backward otherwise, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, placed under the body, its controller settling on the ground there.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2`, and stands where it fell

### Requirement: Arm stiffness
The arms' stiffness SHALL be 0.5 off balance (balance under 0.6, falling, lying or getting up), 0.2 at tone under 0.85 or in the walk and run cycles, and otherwise x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), never under min(0.4·|turn left in rad|, 0.4). An arm part's anchor relaxation SHALL be capped at 0.6 − (x − 0.1)·0.8.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

### Requirement: Anchor relaxation
Each tick a part with link muscles SHALL move its relaxation λ toward 0.6 − 32|Δa|² (cm², the most one of its anchors moved since the last tick) × the race's arm relaxation, capped by the part: up by 0.015 a tick, down at once, and not at all for a negative target. A supported lower leg and its thigh SHALL instead take clamp(0.6 − 0.001·| |hip − foot contact|² − (2r)² |, 0, 0.2), r the contact's (the ankle's) distance from the knee, rising 0.01.

#### Scenario: Held pose
- **WHEN** an idle arm's clip holds still for 40 ticks
- **THEN** its relaxation rises to its cap, 0.36

#### Scenario: Fast and slow motion
- **WHEN** an arm's anchor then moves 0.84 cm in a tick, and next 0.1 cm
- **THEN** its relaxation stays at 0.36, then falls to 0.28

#### Scenario: Standing leg
- **WHEN** a human stands with its hip 85 cm from its ankle, 42 cm from the knee
- **THEN** its leg's relaxation is 0.2

## ADDED Requirements

### Requirement: Balance point
Each tick a living body SHALL take its chest, the mean of its thorax pins, and smooth the chest's last motion by 0.4 (fast) and 0.1 (slow). The support point SHALL be the closest point to the chest on the segment between its foot contacts, the ankles (parameter taken horizontally), moved halfway to their centre standing. A landing SHALL lead a standing chest by the slow velocity × 5, fading 0.05 a tick; moving with support, by 3 × the fast velocity; a crouch takes it 10 cm back.

#### Scenario: Chest and ankles
- **WHEN** a human body is built from the installed skeleton and model
- **THEN** its chest is the mean of its shoulder pins, (0, 9.65, 1.25) cm in the thorax, and each leg stands on its foot's origin, 41.95 cm under the knee, its leg 85.15 cm long

#### Scenario: Landing
- **WHEN** a body regains the ground
- **THEN** its landing weight is 5, and 4.95 the next tick

#### Scenario: Smoothed chest
- **WHEN** a standing body's chest moves 1 cm in a tick
- **THEN** a tick later its fast velocity is 0.4 cm and its slow velocity 0.1 cm a tick

### Requirement: Vertical chest speed
A slow vertical chest speed v over 3 cm/tick SHALL set the instability counter to 100 and, for a body grounded last tick, cut its score by (1 − 0.333(v − 3))², to 0 above 6 cm/tick.

#### Scenario: Sinking
- **WHEN** a standing body sinks 5 cm a tick
- **THEN** on the tenth tick its slow vertical speed passes 3 cm/tick, its counter is 100 and its balance the cut score

### Requirement: Height Query
The level SHALL answer the original's Height Query: a sphere whose lowest point starts at a given point is swept straight down at most a given depth, and the answer is the height of its lowest point where it first meets the front of an upward face, or where it starts if it starts against one. Living bodies are not part of the level.

#### Scenario: Floor and step
- **WHEN** a 10 cm sphere drops from 70 cm onto a floor, and beside a 30 cm step whose edge is 5 cm from its centre line
- **THEN** it finds the floor, and 28.66 cm on the step's edge

#### Scenario: Faces turned away
- **WHEN** a face turned down lies over the floor, and when the sphere starts behind the floor
- **THEN** the first is passed through, and nothing is found

### Requirement: Support
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle, a 10 cm sphere from 70 cm above it (a 5 cm one from it while down or getting up). Ground over half the leg under the pelvis is out of reach, except getting up. A foot is grounded within 20 cm (30 moving) of its ground plus its sole. S SHALL be grounded × balance; a standing body without a grounded foot loses its balance unless its controller stands on the ground. The trace SHALL write S, balance and counter.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** every row has the player's support and balance at 1 and its counter at 80

#### Scenario: No ground
- **WHEN** a standing body has no ground under its feet
- **THEN** its balance and support are 0 and it counts the ticks it is airborne; a moving body keeps its balance

#### Scenario: Perched
- **WHEN** a standing body's controller stands on a stone its feet hang beside
- **THEN** it keeps its balance, with support 0

#### Scenario: Out of reach
- **WHEN** a 65 cm block stands under one foot of a standing human
- **THEN** that foot has no ground, the other is grounded, and its support is 1

#### Scenario: Getting up near the ground
- **WHEN** a body getting up has no foot grounded and its pelvis 20 cm over the ground
- **THEN** its support is half its balance, which rises 0.05 a tick

#### Scenario: Down
- **WHEN** a body lying or getting up has a shelf 40 cm over its ankle, and a 5 cm step's edge 7 cm beside its probe
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the step's edge

## REMOVED Requirements

### Requirement: Support until balance
**Reason**: Support is now measured from the feet and the balance, as the original measures it.
**Migration**: See Support, Balance and Balance point.
