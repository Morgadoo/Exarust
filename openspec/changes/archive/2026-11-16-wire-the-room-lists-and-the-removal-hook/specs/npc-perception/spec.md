## MODIFIED Requirements

### Requirement: The AI tick's room cell and alert
On a level with a grid, each AI tick SHALL first move the brain to the list of its level room, read at the floored position over 600 cm on each ground axis offset by half the grid, before its condition gate. Off the grid nothing SHALL change. A cell in no room SHALL unlink the brain and empty its nearby list. The group alert SHALL decay one step every 10 s, down to 1.

#### Scenario: Off the grid
- **WHEN** a brain linked in room 1 of a 6 × 3 grid moves to row 4
- **THEN** it stays linked in room 1 with its previous room unchanged

## ADDED Requirements

### Requirement: Room lists keep the newest first
A character moving into a room SHALL be put at the head of that room's list and leave its old room's list; its previous room SHALL become the old room only when the old one was a room. A dead brain SHALL stay linked.

#### Scenario: Two brains and a dead one
- **WHEN** two characters enter room 0 in turn and a dead brain stands in room 1
- **THEN** room 0 lists the second first and room 1 lists the dead brain
