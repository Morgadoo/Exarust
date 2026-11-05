## ADDED Requirements

### Requirement: Bodies as snapshots
Behind the bridge the frame SHALL read bodies and the mechanisms on them as snapshots of the simulation, and its writes to them SHALL reach the simulation as posted writes at the start of the next step, before the step runs, alike with and without the bridge.

#### Scenario: A scripted corpse drag
- **WHEN** the player drags a corpse with a scripted press and pointer move, with fixed frames and the load held, once in one world and once behind the bridge
- **THEN** the dragged grab point is the same every frame and the corpse's joint angles do not differ

#### Scenario: A frame write that is not posted
- **WHEN** a frame system changes a body behind the bridge without posting the change
- **THEN** the change does not reach the simulation, the next copy out replaces it, and the bridge warns once
