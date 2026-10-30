# Spec Delta

## ADDED Requirements

### Requirement: Link-iteration ramp
Each living body tick SHALL set a dormant character's link-iteration count to 5, and otherwise raise the count by one toward round((1 − 0.25·feral level)·1660/83), computed in single precision and rounded half to even, or lower it to that target at once.

#### Scenario: A half-feral character
- **WHEN** a living character of feral level 0.5 with a count of 20 is ticked, and then ticked again from 17
- **THEN** its count becomes 18 both times (17.5 rounds half to even)

#### Scenario: A dormant statue
- **WHEN** a dormant character with a count of 20 is ticked
- **THEN** its count is 5
