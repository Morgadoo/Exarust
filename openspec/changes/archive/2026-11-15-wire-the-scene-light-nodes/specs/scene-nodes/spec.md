## MODIFIED Requirements

### Requirement: Light link order
The system SHALL order a node's bucket list heaviest first by taking each entry after the head off the chain and walking it back while the entry before is strictly lighter, an unordered compare stopping the walk.

#### Scenario: Weights with a NaN
- **WHEN** the buckets weigh NaN, 1 and 3 in that order
- **THEN** the list runs NaN, 3, 1

## ADDED Requirements

### Requirement: Bucket order after the link tests
In both node passes, a mesh that needs buckets and just moved SHALL have every alias's link test write its bucket's weight and faces and then its buckets ordered; another such mesh SHALL test the moved aliases and be ordered again only when a test lit it.

#### Scenario: A moved mesh in a room with two aliases
- **WHEN** a moved bucket mesh's link tests weigh its two aliases 1 and 3
- **THEN** its bucket list runs from the second alias to the first

### Requirement: Light alias list
A light SHALL keep the sectors of its aliases in the order they were made; a new alias SHALL be appended to it, and removing one SHALL move the last into its place, mark the alias's sector as having lost a link and clear the alias's owner.

#### Scenario: The first of three aliases removed
- **WHEN** a light aliased in sectors 1, 2 and 3 loses its alias in sector 1
- **THEN** its list reads 3, 2 and sector 1 is marked

### Requirement: Room cell hook
Each time the flood records a light in a cell of a room, the light's alias owned by that room's sector SHALL take the frame counter as its flood stamp; without one, a new alias SHALL be made there, stamped and moved, and the sector marked light-added.

#### Scenario: A flood through a door
- **WHEN** a light in room A floods cells of rooms A and B
- **THEN** it has stamped aliases in both sectors, the one in B new

### Requirement: Flood drops unstamped aliases
After the cell hooks of a flood, every alias of the light whose flood stamp is not the frame counter SHALL be removed, walking the list by index up to its starting length as the removals swap the last alias into the freed slot.

#### Scenario: Four aliases, one stamped
- **WHEN** a light aliased in sectors 1 to 4 floods only into sector 3
- **THEN** its list reads 3 and the other three sectors are marked

### Requirement: Light nodes of the live scene
In a tile-grid level every point light with an id SHALL be an omni light node registered in the sector of the room at its position, or the scene's own sector off the rooms; a moved light SHALL take its new frame, moving sector first when its room changed, and a light that goes SHALL be queued for removal.

#### Scenario: A light carried into another room
- **WHEN** a light moves from room A into room B
- **THEN** its node moves to room B's sector and its alias in room A goes after the next flood

### Requirement: Live light flood
The live scene's activation of a light SHALL take, as its flood, the rooms of the cells the light flood recorded it in that frame, in the order recorded; the link pass SHALL let a mesh take only the lights with an alias owned by its sector.

#### Scenario: A shipped dungeon
- **WHEN** exanima01, exanima02 or exanima06 is loaded and run for two frames
- **THEN** every light's aliases are in its room and the rooms its flood recorded, all stamped by its flood, and a light in no room has none in the scene's own sector
