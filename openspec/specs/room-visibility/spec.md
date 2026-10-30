# room-visibility Specification

## Purpose
Shows what the original's room manager lets the player see in tile-grid
levels: the player's rooms in the open, the rooms behind their doors under
black lids, the other rooms not at all, and every wall hollow black.

## Requirements

### Requirement: Rooms
The system SHALL flood-fill a tile grid's open cells into rooms as the original does: the seed is the first open cell of the last row with one, door seeds wait last in first out, a room spreads over every edge but walls (7) and doors (B), a closed cell joins without spreading, and a door links two rooms when the cell behind it is open and does not face it with a wall.

#### Scenario: Arena hub
- **WHEN** the installed `arenahub` grid is flood-filled
- **THEN** it has 10 rooms and 18 door links, cells (15,17), (14,18), (16,19), (17,15) and (12,20) are in rooms 2, 1, 7, 6 and 0, as in the running original

#### Scenario: First story level
- **WHEN** the installed `exanima01` grid is flood-filled
- **THEN** it has 153 rooms and 356 door links and every open cell is in a room

#### Scenario: Island
- **WHEN** an open cell is walled off from the seed's room with no door path
- **THEN** it is in no room and is always drawn

### Requirement: Active rooms
The system SHALL keep the original's active rooms: the room of the anchor's cell (cells round down) and each room behind a door of that cell whose point 1.5 m inside the cell is within 2 m of the anchor, kept in order, fading in and out over 1 s with the progress carried over when a fade turns around; door neighbours of active rooms are adjacent, the rest hidden, and states lower only when a room leaves.

#### Scenario: Live views
- **WHEN** the anchor stands at (−3.548, 9.482) m and then at (−4.991, 8.784) m in the arena hub for 1.5 s
- **THEN** the active rooms are {2, 1, 7} and {2, 1}, as the running original shows

#### Scenario: Door reach
- **WHEN** the anchor is 1.99 m from a door point of its cell, then 2.01 m
- **THEN** the room behind the door is active, then not

#### Scenario: No room
- **WHEN** the anchor stands in a cell without a room
- **THEN** no room changes state

### Requirement: Room drawing
The system SHALL draw nothing of a hidden room, draw an adjacent room with its occluder and its lid in opaque unlit black, and draw an active room with its occluder only; a room fading in or out SHALL have its lid drawn last in black at the fade's alpha, blended without depth test. Content outside every room SHALL always be drawn, and lights, shadows, GI and collision SHALL not depend on room states.

#### Scenario: Arena hub bar
- **WHEN** `exarust play arenahub` puts the player in the bar 2.6 m from its south door's point
- **THEN** the bar and the hall are lit, the south room is a black lid, and the storage room and kitchen are black

#### Scenario: Walking through a door
- **WHEN** the player comes within 2 m of a door's point
- **THEN** the room behind it appears from under a black overlay that clears over 1 s

### Requirement: Room anchor
The system SHALL anchor the active rooms on the player in play mode and on the point 20 m ahead of the camera in the level view placed with `--camera`; the level overview and `--all-rooms` SHALL show every room as active, without lids or fades.

#### Scenario: Retail pose in the level view
- **WHEN** `exarust level arenahub` runs at a pose of the original's camera
- **THEN** the same rooms show as in play mode with the player at that camera's target

#### Scenario: Every room
- **WHEN** `exarust --all-rooms play exanima01` runs
- **THEN** every room is drawn with its occluders and no lid

### Requirement: Shut door
A shut door SHALL NOT make the room behind it active. Every door neighbour of an active room SHALL be adjacent whether its door is shut or open, so the room behind a shut door is drawn under its lid; opening the door SHALL let that room turn active when the anchor is within 2 m of its door point.

#### Scenario: Closed then open
- **WHEN** the anchor stands 1 m from the door point of a shut door, then the door is opened
- **THEN** the room behind the door is adjacent, then active

#### Scenario: Golem gate
- **WHEN** the player stands at the shut double door between exanima06's two golem statues
- **THEN** the room beyond it is black

