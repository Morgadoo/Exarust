# Spec Delta

## ADDED Requirements

### Requirement: Muscles-off sampler gate
The muscles-off sampler SHALL have no caller: a dead body's tick ends before any sampling, and a body whose muscles are off and which is not dead samples nothing, its clip clock and clip-end test left alone, as the original's body tick skips them for a body that did not locomote (its movement decision needs muscles).

#### Scenario: A knocked-out corpse
- **WHEN** a dead body is knocked out without a revive
- **THEN** it stays a ragdoll and its clip is neither advanced nor sampled

### Requirement: Revived balance
A revive's drive SHALL start from the balance the body's last living tick measured.

#### Scenario: A body revived where it died
- **WHEN** a body dies with balance 0.2 and is revived
- **THEN** its first posture decision reads 0.2 and keeps it down
