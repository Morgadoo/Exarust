## ADDED Requirements

### Requirement: A hovering body looks ahead by the straight walk
The exploration look-ahead SHALL ask the navigation grid's straight walk (static floors, five steps up, following the floor down) for a player body whose race hovers (the race record's hover flag, the wraith), and the line walk for any other body.

#### Scenario: A rise five steps up
- **WHEN** a hovering player and an ordinary one walk forward toward a floor five steps above theirs half a metre ahead
- **THEN** the ordinary player's look-ahead is blocked there and the hovering one's is not
