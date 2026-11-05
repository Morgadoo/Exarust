## ADDED Requirements

### Requirement: Height Query mesh mode
The ground placement's Height Query SHALL take, besides the level, the body meshes listed in the nine sector cells around its point. It SHALL leave out a body out of the world and a living character's limb, and keep a dead character's limb. Each mesh SHALL be pre-tested by the segment against its box and the swept sphere against its bounding sphere, then swept down its tree's column. The highest surface met SHALL win.

#### Scenario: A crate on the floor
- **WHEN** a Height Query of radius 10 cm comes down over a 40 cm crate body standing on the floor, and another beside it
- **THEN** the first finds 40 cm and the second the floor
