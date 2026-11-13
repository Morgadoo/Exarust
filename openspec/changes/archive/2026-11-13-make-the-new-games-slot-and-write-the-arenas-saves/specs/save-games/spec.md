## ADDED Requirements

### Requirement: New game slot
The session begin of a new story SHALL create an empty slot in the story manager, and of a new career in the arena manager, before the mode's start; the slot SHALL be named after the created character and become the current slot. Practice SHALL create none.

#### Scenario: A new story
- **WHEN** a story begins over a folder holding slots 1 and 2
- **THEN** slot 3 is created at the front of the story list, named after the created character, and the story's first save writes it

### Requirement: Current manager saves
The flow's save steps SHALL write the current slot of the mode's manager: the story's in story mode, the arena's in arena mode. An arena save SHALL write the company's tables and the manager; the arena SHALL keep no checkpoint.

#### Scenario: A career's save
- **WHEN** a career's slot is current and a save step runs
- **THEN** the arena slot's save holds the company's money and day and no story slot is written

### Requirement: Arena Begin save
The board's Begin and the auto rematch SHALL ask for a save without the death rule before their launch, written before the launch's level change tears the hub down.

#### Scenario: The rematch
- **WHEN** the auto rematch's timer comes due with a board to replay
- **THEN** one save step and one launch are written

### Requirement: Quit autosave
When the application closes while a session plays, the session's quit SHALL run before the app exits, with the menu's page: the story SHALL save with the death rule, the arena SHALL save from the in-game page and forfeit from the arena's pause page; practice SHALL write nothing.

#### Scenario: Closing the app in the story
- **WHEN** the app is asked to close while a story with a current slot plays
- **THEN** the slot is saved in the same frame, and the next frame writes nothing more

### Requirement: Forfeit save
The quit's forfeit SHALL settle the match first and then write the arena slot's player data alone, with no world databases, level section or promotion pass. The pause page's own forfeit SHALL write no save.

#### Scenario: The quit's forfeit
- **WHEN** the app closes during a match from the arena's pause page
- **THEN** the match is settled once and the forfeit's player-data save is asked for

### Requirement: Golem alarm in the settings chunk
A story save's level section SHALL carry the golem alarm in its settings chunk: the armed flag as the gate, the armed point as the tag, the armed zone's pass and the alarm count. A level read from a save's own section SHALL restore it, armed when the point's height is under 1 000 000.

#### Scenario: An armed alarm
- **WHEN** a story save is written while an alarm is armed and that section is read back
- **THEN** the alarm has the same armed flag, point, pass and count

### Requirement: Live journal
Every save SHALL write the in-game journal's pages as its journal chunk, and a story load SHALL put the loaded pages into the in-game journal with the first page selected.

#### Scenario: The journal round trip
- **WHEN** a save is written with two journal pages and then loaded
- **THEN** the save's journal chunk holds both pages and the in-game journal shows them
