## ADDED Requirements

### Requirement: Task layer shortcuts
A task layer's shortcut SHALL qualify when every modifier it needs is held and score the number of needed modifiers plus one; the highest score SHALL fire, ties keeping the earlier entry. Shift SHALL never be a shortcut and SHALL be passed to the parent layer, as an unmatched key is, unless the layer is the root.

#### Scenario: An unmodified shortcut
- **WHEN** a layer holds a shortcut for a key with no modifiers and that key is pressed with nothing held
- **THEN** the shortcut fires

### Requirement: Task layer focus
A task layer SHALL take the first form made on it as its main form. Leaving the layer SHALL save the focused form when it belongs to the layer; returning SHALL restore it and its control while it is visible, else focus the main form, else the layer, with no control. Closing the main form SHALL close the layer.

#### Scenario: A hidden saved form
- **WHEN** the layer is left with one of its forms focused, that form is hidden and the layer is returned to
- **THEN** the main form takes the focus with no control

### Requirement: Colour picker refresh
The colour picker SHALL keep one picker per swatch (a second open raises it), take a colour swatch's colour bytes as its colour and revert source, and on every change refresh the other form (RGB or HSV) once under a guard, with the engine's colour conversions. A refresh from RGB SHALL write the colour back to a colour swatch as saturated bytes and call the swatch's owner when it has one.

#### Scenario: A hue change
- **WHEN** the hue slider moves to half its range
- **THEN** the colour becomes the engine's HSV to RGB of the new hue with the old saturation and value, the field corners follow, and the swatch takes the colour's bytes

### Requirement: Popup list rows
The combo box's popup list SHALL show up to sixteen six-unit rows; more SHALL cut it at sixteen with a vertical scrollbar and a scroll step of `1 / (count − 16)` worked in double precision. A press SHALL arm the current row; the release SHALL write the selection back to the owner and close the list. Only the selection SHALL draw a band.

#### Scenario: Twenty rows
- **WHEN** a list of twenty items is laid out at unit 4
- **THEN** it is sixteen rows tall plus twice its border of 3, with the scrollbar and a step of a quarter

### Requirement: Palette swatch
The palette swatch popup SHALL be a fixed 400 × 112 grid of 32 columns of 8 three-unit cells (index = column · 8 + row) in the dye palette's lifted colours. Hovering a cell SHALL select it; a selection SHALL show its colour, with the index in the label's alpha byte, and its index in the engine's hex ("0" for zero, else two upper-case digits) on the opener's label and call the owner. A cancel SHALL restore the index the popup opened with.

#### Scenario: A hovered cell
- **WHEN** the pointer hovers the cell of index 42 at unit 4
- **THEN** the index becomes 42, the label reads "2A" and the slot answers 3

### Requirement: Tone picker
The character creator's tone picker SHALL draw a 2 × 2 checkerboard of quads over nine colours built from the creator's skin tones (each tone's value lowered by 0.1, times 2.2, raised to 1.5, turned to RGB), and its picked colour SHALL be the skin tone before encoding. A refresh SHALL go to the owner's callback or tint the character's skin, moving a skin whose material level is under 0.1 a quarter of the way to the pale tone.

#### Scenario: The middle of the field
- **WHEN** the tone picker refreshes at its starting point
- **THEN** its colour is the skin tone of the middle of the grid before the encode

### Requirement: Hair picker
The hair picker SHALL draw three columns blending the stored hue run (dark red, blonde, reddish, dark red) under a shade from black to white at 1.2 times its height; its colour SHALL be the run blended at 3u, rounded to even, scaled by `v/2 + 1/4`, its marker a grey of `140·v`. It SHALL attach a control 120 pixels below itself, as wide and 40 tall, starting at a quarter, and re-shade an attached hair material on refresh.

#### Scenario: The starting point
- **WHEN** the hair picker is made
- **THEN** its colour is (84, 61, 37) and its marker grey 70

### Requirement: Picker point
A creation picker SHALL take a press over its field as `u = x / width` and `v = 1 − (1 − y / height)`, worked in double precision and held to 0–1, ignore a point within an epsilon of the present one, and answer 7.

#### Scenario: Outside the field
- **WHEN** a 160-pixel tone field is pressed at (−100, 1000)
- **THEN** its point becomes (0, 1)

### Requirement: Skill atlases
The skill atlases SHALL be cut as the origin cell, then five sets of eight 96-pixel cells in two rows of four (the origin row the techniques 5–8), as 8-unit icons sharpened when smaller than 96 pixels, and six 288-pixel cells as 24-unit icons; a positive skill page SHALL place four groups at a 72-unit radius stretched 1.25 across (64 and 0.9 with the inset layout).

#### Scenario: Unit 8
- **WHEN** the atlases are built at unit 8
- **THEN** the skill icons are 64 pixels and sharpened

### Requirement: Manual and tips
The manual SHALL open the keyboard or the controller manual by the input mode, list its topics in the directory's own positions with title-cased titles, and show a topic's text without its last byte. A one-time tip SHALL show only for a single bit (0–30) not in the settings mask, as a 504-pixel popup with "Continue" and "Don't show again".

#### Scenario: A topic title
- **WHEN** a directory record's title is "getting started"
- **THEN** the list shows "Getting Started"

### Requirement: Item view drop and rename slots
An item view's select slot SHALL set its hovered entry; dragging over it SHALL refuse a drag that started on the view itself or a view whose state turns drops off; its drop hooks SHALL answer 0 without a hook; a rename SHALL place the caret from the pointer over the entry's line.

#### Scenario: A drag from the same view
- **WHEN** an item dragged from a view is held over that view
- **THEN** the view refuses the drop
