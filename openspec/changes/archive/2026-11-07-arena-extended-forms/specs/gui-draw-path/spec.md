## ADDED Requirements

### Requirement: Extended controls on table-built forms
A table-built form SHALL report a landed release of an extended-class record (text field, combo box, colour swatch, spinner, image) to its owner without changing its on bit or radio group. A combo box record SHALL show its 1-based selected item, and its report SHALL open the drop-down list under it, whose pick writes the row back.

#### Scenario: A combo box's list
- **WHEN** a combo box over three items is released on and the third row taken
- **THEN** the list opens under it and the box shows the third item

### Requirement: The colour picker form
The colour picker form SHALL be built from its 17 records (0x192 × 0x11a, "Colour"): the RGB and HSV trackbars send their channel's value, the spinners their value over 255, the image its point as hue and saturation, the old swatch and Reset revert, and Close finishes; each refresh SHALL be written back into the trackbars, spinners and swatches.

#### Scenario: A spinner step
- **WHEN** the picker opens on red and the green spinner's up button is pressed
- **THEN** the colour and the edited swatch become 255, 1, 0
