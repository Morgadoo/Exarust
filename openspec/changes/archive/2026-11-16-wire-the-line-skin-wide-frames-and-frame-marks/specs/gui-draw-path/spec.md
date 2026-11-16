## ADDED Requirements

### Requirement: Wide line forms
A line-skinned form with the attribute bit 0x40_0000 SHALL draw, for the corner style, the wide window frame (on a held form its own ink and a top bar once more unless it sits within a unit of a titled owner's top; side bars four units from the corners, two on a held form; corner arms two units outside and six in), and for the panel style twelve quads straddling its edges a unit either side, the bars from two units in.

#### Scenario: A combo list
- **WHEN** a line-skinned combo opens its drop-down list at unit 4
- **THEN** the list's window frame's top-left corner arm starts eight pixels above and left of the list

### Requirement: Popups take the line skin
The drop-down list and the palette swatch popup SHALL draw through the line skin's form draw with their own words: the shadow, the backdrop, their content, then the wide window frame. A list opened from a line-skinned combo SHALL lose its panel style bit and rise by a unit less one.

#### Scenario: The merchant's Look for list
- **WHEN** the merchant's Look for combo opens its list at unit 4
- **THEN** the list's style word is 0x18 and it starts three pixels above the combo's bottom edge

### Requirement: Draw skin frame marks
A form that keeps the draw skin's own instance SHALL keep the frame mark under the pointer, the mark a press went down on and the button, from the draw skin's hit test on its default background's mark rectangles; the close mark SHALL draw its hot box under the pointer, its pressed box while held, and a release still over it SHALL close the form.

#### Scenario: The picker's close mark
- **WHEN** the pointer presses and releases on the colour picker's close mark
- **THEN** the picker closes

### Requirement: Default background mark icons
The default background's mark and arrow icons SHALL draw from the small glyph atlas, an active mark through the atlas's outlined draw at scale 1 in the skin's icon inks and any other plainly in the label group's seventh shade.

#### Scenario: An idle close mark
- **WHEN** the colour picker draws with the pointer away from its close mark
- **THEN** the close glyph draws in the label group's seventh shade

### Requirement: Wide title band
The line skin's hit test in the running game SHALL take the six-unit title band.

#### Scenario: The close corner six units deep
- **WHEN** a titled, closable window at unit 4 is hit 22 pixels down and two pixels from its right edge, above its client
- **THEN** the hit answers the close mark (-4)

### Requirement: Swatch backs
A colour swatch SHALL draw its back through its form's skin's control fill: on the line skin the shade (a cap, a bar and a cap a unit either side of half a unit above the top and below the bottom, in the on ink or the highlight ink under the pointer) and the colour a unit in; on the draw skin's own instance the style frames and the colour inset by the margin; both at the GUI's alpha.

#### Scenario: An emblem swatch
- **WHEN** the emblem form draws its first swatch at unit 4
- **THEN** its colour fills the swatch four pixels in from each edge

### Requirement: Table header line
A line-skinned table view SHALL draw one span bar along its header's foot, two units thick at the row height less half a unit and one, across its client at its scroll, its caps the skin's first side length long, in the idle control ink.

#### Scenario: Manage Company
- **WHEN** Manage Company draws its two tables
- **THEN** each draws one header bar two units thick

### Requirement: Skill tooltip boxes
The skill screen's tooltip boxes SHALL draw through the skin's rounded fill at radius 2.

#### Scenario: Unchanged pixels
- **WHEN** a tooltip box is drawn on the skill screen's canvas
- **THEN** its pixels equal those of the screen's former quad
