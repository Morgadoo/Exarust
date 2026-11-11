## ADDED Requirements

### Requirement: Drawn effects are not rolled back
The frame writes the blast, torrent, barrier, bubble, barricade and wisp effects only to draw them. When the simulation runs free, the bridge SHALL drop a frame copy of one of these effects that is based on an older step than the simulation's latest write of that effect. The frame copy SHALL NOT overwrite the simulation's newer state.

#### Scenario: An AI's Force Blast in free mode
- **WHEN** a duellist charges a Force Blast while the simulation runs free and the frame draws the blast every frame
- **THEN** the blast reaches its release and meets the level's meshes, as it does in lockstep
