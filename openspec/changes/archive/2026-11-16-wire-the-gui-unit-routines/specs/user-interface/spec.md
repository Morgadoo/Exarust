## ADDED Requirements

### Requirement: Message box key slot
A message box's key slot SHALL hear the frame's keys with their words: a key-down or its repeat on Enter or Space SHALL close it, on Escape with the result -1, a typed character SHALL be passed over, and no key SHALL climb past it, so no other key closes a window under it.

#### Scenario: A typed character, then Enter
- **WHEN** a box is up and A, then Enter goes down
- **THEN** the box stays after A and closes on Enter, and the loadout under it stays open

### Requirement: Loadout closes on the menu toggle
In the hub, a key-down no record of the open hub window takes SHALL climb to the root task; the menu toggle SHALL then close the loadout form when one is open and nothing else.

#### Scenario: Escape on the loadout
- **WHEN** Manage Company and the loadout form are open and Escape goes down
- **THEN** the loadout form closes and Manage Company stays open

#### Scenario: Escape on Manage Company
- **WHEN** only Manage Company is open and Escape goes down
- **THEN** nothing closes

### Requirement: Character screen leaves on the menu toggle
The saves' character screen SHALL keep the base form's key slot: each key-down SHALL climb to the root task, whose menu toggle SHALL leave the screen to the menu at once, an open box or list included.

#### Scenario: Escape with the delete box up
- **WHEN** the delete box is up on the character screen and Escape goes down
- **THEN** the screen closes and the menu's mode returns

### Requirement: End menu key slot
The end menu SHALL share the main menu's key slot: a typed character SHALL shift into its ten-byte name buffer, toggling the menu music on `barechoons`, and each key-down without a low word bit SHALL climb to the root task.

#### Scenario: The music code on the end menu
- **WHEN** barechoons is typed on the shown end menu
- **THEN** the alternative-music flag sets and every key-down climbed once

### Requirement: Large icons from the common glyph sheet
After a level switch empties the large icon atlas, the atlas SHALL take the common glyph sheet's 19 icons, each copied out of the sheet at its stored corner, resized to the level's cell and sharpened, as the handles 1 to 19 in the sheet's order.

#### Scenario: A synthetic sheet
- **WHEN** a sheet whose 64-pixel squares hold their index is loaded at level 2
- **THEN** the eighth handle's cell holds the eighth corner's value and a level switch asks for the sheet again

### Requirement: Drag distance from the window height
Whenever the game screens', the hub's or the main menu's window manager takes a new window size, its pointer's drag distance SHALL be the height times one five-hundred-and-fortieth, rounded half to even, at least 2.

#### Scenario: A 2160-pixel window
- **WHEN** the window is 3840 by 2160, then 1280 by 720
- **THEN** the drag distance is 4, then 2

### Requirement: New windows kept on the screen
A new inventory or journal window SHALL be clamped to the screen: a negative x SHALL become 0, an x whose right edge passes the root's width SHALL move it flush with it, and its y SHALL be at least a unit.

#### Scenario: A journal remembered off the screen
- **WHEN** the journal's remembered place is 1200, -30 on a 1280-wide screen at unit 4
- **THEN** the journal opens at 1036, 4

### Requirement: Pad focus checks on base forms
With the GUI's pad flag on, a base form left by the focus SHALL drop the focus when its selected record's state has 0x400, and a base form whose newly selected record's style has 0x800000 SHALL take the focus.

#### Scenario: A record that drops the focus
- **WHEN** the selected record of a base form has the state bit 0x400
- **THEN** the pad's leave check answers yes for it and no for a record without it

### Requirement: Record text through the text setter
Setting a record form's text SHALL go through the GUI's text setter: the text, then the record's procedure with the message 0x400.

#### Scenario: The loadout's points line
- **WHEN** the rank slider is dragged on the loadout form
- **THEN** the points line shows the new rank's points

## MODIFIED Requirements

### Requirement: Menu toggle on the open menu
The root task's menu toggle SHALL branch on the session phase: in the menu's phase it SHALL go back on the open menu's start and arena pages and leave the menu up elsewhere; in a session's phases (3, 4, 5, 8) it SHALL close the open menu and the manual with it; with the menu closed it SHALL leave the opening to F1 and the screens' own Escape.

#### Scenario: Escape on the start page
- **WHEN** the menu shows the start page and Escape goes down
- **THEN** the title page shows, and a second Escape leaves the title page up

### Requirement: Keys that climb to the task
A key that a game screen's, the main or end menu's, the settings form's, the manual's, the save screen's or an open hub window's key slot passes up SHALL reach the root task's key slot after every form's handler of the frame; with no screen or form holding the keys, the game view SHALL pass each key-down on after the player's hotkeys.

#### Scenario: A binding over the view
- **WHEN** the pointer is over the game view and Ctrl+S goes down
- **THEN** the hotkeys hear S and the Ctrl+S binding runs

### Requirement: Message box shown
A message box SHALL be shown on top of the hub's form stack with its title as the caption, its text in a label and its buttons, centred across the screen and placed down at its design-pixel top rescaled to the GUI unit, and placed again after a resize. Ok SHALL close it with 1 and Cancel with 0.

#### Scenario: A hire without the money
- **WHEN** Hire is pressed on the hire page for a candidate the company cannot afford
- **THEN** a "Hire" box reading "Insufficient funds" with one Ok button shows over the loadout form, and Escape closes only the box

#### Scenario: A company without a name
- **WHEN** NEXT is pressed on the company dialog with the name field empty
- **THEN** a "Company Name" box reading "Please name your company" shows and the dialog stays open