### Requirement: Portal clip
A point SHALL be seen through a doorway only when the segment from the eye crosses the opening and the door is open. A side nibble of 0 SHALL be 82 × 245 cm, any other nibble 115 × 290 cm, and a special tile 200 × 340 cm.

#### Scenario: Through and beside
- **WHEN** the eye looks through the middle of an open doorway, then beside it, then through it while the door is shut
- **THEN** only the first view is seen

### Requirement: Level passage
A passage SHALL load the level its destination id names. An id the list does not name SHALL keep the current level.

#### Scenario: Named destination
- **WHEN** a passage's destination is the id of exanima02
- **THEN** the loaded level is exanima02

#### Scenario: Unknown id
- **WHEN** a passage's destination is not in the level list
- **THEN** the loaded level stays exanima01

### Requirement: Room cell conversion
The system SHALL convert world points to cells by multiplying by the original's f32 1/600 and flooring, and SHALL convert a room's cell rectangle to centimetre bounds with the 0–280 cm ceiling band.

#### Scenario: A room rectangle
- **WHEN** a 32×32 grid's room spans cells (2, 3) to (5, 4)
- **THEN** its world bounds are x −8400..−6000 cm and z −7800..−6600 cm
- **WHEN** a point is at 5.99 m on X in a 6-wide grid
- **THEN** its cell is (3, 1)

### Requirement: Edge rotation
The system SHALL rotate a packed edge word by quarter turns: the low four nibbles cyclically and the high four nibbles' low three bits, with bit 3 of each high nibble fixed.

#### Scenario: One quarter turn
- **WHEN** the word 0x12345678 rotates once
- **THEN** it becomes 0x23416785
- **WHEN** 0x00000B07 rotates once
- **THEN** it becomes 0x0000B070

### Requirement: Direction step
The system SHALL step one cell opposite to a direction vector using the engine's quick yaw of `(x, z)` widened to double: `|a|` below the double π/4 steps −Z; strictly between the f32 π/2 widened less and plus the double π/4 steps −X; `|a|` above the double 3π/4 steps +Z; strictly between −π/2 plus and less the double π/4 steps +X; anything else (a NaN) does not step.

#### Scenario: Four directions
- **WHEN** the vector is (0, 1)
- **THEN** the step is (0, −1)
- **WHEN** the vector is (1, 0)
- **THEN** the step is (−1, 0)

#### Scenario: Diagonals
- **WHEN** the vector is (1, 1)
- **THEN** the quick yaw is 0.78517902 and the step is (0, −1)
- **WHEN** the vector is (1, −1)
- **THEN** the step is (0, 1)

### Requirement: Door portals
The system SHALL give each door link a portal on the cells' shared edge, spanning the edge's middle ± a half width from −50 cm up to a top: ±82 cm to 245 cm when plane B's side nibble is 0, ±115 cm to 290 cm otherwise, and ±200 cm to 340 cm on a special tile.

#### Scenario: Wide door
- **WHEN** a door link's cell has a non-zero side nibble on that side
- **THEN** its portal is 230 cm wide and runs from 50 cm below the floor to 290 cm above it

### Requirement: Light reach and spill
The system SHALL answer whether light passes between two points as the original does: both in room cells; the same room always; otherwise an open door link of one point's room into the other point's cell whose point ¾ of the way into that cell lies within 2 m of the other point. A light's spill SHALL be the rooms behind its own cell's open door links when it stands within 2 m of that cell's centre. Cells SHALL be indexed as row × width + column, checked only against the cell count.

#### Scenario: Spill from the cell centre
- **WHEN** a light stands 1.7 m from a door's ¾ point but 2.8 m from its cell's centre
- **THEN** it spills into no other room

### Requirement: Room link order
The system SHALL remove a room's links back into itself by moving its last link into the gap, and SHALL group every room's links by target room only after all rooms exist, swapping the first link at least two places on that leads into the same room as link i into place i + 1.

#### Scenario: Three closets
- **WHEN** a corridor's three doors lead into three closets that the flood reaches after the corridor
- **THEN** the corridor's links keep their flood order

### Requirement: Control-side door query
The system SHALL answer the control-side door query from both points' cells without room checks: the same room, or no room for both, reaches; otherwise the first link of the first point's room into the second point's cell decides by its door's test.

