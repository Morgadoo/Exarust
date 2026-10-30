# Spec Delta

## MODIFIED Requirements

### Requirement: Motile cadence and angles
The system SHALL compute cadence as `(1 − 0.25·exertion)·1660/83` in single precision converted to a 64-bit integer with ties to even and narrowed to its low 32 bits, apply the pitch impulse as `sign(v)·(v·v)·2` with the engine's sign followed by a 0.95 decay, wrap angles with 2π and test impacts as `threshold·scale < |v|²`.

#### Scenario: Cadence and wrap
- **WHEN** exertion is 0.5
- **THEN** the cadence target is 18
- **WHEN** exertion is NaN
- **THEN** the cadence target is 0
- **WHEN** the angle is 3.5
- **THEN** it wraps to −2.7831855
- **WHEN** the threshold is 2, scale 1 and squared impact 3
- **THEN** the impact crosses

## ADDED Requirements

### Requirement: Step end foot choice
When a recovery step ends, the system SHALL plant the foot whose posed spot, offset by the chest plus five ticks of its fast velocity, lies horizontally nearest its current position, passing over feet anchored and not reached, keeping the earlier foot on ties, never choosing a NaN distance, and planting the first leg when none wins.

#### Scenario: Anchored foot passed over
- **WHEN** leg 0 is anchored and not reached and leg 1 is free
- **THEN** leg 1 is set touching with its ramp bits kept and its anchor at its position

#### Scenario: Reached foot is a candidate
- **WHEN** leg 0 is anchored and reached and leg 1 is anchored only
- **THEN** leg 0 is set touching and its anchor is left alone

#### Scenario: No candidate
- **WHEN** every leg is anchored and not reached
- **THEN** the first leg is set touching
