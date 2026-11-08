## ADDED Requirements

### Requirement: Force dome teardown
A Barrier's or force Bubble's dome SHALL hold the creator's GI light and the cast's hum voice until it is destroyed; the destructor SHALL free the hum voice and take the light out of the scene, and the node SHALL leave the scene.

#### Scenario: A barrier that has shrunk away
- **WHEN** a Barrier's dome reaches state 4
- **THEN** its hum voice is freed, its GI light is no longer registered and its entity leaves the scene