#### Scenario: Two doors into one cell
- **WHEN** a room has two links into one cell and only the second door passes
- **THEN** the query answers no

#### Scenario: Open ground
- **WHEN** both points stand in grid cells that belong to no room
- **THEN** the query answers yes

### Requirement: Active-room flag
The system SHALL report a room as active once it has faded in, and while it is wanted again in the middle of a fade-out, and not while it fades in or out.

#### Scenario: Turn-around
- **WHEN** a room fading out is wanted again
- **THEN** it is not active until its fade back in ends

### Requirement: Room light lists
The system SHALL give each room the lights of its own cells and then, visiting rooms in order, copy the whole list of every room that owns at least one light into each room its links lead into, skipping lights already listed.

#### Scenario: A room with its own light
- **WHEN** rooms A, B and C stand in a row with lights 7 in A, 8 in B and 9 in C
- **THEN** A lists 7 and 8, B lists 8, 7 and 9, and C lists 9, 8 and 7

### Requirement: Shell ray query
The system SHALL answer the room manager's shell query by walking the active rooms in order, answering no at the first active room without the asked shell, and testing each shell with a one-sided ray test that accepts a front-facing triangle (determinant at least the single precision epsilon) at a distance strictly between 0 and the limit.

#### Scenario: From behind
- **WHEN** a segment crosses an active room's occluder from its back
- **THEN** the query answers no

### Requirement: Quick yaw
The engine's yaw of `(x, z)` SHALL be 0 when `z² + x²` is below the f32 1e-8; otherwise the cosine `z / √(z² + x²)` SHALL give π at or below −1, 0 above 1 and the engine's quick arccosine between (a NaN through the arccosine), negated when `x` is below zero.

#### Scenario: Axes and NaN
- **WHEN** the direction is (−1, 0), (0, −1) or (NaN, 1)
- **THEN** the yaw is −π/2, π or NaN

### Requirement: World-point room query
The room at a world point SHALL be the room of the cell index `row × width + column`, with the cell from the floored 1/600 conversion plus half the grid, accepted when it is at least 0 and below the cell count, so a column off the grid's side reads a cell of the row before or after.

#### Scenario: Column off the side
- **WHEN** a 4 × 4 open grid is asked for the point in column 4 of row 1
- **THEN** it answers the room of cell (0, 2)

### Requirement: Cell active query
A grid cell SHALL be active when it belongs to a room whose active-room flag is set; a cell in no room is not active.

#### Scenario: Entering
- **WHEN** the viewer has just entered a room
- **THEN** its cells are not active until its fade-in ends, and the cells of the room behind its door are not active

### Requirement: Door body binding
A placed door with a body SHALL bind to the link from its cell to the cell one step opposite its third matrix row's `(x, z)` in its cell's room, and the body SHALL be readable from that link and its reverse; with no such link nothing is bound.

#### Scenario: Both facings
- **WHEN** a door in a door cell faces away from the room behind it
- **THEN** it is bound to the link and both sides carry its body
- **WHEN** the same door faces the other way
- **THEN** no link is found and nothing changes

### Requirement: Door link state
A door link SHALL count as open, for the active rooms, the reach query and the spill, while it has no door body or its body's pair flag is clear. The pair flag SHALL be set while each leaf of the door is unlocked and shut or less than 0.4 rad from shut (a sliding leaf: at 0.8 of its travel to shut or more), so a locked door counts as open.

#### Scenario: Swing
- **WHEN** an unlocked hinged door is shut, 0.39 rad open, then 0.41 rad open
- **THEN** its link counts shut, shut, then open

#### Scenario: Locked
- **WHEN** a locked door is shut
- **THEN** its link counts as open

### Requirement: Placed door binding
Each placed object of the door kind SHALL be bound at load to its door link with the body of its object node's first child, and each bound link SHALL take its body's state from both of its rooms every frame; a leaf without a body SHALL leave its link open.

#### Scenario: First story level
- **WHEN** the installed exanima01 is loaded
- **THEN** 96 of its 140 door objects bind to a link, each on both sides
