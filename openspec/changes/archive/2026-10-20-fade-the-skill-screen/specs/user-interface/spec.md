## ADDED Requirements

### Requirement: Skill screen fade
State byte 4 SHALL return alpha and sound 0. Bit 0 clear SHALL return 1. Bit 0 set with the deadline at or below the clock SHALL clear the bit and return 1. Otherwise `t = (deadline − clock) × 0.002`, `x = (1 − t)(1 + t)`, alpha SHALL be the smaller of x and 1, and sound SHALL be `x(2 − x)`.

#### Scenario: Closing, plain and fading
- **WHEN** the state byte is 4
- **THEN** alpha and sound are 0
- **WHEN** bit 0 is clear
- **THEN** alpha and sound are 1
- **WHEN** bit 0 is set, the deadline 500 and the clock 600
- **THEN** the bit clears and alpha and sound are 1
- **WHEN** bit 0 is set, the deadline 1000 and the clock 800
- **THEN** alpha is 0.84 and sound is 0.9744
