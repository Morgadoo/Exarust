## MODIFIED Requirements

### Requirement: Searching the level's objects in the running game
The search's candidates in the running game SHALL be the placed objects the sector's lists hold. Each room cell SHALL list the objects of its body grid's parts, then those of its static grid's meshes, each object once a cell, with its first node's box and its tagged database id. An object with neither a body nor a static collider SHALL not be gathered. One straight under the level SHALL be searchable; placement bit 0x8000 SHALL mark it fixed.

#### Scenario: A chest and a loose blade
- **WHEN** a follower stands 1.5 m from the player, with a placed chest holding a better sword 3 m away and a loose blade of another class 2 m away
- **THEN** the follower takes both: the chest's section no longer holds the sword, the loose blade's object is gone, and the better weapon is in its hand, still listed and marked held

#### Scenario: Placed by the lists
- **WHEN** the lists hold an object in cells 10 and 11 and another in cell 10
- **THEN** cell 10 lists the first then the second, and cell 11 the first
