## ADDED Requirements

### Requirement: Hierarchy copied for the simulation's entities only
Behind the bridge the frame SHALL send the hierarchy and transforms only of the entities the simulation holds and their ancestors, and the simulation SHALL step alike with and without the bridge.

#### Scenario: A walk in a story level
- **WHEN** the player walks in exanima01 or exanima06 with fixed frames, once in one world and once behind the bridge
- **THEN** the player's joint angles do not differ, and the frame's meshes and bones are not copied to the simulation
