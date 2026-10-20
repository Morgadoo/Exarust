## ADDED Requirements

### Requirement: Motile time warps
The system SHALL warp time as `t²(3−2t)` in mode 1 and `0.11t + 1 − 1/(1+8t)` in mode 2, with the reciprocal in single precision.

#### Scenario: Quarter and full
- **WHEN** mode 1 sees 0.25
- **THEN** the warp is 0.15625
- **WHEN** mode 2 sees 1.0
- **THEN** the warp is 0.9988889

### Requirement: Motile clock
The system SHALL advance the spent time, budget and host accumulator by dt, scaled to 0.75 while the tired flag is set.

#### Scenario: Tired
- **WHEN** the tired flag is set and dt is 1
- **THEN** the applied dt and all three accumulators are 0.75

### Requirement: Motile reservoirs
The system SHALL drain the capacity by `rate·spent` and the charge by `rate·drain`, zeroing the level; without a drain it SHALL refill by `0.1·√capacity·rate`, with `rate = (1 or 1.5)/3320 + bonus`.

#### Scenario: A draining and a refilling tick
- **WHEN** the charge is 1, capacity 3, rate 0.5, drain 1, spent 2 and spent rate 0.5
- **THEN** the capacity is 1.998 and the charge 0.499
- **WHEN** the charge is 0, capacity 4, rate 1, drain 0 and the slow flag is set
- **THEN** the rate is 0.99 and the charge is 9.036e-05

### Requirement: Motile inverse mass
The system SHALL distribute inverse masses from the 18-entry segment table with scale `n/((20·m₀ + Σ1/m)·strength)` and mobility `0.175·inverse mass`.

#### Scenario: Four parts
- **WHEN** the root inverse mass is 2, the mass scale 0.5 and the parts have bones 0–3
- **THEN** the scale is 0.691358 and the inverse masses are 0.311111, 0.483951, 0.691358 and 1.935802

### Requirement: Motile friction and action gates
The system SHALL use friction 0.2/0.4 in mode 0, 0.6/0.4 in mode 1 and the base/awareness formula otherwise, smoothing contacts as `0.9c + 0.1t` or `0.8c + 0.04`. A body may act without the `0x82C1` mask, with grip at least 0.7, awareness above 0.7 and no blocking state; readiness SHALL use a 40-tick cooldown and 0.9/0.99 clarity.

#### Scenario: Modes and gates
- **WHEN** the mode is 3 with base 0.5 and awareness 0.8
- **THEN** the friction is 0.1 and the target 0.32
- **WHEN** a clear body has clarity 0.995 and its 20-counter is 21
- **THEN** it is not ready
