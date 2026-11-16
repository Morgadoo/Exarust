## ADDED Requirements

### Requirement: Game mode word
The scene SHALL hold a game mode word. The start of play SHALL enter the Character mode and a Spectate start the Spectate mode; each switch SHALL first answer the leave routine of the mode the word holds when it is a mode's id. The player gone in Character mode SHALL leave the scene in no mode (word 0).

#### Scenario: Switching modes
- **WHEN** play starts, spectating starts, play starts again and the player goes
- **THEN** the word is 3, 4, 3 and 0, leaving no mode, the end of play, the end of spectating and the end of play

### Requirement: Camera record per game mode
Each mode's start SHALL give the chase camera its record: the Character record (flags 0x200300, its first distance 800 over a 600 stop on a Steam Deck) or the Spectate record (flags 0x220b01 with 0x400000), both view 250, size 6000, rate 0.01, distances 1300 in 800 to 2000 and heights 0 to 200. The record's first distance SHALL restart the zoom, and the wheel SHALL step between the held record's distances.

#### Scenario: Spectating starts
- **WHEN** the player arrives, the wheel zooms out to 2000 and spectating starts
- **THEN** the camera holds flags 0x200300, then 0x620b01 with the zoom at 1300

### Requirement: A new camera's first record
A camera built from the Character record SHALL take the first switch into Character as its construction and stay where it is; a switch to no mode SHALL leave the camera its record.

#### Scenario: The level's camera
- **WHEN** a new camera sees the start of play, then the end of play
- **THEN** it keeps its zoom and yaw and holds flags 0x200300 throughout

### Requirement: Free camera pan by record
A camera that follows nothing SHALL pan with the movement keys and the pad's move stick only under a record with flag 0x800 (the Spectate one), flattening the pan under 0x20000.

#### Scenario: The pad's stick
- **WHEN** a free camera with the Spectate record and one with the Character record tick with the stick held forward
- **THEN** the first one's point moves forward and the second one's stays

### Requirement: Readers of the mode word
The room walk SHALL be forced while the scene is in Spectate, and the controls SHALL not take a second end of play when the player is taken away after the switch to Spectate.

#### Scenario: Spectating
- **WHEN** spectating starts and the player is then taken away
- **THEN** the room walk is forced and the controls stay on

### Requirement: Window point projection
A world point SHALL reach the window as the game view's projection routine puts it: in front of the eye, half the size plus the projection scale times the size times a half times the coordinate over the depth; at or behind the eye, or with a NaN depth, the window's edge its side points to, 0 only for a coordinate below 0.

#### Scenario: Behind the eye
- **WHEN** a point behind the eye lies to the left and below
- **THEN** it reaches the window's left edge and its bottom
