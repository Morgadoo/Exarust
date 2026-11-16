## ADDED Requirements

### Requirement: Locomotion head tilt
A locomoting body with support SHALL set its head tilt to the sign of its exertion × its square × 2 from the exertion the routine has just eased, after the run blend, so the twist after it reads this tick's tilt.

#### Scenario: A supported walk
- **WHEN** a body of exertion 0.5 walks one supported tick
- **THEN** its exertion is 0.495 and its tilt 0.495² × 2

### Requirement: Get-up tick ends the step
A body getting up (condition exactly 0x300) SHALL end any step on each balance tick (the stepping and balance-step flags and the long kind clear) and SHALL clear the posture switch's wait.

#### Scenario: Getting up mid-step
- **WHEN** a stepping body with a posture wait of 30 ticks takes a get-up balance tick
- **THEN** it no longer steps and its posture wait is 0
