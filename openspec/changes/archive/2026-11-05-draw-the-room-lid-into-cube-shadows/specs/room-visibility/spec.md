# Spec Delta

## ADDED Requirements

### Requirement: Room lid in cube shadows
Every cube face rendered for a point light SHALL also take the black lid (the merged ceiling strips) of the room at the light's position, after the casters, with front faces culled and the depth written at distance over the light's radius with no caster scale; a light outside every room, or in a room without a lid, SHALL take none, and a light whose lid changes SHALL render its faces again.

#### Scenario: A light under its room's lid
- **WHEN** a light stands below its room's lid
- **THEN** the lid's underside is in each of its faces and nothing above the lid is lit by it

#### Scenario: A light above the lid
- **WHEN** a light stands above a room's lid
- **THEN** the lid's upper side is culled and the room below is lit as without the lid

### Requirement: Shell query callers
The hand's reach test SHALL ask the shell query for occluders along the cursor ray up to the picked point, for every point but a door, and SHALL put the point out of reach when an active room's occluder stands there; the kinesis power's clear line from the head to its aim SHALL ask the occluders, then the lids, before the level.

#### Scenario: A point seen across a wall's hollow
- **WHEN** the cursor ray reaches a point past an active room's occluder
- **THEN** the point is out of reach

#### Scenario: A lid on the kinesis line
- **WHEN** an active room's lid stands between the caster's head and the aim
- **THEN** the aim is not taken and the last one stays
