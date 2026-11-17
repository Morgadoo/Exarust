## ADDED Requirements

### Requirement: Placement snaps the AI heading
A living or ground placement SHALL, after writing the facing, run the AI's aim snap from the frame's forward (the AI's look, heading and offsets at its yaw, the body's heading, look and aim directions along it) and stop its route.

#### Scenario: A living placement of an AI body
- **WHEN** an AI body following a route is placed at yaw 1
- **THEN** its look and heading are the placed yaw, its route is stopped and its body look is the frame's forward

### Requirement: Sentry stops and looks ahead
A brain standing sentry out of combat SHALL soft-stop a route its body follows and SHALL set the body's look and aim directions to the frame's forward.

#### Scenario: A running sentry
- **WHEN** a sentry whose body runs along a route ticks
- **THEN** the route ends, the run bit clears and the body looks along its facing
