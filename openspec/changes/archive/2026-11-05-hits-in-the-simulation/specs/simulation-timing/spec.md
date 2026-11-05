## ADDED Requirements

### Requirement: Hit chain in the simulation
The hit chain and the systems that react to its results in the same frame SHALL run in the simulation's once-a-frame work, before the frame's ticks and before the bodies' frame start queues the blows they sent, on the first fixed pass of each frame, alike with and without the bridge.

#### Scenario: A scripted walk
- **WHEN** the arena runs a scripted walk with fixed frames, in one world and behind the bridge
- **THEN** every joint of every traced tick is the same in both runs

#### Scenario: A blow in a frame without a tick
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it is dealt by the next tick
