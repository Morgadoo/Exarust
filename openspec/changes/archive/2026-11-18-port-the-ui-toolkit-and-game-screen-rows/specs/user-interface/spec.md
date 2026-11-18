## ADDED Requirements

### Requirement: Combo list placement
A combo box's drop-down list SHALL take the owner's width and its layout's height, and SHALL be placed by the GUI's child placement: at the owner's x and one pixel over the owner's bottom edge when it fits on the screen there, else with its bottom one pixel under the owner's top; flush with the screen's right edge less two when it would leave it. The list SHALL NOT be cut to the room left.

#### Scenario: A combo near the screen's bottom
- **WHEN** a combo box whose list does not fit below it opens its list
- **THEN** the list opens above it, its bottom one pixel under the owner's top

### Requirement: Hovered combo text
A combo box in the hovered state (state bits 0xf0000 at 0x10000) SHALL draw its whole text at its label origin through the skin's glowing text; any other combo box SHALL draw its text run through the edit unit's run draw.

#### Scenario: The pointer over a merchant combo
- **WHEN** the pointer is over the merchant's Look for combo box
- **THEN** its text draws in the label group's eighth shade with four additive copies in the highlight group's first

### Requirement: Pad reach of a record
The pad's pick SHALL reach a record of the focused form only when the record is neither hidden nor disabled (state 0x110 clear), carries the pad style bit 0x400000 and is of class 1 to 4, 0x12 or 0x17.

#### Scenario: A label between buttons
- **WHEN** the pad's stick points across a label towards a button
- **THEN** the pick skips the label and lands on the button

### Requirement: Retail pointer slots
The pointer table SHALL leave slots 0 to 4, 9 and 10 empty, as the shipped executable has none of the cursor resources the original loads them from, and SHALL hold the platform's hand, wait, four-way move, text beam and not-allowed pointers in slots 5 to 8 and 11. Showing an empty slot SHALL hide the pointer image.

#### Scenario: Slot 0 before the menu cursor
- **WHEN** the window shows slot 0 before the menu cursor is built
- **THEN** no pointer image shows

### Requirement: GUISymbl first glyph
The first built-in glyph SHALL be the executable's GUISymbl image's 48-pixel square at (0, 48), its alpha as the glyph's channel, resized to the small atlas's cell; the cross SHALL stand in only when the image is missing.

#### Scenario: A scale change
- **WHEN** the GUI level changes
- **THEN** the small atlas is drawn again with the GUISymbl square first
