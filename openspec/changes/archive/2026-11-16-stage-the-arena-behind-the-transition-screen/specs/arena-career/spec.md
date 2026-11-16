## ADDED Requirements

### Requirement: The arena's next round runs behind the transition screen
The end screen's Continue into a next round or a tournament round SHALL raise the round counter, close the end menu and open the transition screen with the music fade off; the round SHALL be stood only at the screen's black, and the end menu SHALL stay closed until then. Without the flow the round SHALL be stood at once.

#### Scenario: A next round's Continue
- **WHEN** the Continue is pressed on the next-round screen with the flow running, on 16 ms frames
- **THEN** the screen opens, the round counter rises, the menu closes, and the fresh opponent is asked for about 38 frames later

### Requirement: The match start runs behind the transition screen
The hub's Begin and the rematch SHALL hold their launch and open the transition screen with the music fade on; the launch SHALL go out as the match start once, at the screen's black, and the screen SHALL stay black over the level change it asks for. Without the flow the launch SHALL go out at once.

#### Scenario: A Begin's launch
- **WHEN** a launch is written with the flow running, on 16 ms frames
- **THEN** no match start goes out during the fade, one goes out about 38 frames later, and the pause stays raised past the load wait

### Requirement: The arena staging holds the freeze under a screen
Standing a match's first round, its next round and a tournament's bracket round under a running transition screen SHALL hold the staging freeze until the screen closes; without a screen they SHALL set the counted freeze.

#### Scenario: The next round's freeze
- **WHEN** a next round is staged at the screen's black and the screen then holds and fades back in
- **THEN** the freeze is held through every frame of the hold and the fade, and is clear once the close's release runs
