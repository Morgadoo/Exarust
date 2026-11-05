## ADDED Requirements

### Requirement: Frame writes as fixed-step commands
A frame system that changes simulation data SHALL be able to post the change, and every posted change SHALL be applied at the start of the next fixed step, in post order, on the world that runs the fixed step, alike with and without the bridge.

#### Scenario: A posted write
- **WHEN** the frame posts a write to an object's position between two steps
- **THEN** the next step's systems see the new position, and the logs of what changed match between one world and the bridge

#### Scenario: Going limp
- **WHEN** a body is asked to go limp in a frame
- **THEN** its muscles go at the start of the next fixed step, before that step's systems run, with and without the bridge
