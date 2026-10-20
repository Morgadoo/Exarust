## ADDED Requirements

### Requirement: Motile posture
The system SHALL compute lift as `0.9 + 0.09·lean − 0.6·(1−sag_b) − 0.1·(1−sag_a)` clamped at 0, and compliance as `1 − 0.01·lean + 0.05·(1−sag_a) + 0.30·(1−sag_b)` clamped at 1, with the second operand on NaN.

#### Scenario: Two postures
- **WHEN** lean, sag_a and sag_b are all 1
- **THEN** lift and compliance are 0.99
- **WHEN** lean is 0.4, sag_a 0.5 and sag_b 0
- **THEN** lift is 0.286 and compliance 1

### Requirement: Motile follow
The system SHALL advance the follow point by 0.6 of the target delta plus the velocity gain (Y a tenth), with gain 0.0125 or +0.0025 while active up to 0.15.

#### Scenario: One step
- **WHEN** the point is (0.1, 1, −0.2), the velocity (0.03, 0, 0), the target (0.4, 1, −0.2) and the gain 0.0125
- **THEN** the point becomes (0.280225, 1, −0.2)

### Requirement: Motile ramps and tone
The system SHALL ramp posture by 0.004 to 1, decay support by 0.02 to 0, and compute the tone weight from the warm blend or the flagged/unflagged error and scale paths.

#### Scenario: Tone paths
- **WHEN** the error is 0.42 in a warm state
- **THEN** the weight is 0.71
- **WHEN** the error is 0.42, unflagged and not warm
- **THEN** the weight is 0.1764
- **WHEN** the error is 0.42, flagged with scale 1.5 and state 0
- **THEN** the weight is 0.171108

### Requirement: Motile cadence and angles
The system SHALL compute cadence as `trunc((1 − 0.25·exertion)·1660/83)`, apply the pitch impulse with a 0.95 decay, wrap angles with 2π and test impacts as `threshold·scale < |v|²`.

#### Scenario: Cadence and wrap
- **WHEN** exertion is 0.5
- **THEN** the cadence target is 17
- **WHEN** the angle is 3.5
- **THEN** it wraps to −2.7831855
- **WHEN** the threshold is 2, scale 1 and squared impact 3
- **THEN** the impact crosses
