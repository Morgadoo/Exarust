## MODIFIED Requirements

### Requirement: Klean preset
The name klean SHALL select background 52 and design 81, give the four colours the palette indices 7, 0, 0 and 6 and clear the layer word. Any other name SHALL select nothing. Background 52 SHALL be named emblembkg052. Design 81 SHALL be named emblemdes081.

#### Scenario: The preset and its pictures
- **WHEN** the name klean is read, and the names Klean and an empty name are read
- **THEN** klean selects background 52, design 81, the palette indices 7, 0, 0 and 6 and no mirror, those pictures are named emblembkg052 and emblemdes081, and the other two names select nothing

## ADDED Requirements

### Requirement: The emblem form's pictures
The emblem form SHALL be built from its 23 records. A step button or a number field SHALL select the background or design through the setter: at or under 1 it is 1 and the step-down button is disabled; at or over the maximum it is the maximum and the step-up button is disabled; otherwise both are enabled; the number is written back into the field.

#### Scenario: A step up
- **WHEN** the background's step-up button is pressed on a new form
- **THEN** the background is 2 and its field reads 2

### Requirement: The emblem form's mirrors and colours
A mirror box SHALL set its layer bit while on. A colour swatch SHALL open the palette swatch popup with its palette index under the pointer, and a cell taken SHALL become that colour's palette index.

#### Scenario: Mirror and a colour
- **WHEN** the background's mirror box is ticked, the first swatch opened and the cell in column 2, row 3 taken
- **THEN** the layer word has 0x10 and colour 1's palette index is 19

### Requirement: Randomise and Previous
Randomise SHALL keep the record, enable Previous, roll and select both pictures through the setter. Previous SHALL take the newest kept record back and disable itself when none is left.

#### Scenario: Back one roll
- **WHEN** Randomise is pressed twice and Previous once
- **THEN** the record is the first roll's

### Requirement: The company dialog
The new career's company dialog SHALL hold the emblem form with BACK and NEXT. NEXT with an empty name SHALL refuse ("Please name your company") and keep the dialog; otherwise it SHALL store the name and the emblem record.

#### Scenario: Naming the company
- **WHEN** NEXT is pressed with no name, then after typing Ravens
- **THEN** the first press keeps the dialog and the second names the company Ravens with the form's emblem
