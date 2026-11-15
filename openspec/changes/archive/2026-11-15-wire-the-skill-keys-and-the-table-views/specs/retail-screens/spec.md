## ADDED Requirements

### Requirement: Skill screen keys
While the skill ring is up, any key SHALL close it through the open routine's toggle, except during the reveal's first 100 ms and while it hides; a page or the compact tree SHALL not close on a key.

#### Scenario: The opening key and a later key
- **WHEN** the screen opened at clock 0 and K is pressed at 50 ms, then Escape at 150 ms
- **THEN** the screen stays up after K and closes after Escape, with the player's controls back on

### Requirement: Skill screen toggle
A request to open the skill screen while it is up SHALL close it.

#### Scenario: Request while open
- **WHEN** a skills request arrives with the ring up
- **THEN** the screen closes and the player's controls are on

### Requirement: Table selection band
A table view SHALL draw its selected row's band under the cells, the client's width and the row less two pixels, one row pitch per row down; with the pointer on that row the band SHALL be filled by a quarter-strength gradient first.

#### Scenario: Hovered selection
- **WHEN** row 2 of a unit-5 table 300 pixels wide is selected and the pointer is on it
- **THEN** the band spans 0, 60, 300 by 28 and is filled

### Requirement: Table drag label
A dragged row's label SHALL sit at the pointer less the grab, its first cell's text width less one wide and the row less two pixels tall.

#### Scenario: Label at the pointer
- **WHEN** row 1 is dragged with grab 10, 15 and the pointer is at 200, 100
- **THEN** the label's box starts at 190, 85
