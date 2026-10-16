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
