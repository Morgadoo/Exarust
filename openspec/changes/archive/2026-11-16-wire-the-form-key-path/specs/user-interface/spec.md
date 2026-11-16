## ADDED Requirements

### Requirement: Key mask word
Each key-down SHALL reach the focused form with the GUI's modifier word: a first key-down with Shift, Ctrl or Alt held SHALL recompute the word as Shift 0x20, Ctrl 0x40 and Alt 0x80, a first key-down with none held SHALL keep the word, and a repeat SHALL send the kept word with 1.

#### Scenario: Shift with Tab
- **WHEN** Shift is held and Tab goes down
- **THEN** the focused form hears Tab with the word 0x20

### Requirement: Key-up and typed words
A key-up SHALL recompute the modifier word one bit lower (Shift 0x10, Ctrl 0x20, Alt 0x40), and a typed character above 0x1f SHALL reach the focused form with the word 2.

#### Scenario: A repeat after a key-up
- **WHEN** Shift and Tab are held, a third key goes down and up, and Tab repeats
- **THEN** the repeat carries the word 0x11

### Requirement: Form key slot
A base form's key slot SHALL, with a focused record, move the focus among the tab stops on Tab and send any other key to the focused record as its key message 0xa0, taken when the answer has both low bits; a key nobody took SHALL go to the parent's key slot unless its word has a low bit, and a key that climbs past the last window SHALL reach the task.

#### Scenario: A button under the pointer
- **WHEN** A goes down over a window's button
- **THEN** the button answers 0 and the key reaches the task

### Requirement: Record key message
Of the record classes only the text field and the combo box SHALL answer the key message 0xa0; every other class SHALL answer 0.

#### Scenario: A check box
- **WHEN** a focused check box is sent the key message
- **THEN** it answers 0

### Requirement: Keys over a game screen
While a game screen (a window or its child form) has the window manager's focus, a key-down SHALL go to that form's key slot and SHALL NOT reach the player's hotkeys; with the focus on the game view the hotkeys SHALL hear it.

#### Scenario: J over the journal
- **WHEN** J goes down with the pointer on the journal's title
- **THEN** the journal stays open, and with the pointer back on the view J closes it

### Requirement: Opening keeps the key focus
The openers' activation of a game window SHALL raise it without moving the key focus, a new inventory's and a new journal's open SHALL end with the forced pointer update, and a window's hide SHALL end with it too, so the focus is the form under the pointer.

#### Scenario: A new inventory
- **WHEN** the inventory opens away from the pointer
- **THEN** the inventory is the active window and the focus stays on the form under the pointer

## MODIFIED Requirements

### Requirement: Tab on the hub's forms
A Tab on a hub form SHALL move the form's focus from its focused record to the next enabled tab stop, wrapping to the first tab stop before it, and SHALL go the other way only when the key-down's word has 0x10, so Shift with Tab (the word 0x20) steps forward; a text field it lands on SHALL take the key focus with its text selected, and the focus SHALL not follow the pointer again until the pointer moves.

#### Scenario: The emblem form
- **WHEN** Tab is pressed with the key focus on the company name
- **THEN** the background's picture number takes the key focus

#### Scenario: Shift with Tab
- **WHEN** Shift+Tab is pressed with the key focus on the company name
- **THEN** the background's picture number takes the key focus
