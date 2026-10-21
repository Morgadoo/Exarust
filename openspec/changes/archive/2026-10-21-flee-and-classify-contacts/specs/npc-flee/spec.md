# Spec Delta

## ADDED Requirements

### Requirement: Starting a flight
A fresh flight SHALL halve the courage, add 2 to the stress and set the level to 4, and SHALL remember the room the threat is in; renewing it SHALL change neither. Cornered, with no way out, the character SHALL return to combat at level 3 with its courage doubled.

#### Scenario: Flight and corner
- **WHEN** a fighter of courage 0.6 and stress 1 starts a flight, renews it, and is then cornered
- **THEN** its courage is 0.3 and its stress 3 at level 4, unchanged on renewal, and its courage 0.6 at level 3 when cornered

### Requirement: The way out
From its room, the character SHALL count every exit to a room it has not fled through whose door is open, and SHALL take, of those not leading to the threat's room, the one whose direction is most opposite the threat's, its cosine under 0.2; the goal SHALL be that exit, reached within 150 cm in mode 30. A room with fewer than two counted exits, or one exit back toward home, SHALL be remembered; eight rooms at most SHALL be remembered, the oldest forgotten.

#### Scenario: Away from the threat
- **WHEN** a threat stands toward +z of a room with exits toward +z, −z and +x; then the −z exit is shut and the +z exit leads to the threat's room; then only the +z exit remains
- **THEN** the −z exit is taken, then the +x exit, and then nothing, the room remembered

### Requirement: The flee state
Once the threat is in another room than the character's and its home's, or with no foe and no focus, the flight SHALL end: level 0 and a calm of 10 s plus a draw of 10 s. Otherwise the character SHALL run and SHALL plan again with no route or within 150 cm of the route's goal; a plan ending in its room within 15 m of the threat SHALL remember the exit's room.

#### Scenario: Escaped, running, replanning
- **WHEN** at 1000 ms the threat is in another room with the generator at 0, then in the character's room with the goal at hand, then 200 cm away
- **THEN** the flight ends calm until 11 000 ms, then it runs and plans again, then it runs on
