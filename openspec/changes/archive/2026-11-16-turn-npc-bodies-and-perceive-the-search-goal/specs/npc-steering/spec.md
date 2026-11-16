## ADDED Requirements

### Requirement: The heading block after the state
After the AI tick's state, a present character on its locomotion whose route is not followed SHALL run the heading block: a stepping body, or one in the attacking condition, SHALL take its look direction as its look smoothing and its heading; otherwise a body with its in-combat mark SHALL run the look snap; otherwise the out-of-combat look chain SHALL be the block.

#### Scenario: An attacking body
- **WHEN** a body in the attacking condition looks along 1.0 with its look smoothing along 0.2 and its heading along 0
- **THEN** its look smoothing and its heading are along 1.0

### Requirement: The look snap
The look snap SHALL move the body's heading to its look direction when the yaw of its look smoothing lies strictly more than 30° (the single product of 30 and the degree) from the yaw of its heading, the gap wrapped into ±π in double precision; a NaN gap SHALL move nothing.

#### Scenario: Within and past the cone
- **WHEN** a heading along 0 is snapped with its look smoothing along 0.5, then along 0.6
- **THEN** it stays along 0, then takes the look direction

### Requirement: The body directions in the request
After the heading block the body's look direction, look smoothing and heading SHALL be written into its locomotion request as the cursor (and the wanted direction), the look and the heading, each the engine's two-argument arctangent of the vector. A request yaw that the body or the route walk wrote since SHALL be taken back as the direction of that yaw at the start of the next brains' tick.

#### Scenario: A turn toward a fall
- **WHEN** after a write the body's turn toward a fall writes the request's heading
- **THEN** the next brains' tick takes that heading back as the body's heading and keeps its look and cursor

### Requirement: A standing body faces its model until its AI writes
A standing character on its locomotion whose route is not driven SHALL face as its model faces only until its AI has written its body directions; after that the standing facing step SHALL leave its request alone.

#### Scenario: A fighter that entered combat
- **WHEN** a duellist on its locomotion has written its directions and stands
- **THEN** the standing facing step leaves its heading where the look snap put it

### Requirement: A fighter turns by its body
A duellist on its locomotion SHALL NOT turn its model to the AI's heading: its combat tick SHALL read the body's heading and look smoothing, and the movement decision SHALL turn the body toward the heading at its rates, so the body's yaw trails the heading, which trails the look by up to 30°. A body without muscles SHALL still be turned to the AI's heading.

#### Scenario: A foe a quarter turn away
- **WHEN** a duellist on its locomotion faces +Z with its foe 3 m along +X and ticks 80 times through the brains' tick and the movement decision
- **THEN** the request's look follows the AI's heading, its heading moves only in snaps over 30°, the body's yaw never passes the heading and the model takes the body's yaw

## MODIFIED Requirements

### Requirement: A standing body keeps its facing
The stand-in that turns the model of an NPC without muscles to the AI's eased yaw SHALL remember that yaw, and the facing the AI reads SHALL be that yaw while the model keeps the rotation the stand-in wrote, and the engine's heading of the model's forward once anything else moved it.

#### Scenario: An idle brain stands for 200 ticks
- **WHEN** an idle brain placed facing yaw 1.0, with no sway, ticks 201 times
- **THEN** its model faces the engine's heading of the placed facing after the first tick and the same yaw, within 1e-6, after the last
