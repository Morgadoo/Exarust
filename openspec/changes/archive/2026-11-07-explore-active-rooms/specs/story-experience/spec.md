## MODIFIED Requirements

### Requirement: Live discovery pass
In the running story, each of the flow's once-a-second discovery passes SHALL first explore around the player in a grid level, the level's rooms giving each cell's room and a room counting as open once it has faded in (its active flag), in the level's map found by its locale name or added at the grid's size; it SHALL then convert the account pool, the player and each companion at the game clock, and SHALL write the player's remaining experience, skills and powers back to the player's record.

#### Scenario: One pass after the level start
- **WHEN** a player carrying 600 experience, training its first skill from 5000 points, starts at 0 and a pass runs at 1000
- **THEN** the record holds 580 experience and the first skill 5008 points

#### Scenario: Exploring an active room
- **WHEN** the player stands in a cell of a 64 × 6 grid's one open room (rows 1–4, columns 1–10) at column 5, row 2, while the room fades in and then once it has faded in
- **THEN** the first pass explores nothing and the second explores 20 cells; a third explores nothing, and another level's name explores the same 20 in its own map
