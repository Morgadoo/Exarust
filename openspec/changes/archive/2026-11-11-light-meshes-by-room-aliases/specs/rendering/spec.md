# Spec Delta

## ADDED Requirements

### Requirement: Lights per sector
In tile-grid levels the system SHALL light a mesh only with lights that have an alias in the sector the mesh is registered in, as the original lights a mesh through its sector's aliases. A light SHALL have an alias in the sector it stands in (the room of its cell, or the scene's own sector off the rooms) and in the room of every cell its flood records it in. The lists SHALL be rebuilt when the light floods again, and a mesh that moves to another sector SHALL be relinked. Terrain tiles SHALL skip the test. The same lists SHALL choose the light's shadow casters.

#### Scenario: Character in a doorway
- **WHEN** a character's root stands in one room and its arm reaches into the next room, whose lamp does not flood into the character's room
- **THEN** the lamp neither lights the character nor takes it as a caster

#### Scenario: Compare
- **WHEN** the game runs with `--no-room-lights`
- **THEN** every light links as before, by weight and wall window only
