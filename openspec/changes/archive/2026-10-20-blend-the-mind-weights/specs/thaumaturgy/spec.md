## ADDED Requirements

### Requirement: Mind node weights
The sight's weights SHALL use A = 0.2·v78 and B = 0.2·(v974 + v978 − v7b4); a marked body or A above 1 SHALL force all three weights to 1.

#### Scenario: Marked and ordinary
- **WHEN** the body is marked
- **THEN** threshold, maximum and ratio are all 1
- **WHEN** v78 is 0.5 and v974, v978 are 0.5
- **THEN** the maximum is 0.2 and the ratio one third

### Requirement: Mind threshold and blend
Otherwise A SHALL be floored at 0, B clamped to 0..1, the maximum their larger, the ratio A/(A+B), and the threshold the largest of maximum + 0.2, 0.6·v7c and 0.015·v74 capped at 1. The entry SHALL blend toward them by 2^(−dt/1000), or one half for a step of a second or more.

#### Scenario: Threshold and a half
- **WHEN** the maximum is 0.2 and the other floors are 0
- **THEN** the threshold is 0.4
- **WHEN** an entry at zero blends toward (1, 0.5, 0.25) over a second
- **THEN** its values are 0.5, 0.25 and 0.125
