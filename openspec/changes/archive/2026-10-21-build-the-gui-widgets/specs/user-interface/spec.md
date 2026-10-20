## ADDED Requirements

### Requirement: GUI fonts
The system SHALL lay interface text out with the original's six GUI fonts (`Rayform06R`, `Rayform08R`, `Rayform10R`, `Rayform12R`, `Rayform16R`, `fontbase24r`), each with its recorded glyph quad, centring height, slant, gap, line height and per-byte advances (the stored widths plus the gap).

#### Scenario: Recorded advances
- **WHEN** `fontbase24r` measures a space, "A", "i" and "m"
- **THEN** they advance 21, 28, 12 and 37, its quad is 36 × 48 and its line 48
- **WHEN** `Rayform06R` measures "lW"
- **THEN** the width is 10

### Requirement: GUI scale levels
A GUI scale level, clamped to 0 through 5, SHALL set the unit (3, 4, 5, 6, 8, 12), the main font (06R, 08R, 10R, 12R, 16R, fontbase24r), the small font (06R at the two lowest levels, else one size down) and the cells of the large and small icon atlases (16, 20, 28, 32, 42, 64 and 12, 16, 20, 24, 32, 48).

#### Scenario: Level 2
- **WHEN** the level is 2
- **THEN** the unit is 5, the fonts are `Rayform10R` and `Rayform08R`, and the cells are 28 and 20

### Requirement: Text line breaks
A line SHALL end after a carriage return, after the last space that fits the width, or, with no space, after the byte that overflows it; a following line feed SHALL be skipped. A text under two bytes SHALL count one line, and a text whose second-to-last byte is a carriage return one more.

#### Scenario: Breaking
- **WHEN** every byte is 10 wide and a space 5, and "ab cd" wraps at 30
- **THEN** the first line ends after the space
- **WHEN** "abcdef" wraps at 30
- **THEN** the first line ends after "d"

