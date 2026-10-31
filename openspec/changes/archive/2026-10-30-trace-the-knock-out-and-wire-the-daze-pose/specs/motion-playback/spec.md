# Spec Delta

## ADDED Requirements

### Requirement: Shock daze pose
Every body tick that a body's shock-stagger flag is set, including the tick its timer clears the flag, the pose the living sampler holds SHALL be the daze pose: skin bones 1 to 7 take the capture of the clip start (the Rebalance's) unless the condition has bit 0x200, and every other bone takes a turn of a random axis by a random angle (four engine draws a bone) blended half way from the Defence stance's first key. A body whose support is under 1 SHALL hold its physical pose instead.

#### Scenario: Shocked on its feet
- **WHEN** a shock-staggered body with support 1 is sampled
- **THEN** its pelvis keeps the Rebalance start's captured turn and its upper body takes the scattered turns

#### Scenario: Shocked while down
- **WHEN** the shock-staggered body's condition is down
- **THEN** its pelvis takes a scattered turn too
