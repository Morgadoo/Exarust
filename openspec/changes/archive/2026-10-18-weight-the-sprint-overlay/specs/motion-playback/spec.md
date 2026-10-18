# Spec Delta

## MODIFIED Requirements

### Requirement: Run overlay
While Walk plays and the run blend is above zero, each bone SHALL be the Walk sample slerped toward the Run sample at the same keys, before any pose hold, by 0.7 times the run blend, or by the run blend itself while sprinting. Sprinting SHALL require the sprint latch, a stance level above 0.9 and a speed above 3.5 cm a tick. The root SHALL stay the Walk sample.

#### Scenario: Full run
- **WHEN** the two clips differ by 90° about one axis and the run blend is 1
- **THEN** that bone is 63° from the walk toward the run, and the root is the walk's

#### Scenario: No run
- **WHEN** the run blend is 0
- **THEN** the pose is the walk sample

#### Scenario: Full sprint
- **WHEN** the body is sprinting, the run blend is 1 and the two clips differ by 90° about one axis
- **THEN** that bone is the run's, and the root is the walk's

#### Scenario: Half sprint
- **WHEN** the body is sprinting, the run blend is 0.5 and the two clips differ by 90° about one axis
- **THEN** that bone is 45° from the walk toward the run

#### Scenario: Level at 0.9
- **WHEN** the latch is kept, the stance level is 0.9, the speed is above 3.5 cm a tick and the run blend is 1
- **THEN** that bone is 63° from the walk toward the run

#### Scenario: Speed at 3.5 cm
- **WHEN** the latch is kept, the stance level is above 0.9, the speed is 3.5 cm a tick and the run blend is 1
- **THEN** that bone is 63° from the walk toward the run
