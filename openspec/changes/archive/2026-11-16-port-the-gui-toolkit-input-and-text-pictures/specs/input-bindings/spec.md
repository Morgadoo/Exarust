## ADDED Requirements

### Requirement: GUI button words
The base-form window managers SHALL receive the right button's press and release as the word 2 and the middle button's as 4, the middle only while the player's controls are off, and every press SHALL carry Ctrl 0x40, Shift 0x20 and Alt 0x80.

#### Scenario: A Ctrl press on a title
- **WHEN** the left button is pressed on a window's title with Ctrl held and the pointer moves
- **THEN** the window does not move

### Requirement: GUI focus loss
When the window loses the system's focus the GUI SHALL release its held buttons, drop the capture and park the focus on the root, remembering the focused form; when the focus returns the remembered form SHALL be focused again.

#### Scenario: Alt-tab during a press
- **WHEN** the focus is lost while a press is held on a form
- **THEN** the form hears the release and the focus is on the root until the focus returns

### Requirement: Double-click time
The GUI's double click SHALL use the system default of 500 ms, the same time as the world's double click.

#### Scenario: Two presses 450 ms apart
- **WHEN** a button is pressed twice 450 ms apart at the same pixel
- **THEN** the second press is a double click

### Requirement: Confirmation dialogs close off their buttons
The arena's Dismiss and yes/no dialogs and the save screen's delete and reset-to-portal boxes SHALL close without an answer on a press inside them off their buttons.

#### Scenario: A press on the dialog's text
- **WHEN** the player presses on the yes/no dialog's question
- **THEN** the dialog closes and its hook does not run
