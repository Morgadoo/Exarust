## ADDED Requirements

### Requirement: Main-menu back cue
A menu back that changes the page SHALL play MenuClick at pitch 1.0 before the page shows; a back that changes nothing SHALL play nothing.

#### Scenario: Back from the arena page
- **WHEN** Escape is pressed on the arena page, then again on the title
- **THEN** the first plays MenuClick at pitch 1.0 and shows the title, the second plays nothing

### Requirement: Main-menu drag selection
While a press is held, the pressed row SHALL be selected only while the pointer is on it; on another row or off the rows no row SHALL be selected, and the release there SHALL run nothing.

#### Scenario: Dragging off the pressed row
- **WHEN** a row is pressed and the pointer moves to another row, back, then off the rows and is released
- **THEN** no row, the pressed row, then no row is selected, and the release runs nothing

### Requirement: Version line over a show
The menu's hide SHALL clear the version-line flag, so the version line does not stay over a show such as the creator or the company dialog.

#### Scenario: Hiding the title's menu
- **WHEN** the menu hides from the title
- **THEN** the version-line flag is clear

### Requirement: Transition screen takes the pointer
While the transition screen is up, the main menu and the hub forms SHALL take no pointer press, release or hover.

#### Scenario: A press during the company dialog's fade
- **WHEN** BACK starts the transition and NEXT is pressed before black
- **THEN** nothing more is asked

### Requirement: Divination reading stream and close
The divination readings SHALL deal from one running stream, so a reading goes on from the last one's draws. A right press SHALL close an open reading; a left press SHALL not. The reading SHALL paint at the window's GUI unit.

#### Scenario: Four readings
- **WHEN** four readings are opened and closed in turn
- **THEN** they deal the stream's successive cards and not the same three each time

### Requirement: Loadout encumbrance dots
The loadout form's encumbrance record SHALL show ten times the square of the shown kit's encumbrance, drawn as rating dots.

#### Scenario: A worn piece and a shield
- **WHEN** the roster page shows a kit of encumbrance 0.525
- **THEN** the record's value is 2.75625

### Requirement: Tip banner box
The tip banner's box SHALL be the rounded program's black fill, corner 4 units, alpha 0xa0.

#### Scenario: The banner's box
- **WHEN** the banner is drawn 100 pixels of text wide at unit 4 in a 640-pixel window
- **THEN** its box is the rounded fill of (266, 32, 108, 16) at corner 4 and alpha 0xa0

### Requirement: Destination rows' words
Each destination row SHALL carry its entry's destination word, which the journey takes.

#### Scenario: A journey from the first row
- **WHEN** a row whose destination word is 7 is clicked
- **THEN** the journey's level and marker are 7
