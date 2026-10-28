## ADDED Requirements

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
