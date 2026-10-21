## MODIFIED Requirements

### Requirement: Main-menu pages
The system SHALL provide the title, start, in-game, arena-pause, story-pause and class pages with their exact rows and order; CONTINUE SHALL be disabled without a save, VERSUS SHALL appear only when the versus lobby is available, and the class page's EXPERT row SHALL be disabled until the expert difficulty is unlocked.

#### Scenario: Title and start
- **WHEN** the title page is built without a save
- **THEN** its rows are BEGIN, CONTINUE, ARENA, MANUAL, SETTINGS, EXIT and CONTINUE is disabled
- **WHEN** the start page is built with the versus lobby available
- **THEN** VERSUS appears before BACK and the page has five rows
- **WHEN** the class page is built before the unlock
- **THEN** NOVICE is enabled and EXPERT is disabled

### Requirement: Main-menu row advance
The system SHALL advance rows by `round(70·row_h)` in single precision, half to even, and start the first row at `round(height·origin)`.

#### Scenario: Unit row height
- **WHEN** row_h is 1.0
- **THEN** the advance is 70
- **WHEN** row_h is 0.25 or 0.75
- **THEN** the advance is 18 or 52
- **WHEN** the height is 1013 and the origin 0.37
- **THEN** the first row starts at 375

### Requirement: Main-menu row scale
The system SHALL derive `row_h` as `round(H/135)·0.125` and advance rows by `round(70·row_h)`.

#### Scenario: Live heights
- **WHEN** the client height is 1080
- **THEN** `row_h` is 1.0 and the advance 70
- **WHEN** the height is 453
- **THEN** `row_h` is 0.375 and the advance 26
- **WHEN** the height is 720
- **THEN** `row_h` is 0.625

### Requirement: Main-menu paint
The system SHALL start row baselines at `round(H·origin) + ascent`, advance by `round(70·row_h)`, place a row at `(W − width) >> 1` as an unsigned 64-bit shift, draw disabled rows grey at half alpha, and append `[` to a row with one mark and `[[` to a row with more.