### Requirement: Text markup
With markup, a `[` SHALL open a tag that takes no width: `[tcol=RRGGBB]` pushes a colour (at the skin's tag alpha; eight hex digits give their own), `[/tcol]` pops it, `[scrp=i]` slants the glyphs by the main font's slant, and `[algn=2]` moves its line right by half the width left. A tag closed by the text's last byte SHALL lose its last letter, as in the original.

#### Scenario: Colour and centring
- **WHEN** "a[tcol=ff0000]b[/tcol]c" is drawn from x 7 with 10-pixel bytes
- **THEN** "b" is at 17 in red at the tag alpha and "c" at 27 in the text colour
- **WHEN** "[algn=2]ab" is drawn 100 wide
- **THEN** "a" is at 40

### Requirement: Text pictures and spans
`[rimg=NAME]` SHALL draw a picture its size over 6 in units, centred in the frame, moving the text below it down by its height less a pitch. `{%` SHALL start a span slanted by the main font's slant in (180, 200, 240) that ends after `%}`.

#### Scenario: A picture
- **WHEN** a 60 × 60 picture is drawn at unit 5 in a 100-pixel frame
- **THEN** it is 50 × 50 at x 25, and the next line starts 50 pixels down

### Requirement: Key names in text
`[inptN]` (one or two digits) SHALL be replaced by the name of the key bound to control N in title case, underscores turned to spaces.

#### Scenario: A binding
- **WHEN** control 3 is bound to "SPACE"
- **THEN** "Press [inpt3] now" reads "Press Space now"

### Requirement: Caret moves in text
Home and End SHALL go to the wrapped line's start and its end before the break; Up, Down and the page keys SHALL keep the pen offset, picking the nearer side of the byte under it, and scroll the target line in as the first line or the second-to-last; the pointer SHALL place the caret before the first byte whose middle is right of it, its line being y over the font's line height.

#### Scenario: Down keeps the pen
- **WHEN** "abc def ghi" wraps at 40 and the caret is before "e"
- **THEN** Down puts it before "h"

### Requirement: Text box editing
The text box SHALL insert typed bytes at the caret (replacing a selection), insert `\r\n` on Enter, and delete a `\r\n` as one with Backspace or Delete; Left SHALL pass a `\r\n` from after it, and Right from before it SHALL stop between the two; Shift SHALL keep the anchor; a read-only box SHALL take no keys. Enter over a selection SHALL grow the text by the length before the cut, as the original does.

#### Scenario: A line break
- **WHEN** "ab", Enter and "cd" are typed
- **THEN** the text is "ab\r\ncd" in two lines, and Backspace after the break removes both bytes

#### Scenario: Enter over a selection
- **WHEN** "abcdef" has "bc" selected and Enter is pressed
- **THEN** the text is 8 bytes and starts "a\r\ndef"

### Requirement: Text box clipboard and Tab
Ctrl+C, Ctrl+X and Ctrl+V SHALL copy, cut and paste the selection; Ctrl+Home and Ctrl+End SHALL go to the text's ends and scroll there; Tab SHALL reach the owner only when it takes Tab.

#### Scenario: Cut and paste
- **WHEN** "world" of "hello world" is cut and pasted at the start
- **THEN** the text is "worldhello " with the caret after "world"

### Requirement: Text box scroll
The text box SHALL count its lines at its width less a unit and its margin each side (200 units when it sizes itself), pitch them at its spacing in units plus the font's line height, scroll over the lines that do not fit in whole-line steps, snap a set scroll to steps, ease the drawn scroll a fifth of the way per frame (snapping within 0.001), and, while dragging, scroll at most every 180 ms. A box that grows SHALL take the content height plus its top and left insets.

#### Scenario: Five lines in three
- **WHEN** a 60-pixel box with `Rayform10R` holds five lines
- **THEN** the content is 100 pixels and a step is half the scroll

### Requirement: Icon atlas
An icon atlas SHALL put an icon in its first free cell; a full atlas SHALL double its shorter side, its height first, a wider atlas repacking its cells in order; the first icon SHALL open a square the next power of two of the cell; a handle SHALL be the cell's index plus one.

#### Scenario: Growth
- **WHEN** a 16-pixel atlas takes three icons
- **THEN** it is 16 × 16, then 16 × 32, then 32 × 32 with the second icon moved to the first row

### Requirement: Icon draws
Icons SHALL draw as one cell-sized quad, plain, turned by the recorded turn table, mirrored, outlined (the icon in the face colour, then four outline copies a pixel left, right, below and above) or glowing (four copies two pixels out at half alpha, then the icon in white).

#### Scenario: An outline
- **WHEN** an icon is outlined at (5, 6) at alpha 0.5
- **THEN** five quads draw: the icon, then copies at x 4 and 6 and at y 5 and 7, their alphas halved

### Requirement: Built-in glyphs
The interface SHALL draw twelve glyphs into its small atlas at the GUI unit, in order: a cross (or the pack's symbol), right, left, up and down chevrons, three window frames, minus and plus boxes, a drop-down chevron over a bar and a tick; strokes are bright with dimmer fringes, a fringe over a fringe halving the first and adding its own.

#### Scenario: The minus box at unit 5
- **WHEN** the minus glyph is drawn into a 20-pixel cell at unit 5
- **THEN** its bright bar is pixels 4 through 10 of row 12 from the bottom

### Requirement: Icon hits
A colour icon SHALL be hit by its alpha: at a point, its alpha over 255, the row read being the cell height less y up from the cell's bottom; near a point, any alpha within a unit of it, the square cut to the cell at its low edges and at its high edges only when the low edge is inside.

#### Scenario: One opaque pixel
- **WHEN** a 16-pixel icon has one opaque pixel at column 4, row 10 from the bottom
- **THEN** the alpha at (4, 6) is 1 and a press within 2 pixels of (6, 8) hits it

### Requirement: Table view rows
A table SHALL draw rows 6 units tall under a 6-unit header, scroll over all its rows but one, and not hit the header or the rows behind it. Down SHALL pass greyed rows and Up SHALL not. Rows SHALL keep the order they were added in.

#### Scenario: Rows and hits
- **WHEN** a unit-5 table 200 pixels tall has five rows
- **THEN** the content is 320 pixels, y 29 hits nothing and y 30 hits row 1

#### Scenario: Down over a greyed row
- **WHEN** row 2 is greyed and row 1 is selected
- **THEN** Down selects row 3 and Up then selects row 2

### Requirement: Table view presses
A press SHALL select a row that is not greyed; a second press SHALL edit an editable cell (all of it selected) or open the column's choice list; a double press SHALL activate the row; a press past the last row of a growing table SHALL add a row and edit it; committing an emptied first cell of the last row SHALL delete that row.

#### Scenario: Select then edit
- **WHEN** row 2 is pressed twice on its editable first cell
- **THEN** it is selected, then its text is edited with all of it selected

### Requirement: Item view layout
An item view SHALL lay its entries out as a grid (pitch the icon plus 4 units, rows of 6 units more and a line, names cut under the icon to one byte fewer than fit at the gap, with an ellipsis), a list (one per row of the icon plus a unit) or columns (the icon plus 2 units apart) that scroll sideways, and SHALL render one missing visible icon per request.

#### Scenario: A grid
- **WHEN** a unit-5 grid 200 pixels wide holds seven entries
- **THEN** icons are 20, the pitch 40, rows 70 and four entries share a line

### Requirement: Item view input
An item view SHALL rename on a second press, activate on a double press, toggle marks in check mode (on the selected entry setting every other flag bit, as the original), and jump to the next entry whose name starts, case-blind, with the letters typed within 600 ms.

#### Scenario: Type-ahead
- **WHEN** "S" is typed and then "h" within 600 ms
- **THEN** the selection goes to "Sword" and then to "shield"

### Requirement: Basic controls
The interface SHALL know the seven control classes (button 80 × 24, check box 16 × 16, image button 28 × 28, trackbar 16 tall, label 20 tall, icon button and picture 28 × 28) and answer their messages: hover lights a control that is not on, a release over it toggles it (a radio group keeps one on; a three-state box cycles off, on, partly on), and a trackbar takes its value from the pointer past its margin and half its knob over its travel, quantised to its steps, or steps by keys and the stick.

#### Scenario: A trackbar
- **WHEN** a 100-pixel trackbar at x 10 has its 22-pixel knob and 2-pixel margin
- **THEN** the pointer at 60 gives 37 / 74

### Requirement: GUI level in the app
The app SHALL keep the GUI level with the GUI scale; a change SHALL empty both icon atlases, give them the level's cells and draw the glyphs again, and the glyph atlas SHALL be kept as an upright texture.

#### Scenario: A larger window
- **WHEN** the scale goes from 5 to 8
- **THEN** the atlases' cells become 42 and 32 and the glyphs are drawn again at unit 8
