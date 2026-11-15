## MODIFIED Requirements

### Requirement: Right-release close
A right-button click release over a loot window, the inventory or the journal SHALL close it; over the journal's text box the release SHALL close the journal too.

#### Scenario: Release over the inventory
- **WHEN** the right button is released over the focused inventory
- **THEN** the inventory closes

#### Scenario: Release over the journal's text
- **WHEN** the right button is released over the journal's text box
- **THEN** the journal closes and stores its text in the selected entry

## ADDED Requirements

### Requirement: Journal text box
The journal SHALL hold an editable text box at (0, 88), 228 × 412, with no border, a vertical scroll bar and the small font, whose margin is the GUI unit scaled by the show's rescale from unit 4. While the window manager's focus is on it, the keys SHALL go to the box and not to the player's hotkeys.

#### Scenario: Typing a note
- **WHEN** the pointer is over the journal's text and the player types "Well", Enter and "dry"
- **THEN** the box holds "Well\r\ndry" and the journal stays open

### Requirement: Journal text box pointer and caret
A left press on the journal's text SHALL put the caret under the pointer and light it; a held press past the drag threshold SHALL select by dragging; the scroll bar and the wheel SHALL set the box's scroll target, which the drawn scroll eases to. The caret SHALL blink every fifth frame while the box has the focus and be drawn only while lit.

#### Scenario: A press before the first letter
- **WHEN** the player presses at the text's top-left corner
- **THEN** the caret goes before the first byte and is drawn

### Requirement: Journal Tab callback
Tab in the journal's focused text box SHALL turn combat mode on, which closes the screens.

#### Scenario: Tab while writing
- **WHEN** the player presses Tab in the journal's text
- **THEN** combat mode turns on

### Requirement: Journal buttons and entry list
The journal SHALL carry three records: New, Erase and the entry list, a combo box 220 wide whose caption is the selected entry's title. New SHALL store the text, add a "New Entry" and show it empty; Erase SHALL remove the selected entry (blanking the only one) and show the entry left.

#### Scenario: New then Erase
- **WHEN** the player writes "first", clicks New, then clicks Erase
- **THEN** a second empty entry is shown, then removed, and "first" is shown again

### Requirement: Manual topic list
The manual's topic list SHALL be an item view rescaled to the GUI unit (rows of five units). The pointer's entry SHALL be its hover; a click SHALL select an entry; while the list has the focus the arrows, Home, End and typed letters SHALL move the selection. A selection change SHALL show the topic and focus the page.

#### Scenario: Click then Up
- **WHEN** the player clicks the second topic, then presses Up
- **THEN** the second topic is shown, then the first, and a second key press reaches the read-only page and changes nothing

### Requirement: Manual page box
The manual's page SHALL be a read-only text box with markup and a margin of two units, kept between frames; its scroll bar and the wheel SHALL set its scroll target and the drawn scroll SHALL ease to it.

#### Scenario: Scrolling a long page
- **WHEN** the player turns the wheel over a page longer than its view
- **THEN** the page eases down by whole lines

### Requirement: Form record hold
A press on a form's record SHALL hold the record by its 1-based index only when the record answers with both bits of 5, and the release SHALL act on the held record whose index the hit code's low word matches, so a button's release over it reports to its callback.

#### Scenario: Clicking New
- **WHEN** the player presses and releases the left button over the journal's New button
- **THEN** the button's callback runs once
