## ADDED Requirements

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
