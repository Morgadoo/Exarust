## ADDED Requirements

### Requirement: Combo box edit state
An editable combo box (style bits 0xd) SHALL answer the pointer query with the text cursor word 0x8020000 for a point left of its arrow (x below its left edge plus its width less (4 units | 1) less its pad), and 0 otherwise, so the record's hover bits take 0x20000 over its text and 0x10000 elsewhere.

#### Scenario: The pointer over the journal list's text
- **WHEN** the pointer rests on the left part of the journal's entry list
- **THEN** the list's state bits 0xf0000 read 0x20000

### Requirement: Combo box press
A left press on a combo box in its edit state SHALL answer 0x15, so the box holds the press and takes the key focus, and the caret and anchor go under the pointer; any other left press SHALL open the drop-down list and answer 3; the release SHALL only answer 3.

#### Scenario: The arrow and the text
- **WHEN** the journal list is pressed on its arrow, then on its text
- **THEN** the first press opens the list of titles and the second places the caret and opens nothing

### Requirement: Journal list rename
The journal's entry list SHALL take the key focus with its whole title selected when New adds an entry, and with no selection change from a press on its text. While it has the focus its caret SHALL blink and draw, a held drag SHALL select, and every key but Tab SHALL be its key message; each edit SHALL call its callback in the edit state, which makes the list's text the selected entry's title and its row in the list's titles.

#### Scenario: Naming a new entry
- **WHEN** New is clicked and Camp is typed
- **THEN** the new entry's title is Camp and the list opens with New Entry and Camp

### Requirement: Combo box caret draw
A combo box with the focus bit (state 0x400) SHALL draw its selection, and its caret where the caret is when its form's caret bit is lit.

#### Scenario: The focused journal list
- **WHEN** the journal list has the focus and its form's caret bit is lit
- **THEN** the caret glyph draws at the caret in the list's text ink

### Requirement: Combat entry releases the GUI drag
Entering combat mode SHALL release the GUI's held left button before the screens close: the window manager's held press ends, and an item drag in flight is released at the pointer, dropping where a panel under the pointer takes it and cancelling elsewhere.

#### Scenario: A drag in the pack
- **WHEN** combat mode is entered while an item is dragged over a free spot of the pack
- **THEN** the item lands there and the drag ends before the inventory closes

### Requirement: Pad words to an open popup
While a hub combo list or colour palette is open, the pad's menu word SHALL go to it first: a navigation step moves its selection, MenuSel chooses the list's selected row (written back to its combo) or keeps the palette's pick, and MenuBak closes it, the palette restoring its opening index.

#### Scenario: The merchant's list by pad
- **WHEN** the Look for list is open and the pad steps down twice and presses MenuSel
- **THEN** Weapons is chosen and the merchant looks for weapons

### Requirement: Board buttons in the pad pick
The arena board's bottom buttons (End Day, Close, Enter, Forfeit) SHALL be pad pick candidates beside its tables when the pad reaches them (shown, enabled, the pad focus style), and MenuSel on the focused one SHALL run the callback its click runs.

#### Scenario: Close by pad
- **WHEN** the pad steps down from the lineup to the Close button and presses MenuSel
- **THEN** the board takes a click on Close

## MODIFIED Requirements

### Requirement: Journal entry list drops down
The journal's selection SHALL keep the entry list's 1-based row, the entry count and every title; a press on the list's arrow SHALL open its drop-down list of those titles under it with the row chosen, and a row picked there SHALL store the text box's text into the selected entry and select the picked entry, whose title and text the list and the box then show.

#### Scenario: Picking the first entry
- **WHEN** the journal has two entries, the second selected with text typed, and the player opens the list and picks its first row
- **THEN** the second entry keeps the typed text and the first entry's text shows
