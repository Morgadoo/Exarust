## ADDED Requirements

### Requirement: Motile shock and split
The system SHALL clear the 0x40 shock bit only when the unsigned clock is past the deadline, and SHALL enter the posture block when balance is below 0.6 or the condition is 0x300.

#### Scenario: Shock and split
- **WHEN** the deadline is 1000 and the clock 1000
- **THEN** the bit does not clear
- **WHEN** the clock is 1001
- **THEN** it clears
- **WHEN** balance is 0.59
- **THEN** the posture block is entered

### Requirement: Motile attack arms
The system SHALL compute the attack arm base and length from the posture, clock, size and weapon class, with the stab 0.3 scale and the second arm's `(1−clock)²·p·4`.

#### Scenario: Two clocks
- **WHEN** p is 1, clock 0.1, size 1 and class 0
- **THEN** the length is 1.190995 and the other arm 1
- **WHEN** clock is 0.6
- **THEN** the length is 1.01596 and the other arm 0.64

### Requirement: Motile arm and phase leaves
The system SHALL compute the arm floor, parry ramp, gait clamp, auto-parry deadline, switch phase and get-up scale.

#### Scenario: Leaves
- **WHEN** the turn is 0.5
- **THEN** the arm floor is 0.2
- **WHEN** the parry previous is 0
- **THEN** the ramp is 0.04
- **WHEN** the gait is 6.334
- **THEN** the gait arm is 0.05
- **WHEN** the 0x20 flag is set with ea4 1000
- **THEN** the deadline is 1400
- **WHEN** progress is 0.6
- **THEN** the phase is 0x10004
