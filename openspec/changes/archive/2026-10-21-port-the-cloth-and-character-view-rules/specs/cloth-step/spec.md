## ADDED Requirements

### Requirement: Cloth owns its mesh
The system SHALL give each cloth node a private copy of its mesh asset before the cloth moves its vertices, and SHALL skip for good a cloth node whose mesh has no positions or no indices.

#### Scenario: Two copies of one model
- **WHEN** two entities share one mesh asset and both carry a cloth marker
- **THEN** each holds its own mesh asset and the cloth of one does not move the vertices of the other

### Requirement: Cloth importer gate
The system SHALL give a marked node a cloth controller only when the scene the node is loaded into has the cloth scene flag (0x10000).

#### Scenario: Marker without the flag
- **WHEN** a node carries the cloth marker in a scene without the flag
- **THEN** it gets no controller
