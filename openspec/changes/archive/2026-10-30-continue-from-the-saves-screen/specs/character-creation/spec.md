## ADDED Requirements

### Requirement: Saves' character screen
The menu's CONTINUE rows SHALL open the creation form in its continue mode with the original's eleven-record continue table, showing the first listed slot's character; Previous, Next and the slot combo box SHALL select other slots, Delete SHALL ask in the `Delete Character` box before deleting the slot, and the story's toggle SHALL switch between the save and the checkpoint when both exist.

#### Scenario: Two slots
- **WHEN** the screen opens over two story slots
- **THEN** the newest slot's character shows, Previous is disabled and Next is enabled

#### Scenario: Deleting the last slot
- **WHEN** the only slot is deleted through the box
- **THEN** both of its files are gone and the screen returns to the menu

### Requirement: Story save load
A story save SHALL hold the player's record and placement, and the screen's CONTINUE SHALL load the slot into the running game: the player spawns from the saved record, dressed from its items, at the saved placement on the save's level, with the saved world clock.

#### Scenario: Save and continue
- **WHEN** a story session is saved to a new slot and that slot is continued
- **THEN** the restored record has the saved name, worn and carried items, and the placement the saved position and heading
