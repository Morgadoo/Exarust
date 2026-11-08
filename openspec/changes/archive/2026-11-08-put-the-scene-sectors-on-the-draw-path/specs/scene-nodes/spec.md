## ADDED Requirements

### Requirement: Live scene of a grid level
The system SHALL keep a scene for the loaded tile-grid level: the scene's own sector first, then one sector per room in room order, each with the door portals of its links facing the room behind and its linked sectors; every drawn mesh SHALL be a mesh node registered in its room's sector (a character's meshes in the room at the character's position, anything else in the scene's own sector), moved when its room changes, queued for removal when it goes, and its frame refreshed when it moves. Each frame the scene SHALL take the room states, with its own sector active, and run its frame with the main camera's view volume; a registered mesh the visible pass leaves without a draw bit SHALL leave the main camera's visible list, and shadow views SHALL keep it.

#### Scenario: Two rooms with a door
- **WHEN** a grid of two rooms joined by one door is loaded
- **THEN** the scene has three sectors, each room sector has one portal leading to the other room's sector, facing it, and lists it as linked

#### Scenario: A mesh outside the doorways
- **WHEN** a mesh of an adjacent room lies outside every frustum through its doorways to the active rooms
- **THEN** the main view does not draw it and its shadows are unchanged

### Requirement: Quiet passes
With quiet passes on, a sector whose last pass was the hidden pass and whose lists did not change since SHALL visit only the meshes refreshed since, with the same masks and cells as the full pass.

#### Scenario: A hidden room twice
- **WHEN** a hidden room's pass runs twice and one of its meshes then moves
- **THEN** the third pass reports that mesh moved and stores its cell, as without quiet passes
