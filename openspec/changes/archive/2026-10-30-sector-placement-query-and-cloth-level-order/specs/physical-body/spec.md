# Spec Delta

## ADDED Requirements

### Requirement: Placement overlap query gates
The placement overlap query SHALL offer a probe body each body bucketed in the nine 600 cm cells around the probe's cell whose mesh sphere touches the probe's bound sphere, whose oriented mesh box lies within the probe's sphere radius of the probe's sphere centre, and whose sphere centre lies within its own radius of the probe's oriented mesh box. Each box gate SHALL skip only when the radius squared is at most the distance squared.

#### Scenario: Box gate
- **WHEN** a probe with a 10 cm sphere and a 5 cm cube at the origin meets a body of the same shape whose box centre lies 15 cm away
- **THEN** it is not offered, and at 14 cm it is offered

#### Scenario: Turned body
- **WHEN** the body at 15 cm is turned 45° about the vertical
- **THEN** its box reaches 7.07 cm towards the probe and it is offered

#### Scenario: NaN distance
- **WHEN** a body's sphere centre is NaN
- **THEN** the box gates let it through

### Requirement: Placement overlap query filters
The placement overlap query SHALL skip a body of the probe's root when the group words share a bit, a body of another root when the layer words name a shared exclusive channel, and a disabled body always, and SHALL put the side with the lower pair key first in each offered pair.

#### Scenario: Disabled body
- **WHEN** an otherwise offered body is disabled
- **THEN** it is not offered

#### Scenario: Pair order
- **WHEN** the probe's key is 10 and two offered bodies have keys 20 and 5
- **THEN** only the body with key 5 goes first
