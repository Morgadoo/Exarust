## ADDED Requirements

### Requirement: Golem stance from the race record
A golem SHALL keep its race record's stance factor, damping ratio, clip rate and pin factor: the character init's golem branch values are written over by the race record before the init ends. Its arm relaxation (0.5, the boss's none) and its powered golem suit SHALL stay.

#### Scenario: A golem guard
- **WHEN** a golem guard (record stance 1, damping ratio 0.95) spawns
- **THEN** its body's stance factor is 1, its arm relaxation 0.5 and its golem suit powered
