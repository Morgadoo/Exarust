## ADDED Requirements

### Requirement: Forced room walk
While the view follows no player (the Spectate mode of an arena match in which no entrant is the player's), the active-room walk SHALL ignore the room anchor. It SHALL want only the room at the arena's spawn spot, with the walk's own list, hysteresis and fades, and SHALL want no door's room. When no room is there, or the session has no forced point, the walk SHALL leave the rooms unchanged. The forced point SHALL hold only while the match scene exists.

#### Scenario: Forced beside a door
- **WHEN** the walk is forced to a room whose door point is 1 m from where the anchor would be
- **THEN** only that room is wanted, it fades in over 1000 ms, and forcing another room makes the first leave over the fade

#### Scenario: A spectated match
- **WHEN** the manager is benched because no entrant is the player's
- **THEN** the walk is forced to the room at the arena's spawn spot until the match level is left
