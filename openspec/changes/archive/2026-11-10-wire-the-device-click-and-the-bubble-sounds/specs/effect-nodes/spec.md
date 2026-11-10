## ADDED Requirements

### Requirement: Force dome teardown
A Barrier's or force Bubble's dome SHALL hold the creator's GI light and the cast's hum voice until it is destroyed; the destructor SHALL free the hum voice and take the light out of the scene, and the node SHALL leave the scene.

#### Scenario: A barrier that has shrunk away
- **WHEN** a Barrier's dome reaches state 4
- **THEN** its hum voice is freed, its GI light is no longer registered and its entity leaves the scene

### Requirement: Teleport bubble sounds
A teleport bubble SHALL take a voice of its own for `teleportf01` at its place when it is made (volume 0.3, pitch 1); a plain bubble that has grown SHALL play `teleportc01` at its place (volume 0.3, pitch 1, flags 0); while it shrinks its voice's volume SHALL be `(value + 1)·0.5`; and its voice SHALL be freed when it leaves.

#### Scenario: A plain journey's bubble
- **WHEN** a bubble without the hold grows for 1800 ms, rests 500 ms and shrinks
- **THEN** `teleportf01` starts with it, `teleportc01` plays once as it starts to rest, the voice is at 0.25 half-way through the shrink and is freed with the bubble

### Requirement: Teleport bubble light extent
The bubble's GI light SHALL take its radius and its box from the bubble node's stored 120 cm; the charge's swell SHALL scale the light's colour and pulse, compared and computed in double precision, and SHALL NOT grow the light.

#### Scenario: A charged bubble
- **WHEN** a holding bubble's charge is 0.9
- **THEN** its light's radius is 120 cm and its colour twice the narrowed swell `1 − (0.9 − 0.8)·5`
