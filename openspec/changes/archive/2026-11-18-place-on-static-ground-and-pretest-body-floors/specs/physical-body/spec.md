## ADDED Requirements

### Requirement: Static ground placement for the exit nudge
When the story continue nudges the player out of a level exit, the nudged point SHALL be taken to the navigation grid's nearest walkable point, the character's root SHALL go there, and the body SHALL be placed on the ground by the leg snap whose Height Query meets the sectors' static meshes only, facing the placement's yaw; no living placement SHALL be asked for it.

#### Scenario: A continued player standing in an exit
- **WHEN** the continued player's saved place lies inside a level exit
- **THEN** its root stands at the walkable point past the exit and its body is placed over the static meshes only

### Requirement: Static ground placement for the summoned demon
When a summoned demon arrives, its body SHALL be placed on the ground by the leg snap over the static meshes only, its frame at the summon's place and turned by the summon's facing.

#### Scenario: A demon arriving on a crate
- **WHEN** a demon arrives at a place over which a loose crate lies
- **THEN** its legs are snapped to the level's static floor, not to the crate

### Requirement: Ground Query pre-test of a body node
The Ground Query SHALL meet a rigid body's mesh only when its downward ray, within the nearest hit found so far, crosses the body node's box in the node's own frame by the original's slab test: the approximate reciprocal of the direction, the entry clamped at 0 strictly before the exit and the entry strictly before the length, a NaN failing either compare.

#### Scenario: A flat body mesh
- **WHEN** the ray falls onto a body whose mesh is a flat plate (a box without height)
- **THEN** the Ground Query does not meet it, while a static plate in the same place is met
