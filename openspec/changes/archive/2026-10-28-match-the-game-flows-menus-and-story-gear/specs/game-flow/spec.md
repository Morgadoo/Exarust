## ADDED Requirements

### Requirement: Story start record
Before the level loads, a new story SHALL clear the player's role list and add roles 7, 1 and the background's own; the unknown background SHALL carry item 0x10c with the third word 0x0080_0020 and get the power word 0x510000 and value 4.0; the knight and proctor SHALL hold item 0x1f in the off hand and carry it with flags 0x100 and the default third word 0x0004_0028.

#### Scenario: The knight's gear
- **WHEN** a knight's story starts
- **THEN** its roles are 7, 1 and 0x1b, its off hand holds item 0x1f and its carried list gains item 0x1f with flags 0x100

### Requirement: Menu page rows
The title's CONTINUE SHALL be live only when the story's save manager has a save and the start page's only when the arena's has one; the in-game page's first row SHALL read NEW GAME in practice and CONTINUE otherwise; the arena pause page's last row SHALL read EXIT when the session word has its value-4 bit and FORFEIT otherwise.

#### Scenario: Only an arena save
- **WHEN** only the arena's manager has a save
- **THEN** the title's CONTINUE is greyed out and the start page's is live

### Requirement: Get-up writes
While a villager's get-up holds, the story hook SHALL write animator flags 0x600, blend 1 and move and turn 0.49; once 500 ms have passed it SHALL write flags 0x20, blend 0.4, turn 0.4 and the clip clock at the motion's value less 1490 ms. A revival SHALL set flag 0x40 and push the clip clock 2000 ms on.

#### Scenario: The get-up starts
- **WHEN** a get-up begun at 10 000 ms is ticked at 10 501 ms
- **THEN** flags 0x20, blend 0.4 and turn 0.4 are written and the get-up ends

### Requirement: Session bookkeeping
Leaving a session SHALL stop the hook and frame task, close the game view, the dialogue and the prelude, run a scripted mode's end hook, then close the save package and hide the end menu before the world goes; back to the title SHALL show the main menu's title logo and leave the GUI scale alone; a session's begin and continue SHALL clear the end menu's no-row-selected flag; a story continue SHALL clear only the first wake-up's animating flag.

#### Scenario: Back to the title
- **WHEN** a story returns to the title
- **THEN** the title logo shows and neither the help flag nor the GUI scale is written
