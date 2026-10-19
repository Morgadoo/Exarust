# Spec Delta

## ADDED Requirements

### Requirement: Door motor
A toggle or an open command SHALL aim at (1 − fraction) times the range times the sign, plus the closed angle. A plain step SHALL be the absolute error over the range, times 0.001, plus 0.0003. A motorised door SHALL step 0.0003, and a fast one 0.005. The drive SHALL move by that step toward the aim. A toggle SHALL arrive within its step, and an open command within 0.1 rad. A countdown below 2 SHALL end the command after this tick's drive.

#### Scenario: A plain step and an arrival
- **WHEN** a plain toggle aims 1 rad open from a shut angle of 0, a fast motorised toggle aims the same way, and an open command is 0.05 rad short of that aim
- **THEN** the drives are 0.0013 and 0.005 toward the aim, and the open command arrives on the aim and ends
