## MODIFIED Requirements

### Requirement: Pose hold
A living body's sampled turns SHALL be pulled toward the pose it holds (its pelvis's and legs' targets, its other parts' physical turns, taken while it is dazed, its blow sag on or its support under 1) by w = 1 − (1 − S)²·(1 − s)·(E/Emax)⁴ while dazed (energy more than 0.05 under its most, in double precision), else (1 − S)·s, when w is over 1.19e-7. The sag SHALL fade 0.01 a tick and be 0 once the body falls.

#### Scenario: Dazed on its feet
- **WHEN** a body standing with support 1 is dazed to half its energy
- **THEN** its targets are its physical pose until its energy is back over 0.95

#### Scenario: Dazed lying
- **WHEN** a lying body (support 0) has half its energy
- **THEN** its turns are pulled 0.9375 of the way to its physical pose

#### Scenario: Alert
- **WHEN** an alert body has no blow sag
- **THEN** its targets are its clip's pose

#### Scenario: Threshold
- **WHEN** a body's energy is 0.95 as a float, a hair under the double 0.95, or the next float up
- **THEN** it is dazed, or it is not
