# Spec Delta

## ADDED Requirements

### Requirement: Sprint thrust
While sprinting, the capsule's forward speed SHALL blend from the walk speed to the jog speed times the feral sprint thrust over the jog thrust, by the run blend. The feral sprint thrust SHALL be 472.2 at feral 0 and the jog thrust 350 at feral 1. Without sprinting the jog end SHALL stay 3.6 m/s.

#### Scenario: Full sprint
- **WHEN** the run blend is 1, the body is sprinting and the feral level is 0
- **THEN** the forward speed is 3.6 × 472.2 / 350 metres per second

#### Scenario: Feral sprint
- **WHEN** the run blend is 1, the body is sprinting and the feral level is 1
- **THEN** the forward speed is 3.6 metres per second

#### Scenario: Not sprinting
- **WHEN** the run blend is 1 and the body is not sprinting
- **THEN** the forward speed is 3.6 metres per second
