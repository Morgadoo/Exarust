# Spec Delta

## ADDED Requirements

### Requirement: Run overlay
While Walk plays and the run blend is above zero, each bone SHALL be the Walk sample slerped toward the Run sample at the same keys by 0.7 times the run blend, before any pose hold. The root SHALL stay the Walk sample.

#### Scenario: Full run
- **WHEN** the two clips differ by 90° about one axis and the run blend is 1
- **THEN** that bone is 63° from the walk toward the run, and the root is the walk's

#### Scenario: No run
- **WHEN** the run blend is 0
- **THEN** the pose is the walk sample
