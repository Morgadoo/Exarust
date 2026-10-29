## ADDED Requirements

### Requirement: Placed objects through the stream
A level load SHALL build its placed objects through the content stream's level pass, one entry a pass in file order within the step's budget, beginning the level with the session's arrival word and previous level (the words for none when nothing set them). An entry whose record is not in the database or whose model is not found SHALL count as missing; the placed player's position SHALL be reported with the level.

#### Scenario: A level without session words
- **WHEN** a level is loaded with no arrival word set
- **THEN** no placed object is the player and the missing count is at least the objects whose model was not found

### Requirement: Level wait report and handoff
The level wait SHALL report a non-zero missing count as "N missing database objects." under "Content management" and SHALL report nothing for zero. It SHALL hand the finished objects to the scene's grid environment when the scene has one and its room-mode word has either bit of 0x3, and SHALL append them to the caller's list otherwise; every shipped scene's room-mode word SHALL be 0x73.

#### Scenario: Thirty-seven missing
- **WHEN** a level ends with 37 missing objects
- **THEN** the report reads "37 missing database objects."

#### Scenario: A grid scene
- **WHEN** a scene with a grid environment has the room-mode word 0x73
- **THEN** the objects go to the grid environment, and with the word 0x70 they are appended