#### Scenario: Title paint
- **WHEN** the title paints at 1600×1080 with origin 0.38, row_h 1 and ascent 49 with BEGIN selected
- **THEN** the first unselected row is at y 529 and BEGIN is drawn last at 459
- **WHEN** CONTINUE is disabled with one mark
- **THEN** its colour is (128,128,128,128) and its text is CONTINUE[
- **WHEN** a row is 102 wide in a 100-wide client
- **THEN** its x lands far past the client

### Requirement: Selected row last
The system SHALL draw the selected row last, in white whatever its state, and nudge it down by unit/3 while it is the pressed row; with no selection every row SHALL be drawn in order.

#### Scenario: Last row
- **WHEN** the last of three rows is selected and pressed with unit 6
- **THEN** it is drawn last at its row y plus 2
- **WHEN** the second of three rows is selected and pressed with unit 6
- **THEN** it is drawn last at its row y plus 2
- **WHEN** it is selected but not pressed
- **THEN** it is drawn at its row y

### Requirement: Main-menu activation
The system SHALL turn the selected enabled row into its action and return `None` for a disabled row.

#### Scenario: Continue and novice
- **WHEN** CONTINUE is selected without a save
- **THEN** activation returns `None`
- **WHEN** it is selected with a save
- **THEN** activation returns Continue
- **WHEN** NOVICE is selected
- **THEN** activation returns Novice, NOVICE being always enabled
- **WHEN** EXPERT is disabled
- **THEN** activation returns `None`

## ADDED Requirements

### Requirement: Main-menu hit band
The main menu SHALL hit a row when the point lies strictly inside its band: the menu font's ascent less its second origin field tall from the band's top, and the row's width from `round((W − width)·0.5)`. The first band SHALL start at `round(H·origin)` and each next one at `round(70·row_h + top)` in single precision; a disabled row SHALL take no point but advance.

#### Scenario: Title band
- **WHEN** the title page at 1600×1080 has a 100-wide first row, ascent 49 and inset 16
- **THEN** y 420 at x 800 hits row 1, y 410 and y 443 miss, and x 751 hits while x 750 misses
- **WHEN** row_h is 0.25 and the top 101
- **THEN** the next bands start at 118 and 136

### Requirement: Main-menu miss
While a manual opened from the menu covers it, the main menu SHALL keep every point; otherwise a point on no row SHALL be kept when the menu blocks and let through when it does not.

#### Scenario: Blocking
- **WHEN** a point misses every row of a blocking menu
- **THEN** the menu keeps it

### Requirement: Main-menu commands
The main menu's title and arena pages SHALL open, row by row: a story game at the class select, the story saves, the arena page, the manual, the settings and the quit; an arena game at the emblem generator, the arena saves, the practice page, the versus lobby (or the title without it) and the title. The practice page SHALL open a practice game at the novice or expert difficulty, or go back.

#### Scenario: Arena fourth row
- **WHEN** the arena page's fourth row is clicked without the versus lobby
- **THEN** the title page is shown

### Requirement: Main-menu in-game commands
The in-game CONTINUE SHALL resume outside practice and start a new practice character in practice; its EXIT SHALL save and leave an arena, quit a story, and return to the title otherwise, never quitting the application. An end page's third row SHALL finish the match, as a forfeit unless the session state has bit 4.

#### Scenario: Exits
- **WHEN** the in-game EXIT is clicked in an arena, a story and a practice game
- **THEN** the commands are the arena's exit, the story's quit and the exit to the title

### Requirement: Main-menu press and release
The running main menu SHALL select the row under the pointer, mark a pressed row, run a row's command on the release over the pressed row (and not when released elsewhere), and run the selected row at once on Enter. Escape SHALL go back from the arena and practice pages and close the menu elsewhere.

#### Scenario: Release elsewhere
- **WHEN** a row is pressed and the pointer is released over another row
- **THEN** no command runs and the press mark clears

### Requirement: Window focus
The window manager SHALL let only the modal form (or no form) take the focus while one is set, clearing the focus otherwise. A new form SHALL deselect the old one and start with no control; a control the form has not already selected SHALL be selected and stored, and a control it has SHALL leave the stored control as it was.

#### Scenario: Reselecting
- **WHEN** a form already on control 2 is focused with control 2 while the stored control is 1
- **THEN** nothing is selected and the stored control stays 1

### Requirement: Tooltip request
A select answer with bit 0x100 SHALL request a tooltip for the focused form and control, stamped now, or 400 earlier when a tooltip shows and the answer lacks 0x200. The tick SHALL show it once 400 clock units have passed if the focus is still on it, and clear the request either way; a press or the tooltip's own time SHALL hide a showing tooltip with a 256-unit fade.

#### Scenario: Delay
- **WHEN** a hinted control is focused at 1000
- **THEN** the tick at 1400 shows nothing and the tick at 1401 shows its tooltip

### Requirement: Modal form
The modal form SHALL take every hit, in its own coordinates; a press whose target is another form (through a capture) SHALL close it and clear it.

#### Scenario: Outside point
- **WHEN** a modal form is set and the pointer is outside it
- **THEN** the hit is still the modal form, with no control

### Requirement: Pointer drag
With a button held, the pointer SHALL start a drag once it moves more than `(200 − elapsed)/20` times the drag distance from the press point during the first 191 clock units, and the distance itself after; the distance SHALL be one pixel per 540 of screen height, rounded half to even, at least 2.

#### Scenario: Threshold
- **WHEN** a press at x 100 with distance 2 is followed at once by moves to 120 and 121
- **THEN** 120 does not drag and 121 does

### Requirement: Double click
The same button pressed again within 450 clock units and one pixel of the press point SHALL send the double-click press, then the plain press when that is answered 0 or 0x10, and otherwise swallow the button's release.

#### Scenario: Window
- **WHEN** a second press comes 449 units after the first at the same point and the form answers 0
- **THEN** the form receives the double-click press and then the plain press

### Requirement: Press and release dispatch
A form answering a press with anything but 0x40 SHALL hold it, come to the front, and take the capture when the answer has bits 1 and 4. A release of the held buttons SHALL click (0x2000) a press nobody held that did not drag, send a held press's release to its form, and let a capture keep itself on 0x40 and drop on 0x80; a release of other buttons SHALL do nothing.

#### Scenario: Unheld click
- **WHEN** a form answers its press with 0 and the button is released
- **THEN** it receives the click word with 0x2000

### Requirement: GUI level lengths
The level switch SHALL derive the title as six units less two, a title-bar control as four units and one, the controls' step as half a unit (shifted) and a control, and a grip as four units less one, from level + 3, or 8 at level 4; level 5 SHALL then raise the unit to 12 with the other lengths kept.

#### Scenario: Level 5
- **WHEN** the level is 5
- **THEN** the unit is 12, the title 46, a control 33 and a grip 31

### Requirement: Form clamp and toggle switch
A clamped form SHALL keep x at 0 or more, move flush with the screen's right edge when it overflows, keep y at least a unit, and keep its size. Switching a toggle on SHALL clear the on and lit bits of every on control of its group, itself included, reporting each unless quiet, then set it on with its lit bit following.

#### Scenario: Overflow
- **WHEN** a 100-wide form at x 750 is clamped on an 800-wide screen
- **THEN** it is at x 700

### Requirement: Text truncation
Shortening a text with the ellipsis SHALL count the ellipsis' advance and then the bytes before the last; at the k-th byte that passes the width the first k − 1 bytes and the ellipsis SHALL remain. A line cut to `round(width)/gap` glyphs SHALL replace its last with the ellipsis and still advance by the replaced byte. A fitted tail SHALL be measured with the GUI level's main font.

#### Scenario: Ellipsis
- **WHEN** every byte and the ellipsis advance 10 and "abcdef" is fitted to 45
- **THEN** the result is "abc" and the ellipsis

### Requirement: Text width fit
The width fit SHALL count, for each line, every byte that fits (the head of a wrapped word included), narrow the width to the average, widen it by the GUI unit until the text takes no more lines than before (a final carriage return counting one more), keep the widest line of that pass and return that pass's count. The line count of an empty text SHALL be zero.

#### Scenario: Carriage return
- **WHEN** "abc def" and a carriage return is fitted from 30 with every letter 10, a space 5 and unit 4
- **THEN** two lines are returned and the width is 30

### Requirement: Wrapped text orientation
A wrapped block SHALL draw each line's glyphs hanging down from the line's top and stop when the top passes the block's height less a line; the lines it skips SHALL be drawn at the block's top.

#### Scenario: Three lines
- **WHEN** "ab cd ef" wraps at 30 from y 0 with a 16-pixel line
- **THEN** the glyph tops are 0, 16 and 32

### Requirement: Menu font record
The menu font record SHALL name `titlefont.rfi`, hold 112-pixel cells, ascent 98, a 36-pixel space, line height 128 and a trim of 6, and rescale every metric and the advances of `A` through `a` by half the row scale, rounded half to even.

#### Scenario: Quarter scale
- **WHEN** the record is rescaled by 0.25
- **THEN** the cell is 28 and C, 70 at design, advances 18

### Requirement: Single-line edit keys
The single-line editor SHALL leave the window's end to its scroll; a byte refused at the maximum, a key below Backspace and an unknown key SHALL change nothing and not scroll; a copy or cut with nothing selected SHALL take the whole text; a paste SHALL scroll even when the clipboard had no text. The caret from a pointer SHALL walk up to the window's last byte.

#### Scenario: Copy all
- **WHEN** "hello" has no selection and Ctrl+C is pressed
- **THEN** the copied text is "hello"

### Requirement: Single-line edit draw
The editor's whole-run draw SHALL read the window's end as a count from the first visible byte; without a selection shown it SHALL draw the window in the bound colour; with one it SHALL draw the bytes before it in the state's ink, a rectangle one line tall ending a pixel below the baseline in the selection colour at half alpha, the selected bytes in the selected ink and the rest in the state's ink. A `[tcol]` tag with its own alpha SHALL keep it, zero included.

#### Scenario: Selection
- **WHEN** "abcd" with 10-pixel bytes and a 16-pixel line has b and c selected at baseline 20
- **THEN** the rectangle is 10, 5, 20 wide and 16 tall at alpha 50 of 100

### Requirement: Viewport samples
A supersampled frame SHALL render every sample of its level: sample i uses the level's point i, the share `(i + 1)/count` in double precision, `0.5 − (i + 0.5)/count` and, after the first, the blend `1 − 1/(i + 1)`; a point's jitter SHALL be `2·scale·point/size` per axis with the view's projection scales. The camera scale SHALL be `(2·fov)·0.5` over the distance held to 100–3000, a not-a-number distance staying one.

#### Scenario: Level 2
- **WHEN** level 2 is rendered
- **THEN** it takes points 3, 4 and 5 with shares one third, two thirds and one

### Requirement: Viewport frame-rate readout
The frame-rate readout SHALL refresh once more than 250 ms have passed since the last, with the frames over the elapsed seconds, two decimals and " FPS", and sit with its right end a unit inside the client, its width the main font's gap times its length.

#### Scenario: Place
- **WHEN** "57.25 FPS" is placed with an inner right of 800, border 8, unit 5, gap 6 and line height 16
- **THEN** it is at 733, 21

### Requirement: Viewport cursor actions
A view press with no control selected SHALL pick the cursor action whose mask the press word holds with the most bits in the mask's low 16, the last among equals; a running action SHALL be ended by the next press (taking it on bit 0), keep running on a release answered 0x40, and in a view drag get the moves until it answers 0x80.

#### Scenario: Low bits
- **WHEN** the masks are 0x1 and 0x10001 and the word is 0x10001
- **THEN** the later mask is picked

### Requirement: Control layout pass
The control layout SHALL place only records not yet placed, mark them in its second pass, resolve a row start by scanning back from the record above, fill a known class's size, style and default margin (1 for the icon button and picture), initialise a trackbar, snap widths and x to four pixels half to even, and right-align by the client's inner right less the border. Appending records SHALL keep those up to the last in use and drop the rest.

#### Scenario: Snap
- **WHEN** a trackbar record 50 wide is laid out at unit 4
- **THEN** it is 48 wide and 16 tall with an 18-pixel knob and margin 3
