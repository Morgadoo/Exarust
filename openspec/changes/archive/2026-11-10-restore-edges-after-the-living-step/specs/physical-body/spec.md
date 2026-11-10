## ADDED Requirements

### Requirement: Living step restores the edges
At the end of a living body's step, after every part's inertia, carry and pose drive and before the iterations, each part's six edges SHALL be restored once, as the tree's root does when it steps last.

#### Scenario: Stretched part
- **WHEN** a living arm whose particles moved apart over the last tick is stepped
- **THEN** its parts' edges are under half as far from their rest lengths as a limp arm's after the same step

## MODIFIED Requirements

### Requirement: Link muscles
In a living body each slack link SHALL pull its point toward its anchor, where the clip's joint angle puts it on the parent: e = anchor − point, plus the gap's change over the tick × D·k/(0.5|e|² + 1.4), D = (E − 0.5)², or 0 when E − 0.5 compares below 0 (a NaN stays), k = 50 moving, else 40 / stance factor, the division in double and ×0.6 (in double, before D) on the pelvis and legs; then e·r·RCPSS(c + r), r = RSQRTSS(|e|²), about e/(1 + c|e|), c = 0.1, in cm; times strength × part factor × √E·T (a NaN or negative root stays NaN), the excess beyond the slack added at full weight.

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

#### Scenario: Processor approximations
- **WHEN** a link muscle saturates an error of (3, −4, 12) cm
- **THEN** it is scaled by r·RCPSS(0.1 + r), r = RSQRTSS(169), within 0.1% of e/(1 + 0.1·13)

### Requirement: Link strength and smoothing
Each link SHALL keep a strength: 1, except a lower leg's first link with over 10 cm of slack, its leg limit, at 0.2 when a living body is built, then following its leg's gait, and 1 once the body goes limp; the femur's first such link SHALL be its leg limit. Each link SHALL keep the smoothing (1 − (r² − 25)·0.001)² in double, rounded once, 0.99 when it compares over the double 0.99, and 0 when r² compares at most the double 0.1 or is not a number, r² being its point's squared distance from the part's origin in cm, summed (x² + y²) + z².

#### Scenario: Human leg
- **WHEN** a living human body is built
- **THEN** each lower leg's 12.66 cm link pulls at 0.2, each femur's 14.88 cm link is its leg limit, and the femur's point 12.49 cm from its origin has smoothing 0.755

#### Scenario: Gone limp
- **WHEN** the body goes limp
- **THEN** each lower leg's limit link pulls at full strength
