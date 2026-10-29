## ADDED Requirements

### Requirement: The combat state asks the navigation grid
In a level with a navigation grid, a fighter's footwork SHALL cost each place the grid's cost sample under it as `penalty·40 + ((clearance² + clearance)·120 + 1)`, SHALL pull a place, and a long step's continuation, back to where the grid's line walk from its base stops (keeping the asked height), and its chase check SHALL count a target behind an obstacle as out of sight. The snap SHALL leave a point where it was.

#### Scenario: A wall ahead
- **WHEN** a fighter on a flat grid weighs a place 8 cells ahead with a wall 4 cells ahead
- **THEN** the place is pulled back to the cell before the wall, and a foe beyond the wall is out of sight

#### Scenario: A footprint and an open floor
- **WHEN** a place lies on a cell of full penalty, and another on open ground
- **THEN** the first costs 41 and the second 1

### Requirement: The fighter's own footprint and a level without a grid
The fighter's own footprint SHALL be off the grid while its combat state runs and back where it was afterwards. Without a grid every place SHALL cost 1, no line SHALL be pulled back and nothing SHALL be out of sight.

#### Scenario: Its own footprint
- **WHEN** a fighter whose footprint is stamped where it stands runs its combat state
- **THEN** where it stands costs only the other characters' footprints, and after the state its footprint is stamped there again

#### Scenario: No grid
- **WHEN** the level has no navigation grid
- **THEN** every place costs 1, no place is pulled back and the foe is in sight
