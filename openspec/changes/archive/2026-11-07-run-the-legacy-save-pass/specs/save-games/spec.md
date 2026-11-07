## ADDED Requirements

### Requirement: Legacy pass at start-up
When the running game is given a save folder, making the slot managers SHALL run the legacy removal over the story manager's saves and then the arena manager's, before any slot is listed; a pass that fails SHALL be logged and SHALL not stop the start-up.

#### Scenario: An old story save in the folder
- **WHEN** the game starts with a save folder holding a current story save and one whose player-data signature is 0x6F5AEA13
- **THEN** the old save is deleted before the menu lists the slots and the current one stays
