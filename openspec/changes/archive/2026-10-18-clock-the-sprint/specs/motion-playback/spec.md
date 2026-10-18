# Spec Delta

## ADDED Requirements

### Requirement: Sprint clock
While sprinting, a forward gait's clock SHALL blend from the walk rate toward the feral sprint row by the square root of the run blend, then multiply by 1 − 0.1 times the encumbrance. The feral sprint row SHALL be the sprint rate blended to the jog rate by the feral level. Without sprinting it SHALL keep the jog row and skip that factor.

#### Scenario: Unencumbered sprint
- **WHEN** a human sprints at feral level 0, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.727 frames a tick

#### Scenario: Feral sprint
- **WHEN** a human sprints at feral level 1, with no encumbrance, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.894 frames a tick

#### Scenario: Sprint in armour
- **WHEN** a human sprints at feral level 0, encumbrance 0.5, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.691 frames a tick

#### Scenario: Level at 0.9
- **WHEN** the latch is kept, the stance level is 0.9, the run blend is 1 and the speed is 4.337 cm a tick
- **THEN** its clock gains 0.894 frames a tick

#### Scenario: Speed at 3.5 cm
- **WHEN** the latch is kept, the stance level is above 0.9, the run blend is 1 and the speed is 3.5 cm a tick
- **THEN** its clock gains 0.76 frames a tick
