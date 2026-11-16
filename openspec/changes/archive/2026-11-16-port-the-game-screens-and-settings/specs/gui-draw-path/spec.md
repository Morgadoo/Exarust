## ADDED Requirements

### Requirement: Text box line count
The text box's line count SHALL be one for a text under two bytes and SHALL add a line when the byte before the last is a carriage return.

#### Scenario: Empty text
- **WHEN** the text is empty
- **THEN** it counts one line

### Requirement: Forget dialog lines centred
The skill screen's forget dialog SHALL draw its title and question centred in its text width and clipped after the glyph that passes the width's end.

#### Scenario: A narrow title
- **WHEN** the title is narrower than the dialog's text width
- **THEN** it starts at the rounded centre offset

### Requirement: GUI reselect on mode switch
A game mode switch SHALL force the game GUI's reselect under the pointer.

#### Scenario: Start of play
- **WHEN** the Character mode starts
- **THEN** the GUI's active control becomes the reselect code and the pointer is read again
