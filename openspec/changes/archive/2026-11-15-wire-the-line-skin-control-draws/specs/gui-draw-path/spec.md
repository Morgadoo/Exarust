## ADDED Requirements

### Requirement: Separator line frame
A separator record SHALL draw each of its line rectangles as the skin's four-ink line loop, one-pixel lines half a pixel in: the top in the hot-form group's first shade, the left side in its second, the bottom in its seventh and the right side in its fifth, or with style 0x40 its eighth, sixth, first and second.

#### Scenario: A Settings rule
- **WHEN** the Settings form draws one of its rules
- **THEN** the rule is a batch of eight line vertices: two in each of the four inks, top, left, bottom, right

### Requirement: Line-skinned label fronts
A label record on a line-skinned form SHALL draw no back for a frame style and SHALL pick its front as the label class does: style 0x20000000 the block draw, its line fitted to the record less the border on both sides and cut with an ellipsis (or, right-aligned, its tail kept), a record with its own colour the caller's-ink draw, else the plain draw.

#### Scenario: A key cell
- **WHEN** the Controls page draws a key cell (style 0x20003380)
- **THEN** its name is drawn through the block draw in the label group's seventh shade

### Requirement: Hovered label and button inks
A hovered record (0x10000 in its 0xf0000 bits) SHALL draw its caption through the skin's glowing text when it is a plain label or a line-skinned button (a pixel lower while lit), and in the highlight group's eighth shade at the GUI's alpha when it is a block label.

#### Scenario: The pointer on Apply
- **WHEN** the pointer rests on the Settings form's Apply button
- **THEN** its caption draws in the label group's eighth shade and four copies a pixel off in the highlight group's first shade under the additive blend

### Requirement: Line skin combo box back
A combo box on a line-skinned form SHALL draw its back as the line skin's rail: a track along the box's bottom from two units outside its left edge to two units past its right, its caps four units wide, and a thumb two units down from one to three units short of its right edge, in the disabled or idle control ink, the thumb in the highlight group's eighth shade at the GUI's alpha while hovered.

#### Scenario: The loadout's fighter list
- **WHEN** the arena loadout form draws its fighter list at unit 4
- **THEN** the rail's thumb runs from four to twelve pixels short of the list's right edge, eight pixels below its top

### Requirement: Line skin text field back
A text field on a line-skinned form SHALL draw no frame of its own; with a frame bit of its style (0xf0) it SHALL draw the line skin's capped span along its bottom less two pixels, from one pixel in on each side, in the idle control ink.

#### Scenario: A framed field
- **WHEN** a framed text field at (10, 20), 100 by 24, draws at unit 4
- **THEN** its left cap starts at (3, 34) and its middle bar at (27, 42)

### Requirement: Line skin image button
An image button on a line-skinned form SHALL draw no frame and SHALL draw its icon through the line skin's centred icon: centred in the record, in the label group's fourth shade while disabled, its seventh with the alpha raised by 0x40 while lit and its sixth otherwise for a style with bit 0x80000000, outlined while hovered.

#### Scenario: The Flip weapon button
- **WHEN** the item card draws its Flip weapon button lit
- **THEN** only the grip icon draws, in the label group's seventh shade with its alpha raised by 0x40

### Requirement: Rounded label boxes through the skin
The domain wheel's label box and the power tree's shaded boxes SHALL be the skin's rounded fills: the wheel's grown by twice four units at `0.6 · fade`, the tree's by its style times four units at its alpha byte.

#### Scenario: A wheel label
- **WHEN** the wheel's hovered domain shows its label at fade 1
- **THEN** one rounded batch draws in black at alpha byte 153

### Requirement: Dim link fade per row
A dim tree link SHALL take one alpha byte per spline row, `alpha · (2·row / segments) · 0.5` rounded to even, on both of the row's vertices, `segments` being the count the link's spline began with.

#### Scenario: The second row of a three-segment link
- **WHEN** a dim link of alpha 200 is built over three segments
- **THEN** both vertices of its second row take alpha 67
