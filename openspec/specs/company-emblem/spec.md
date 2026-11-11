# company-emblem Specification

## Purpose
The company emblem a name can select, and the picture that index names.

## Requirements

### Requirement: Klean preset
The name klean SHALL select background 52 and design 81, give the four colours the palette indices 7, 0, 0 and 6 and clear the layer word. Any other name SHALL select nothing. Background 52 SHALL be named emblembkg052. Design 81 SHALL be named emblemdes081.

#### Scenario: The preset and its pictures
- **WHEN** the name klean is read, and the names Klean and an empty name are read
- **THEN** klean selects background 52, design 81, the palette indices 7, 0, 0 and 6 and no mirror, those pictures are named emblembkg052 and emblemdes081, and the other two names select nothing

### Requirement: Emblem editor start and roll
The emblem editor SHALL start on background 1, design 1, no mirror and the palette indices 0xd3, 0x72, 0x5a and 0x4e, and its randomiser SHALL step `state · 0x8088405 + 1`, take each draw as the high half of `state · n`, roll only the colours' palette bytes, and keep four records of history.

#### Scenario: Seed 1
- **WHEN** the randomiser is seeded with 1
- **THEN** the palette indices are 8, 220, 66 and 70, no mirror is set and the pictures are background 22 and design 7

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

### Requirement: The emblem name submits klean
The emblem name field's callback SHALL run on every key but act only on Enter's submitted bit: the name klean then selects the preset. A picture number field SHALL select its number on Enter.

#### Scenario: klean typed, then Enter
- **WHEN** klean is typed into the name field, and then Enter is pressed
- **THEN** the pictures stay 1 and 1 until Enter, and then are 52 and 81

### Requirement: Emblem swatch drop
An emblem swatch taking a dropped colour SHALL store the whole colour word, palette index in the alpha byte included, in its colour slot without a refresh.

#### Scenario: Slot 4 takes slot 1's word
- **WHEN** the first swatch's colour is dropped on the fourth
- **THEN** colour 4's word equals colour 1's
