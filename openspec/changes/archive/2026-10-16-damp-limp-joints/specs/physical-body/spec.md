## MODIFIED Requirements

### Requirement: Limp bodies
The system SHALL simulate a limp body as the original does. Each part is a rigid tetrahedron of four particles laid over its mesh, integrated at 83 Hz under gravity, and joined to its parent at its origin and by the skeleton's links. A limp pivot (slack at most 1.19e-7 cm) SHALL close strength/(1 + 0.001|d|²) of its gap d in cm; a slack link SHALL damp its length inside the slack and take back strength times its excess beyond it.

#### Scenario: Body at rest
- **WHEN** a body goes limp above a floor
- **THEN** its parts fall, come to rest on the floor, every slack link stays within its slack and every part stays within a centimetre of its socket

## ADDED Requirements

### Requirement: Limp joint damping
Inside its slack a limp link SHALL update its smoothed length L ← s·L + (1 − s)·|d|, s being its smoothing and |d| its length, and take its points back to the L they had by strength·(|d| − L)/|d| of the gap. Beyond the slack L SHALL be the slack. L SHALL start at the posed distance when the body is built and at the distance, at most the slack, when the body goes limp.

#### Scenario: One tick of a damped link
- **WHEN** a limp link with smoothing 0.755 inside its slack is stretched 1 cm in a tick and each of the 20 passes closes its correction
- **THEN** 0.20 cm of the stretch is left at the end of the tick

#### Scenario: Swinging leg
- **WHEN** a limp leg swings at its hip inside the range of its links
- **THEN** its swing falls below a fifth within ten ticks, while a leg held by its hip alone keeps over four fifths

#### Scenario: Joint at rest
- **WHEN** a body with a joint bent inside its range and at rest goes limp
- **THEN** the joint stays where it is

#### Scenario: Stretched past the slack
- **WHEN** a body goes limp with a link stretched past its slack
- **THEN** that link's smoothed length is its slack

### Requirement: Velocity retention
A limp part SHALL keep its velocity as the original's default environment gives it: 0.426, 0.25 or 0.05 per second (0.98977, 0.98344 or 0.96455 per tick) when the farthest of its particles moved over 40/83 cm, over 20/83 cm, or less in the last tick. A living body's limbs SHALL keep the fast retention times u², u being 1 for a human, whatever their speed.

#### Scenario: Limp bands
- **WHEN** a limp part's farthest particle moved 0.49 cm, exactly 40/83 cm, or exactly 20/83 cm in the last tick
- **THEN** it keeps 0.98977, 0.98344 or 0.96455 of its velocity this tick

#### Scenario: Living limbs
- **WHEN** a living human body's part drifts at 6 cm/s
- **THEN** it keeps 0.98977 of its velocity per tick
