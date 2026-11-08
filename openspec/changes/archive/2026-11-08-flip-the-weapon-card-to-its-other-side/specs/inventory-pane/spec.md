## ADDED Requirements

### Requirement: Weapon card flip button
A weapon card whose weapon has its flip byte set SHALL show the Flip weapon button after its striking rows: an image button (style 0x80c00080, radio group 1) drawn with the line skin's button frame and the grip sheet's first cell, made at eight units a side in the grip manager's format, centred in the button in the label group's seventh shade (its eleventh while on).

#### Scenario: The falchion's button
- **WHEN** exanima01's falchion card is open
- **THEN** the grip icon's button stands under its Slash row

### Requirement: Weapon card flip press
A left press on the Flip weapon button and the release over it SHALL toggle it, and the card's striking rows SHALL then show the weapon's second side while it is on and its first while it is off.

#### Scenario: The falchion
- **WHEN** exanima01's falchion card is open and its Flip weapon button is clicked
- **THEN** the button turns on and the Slash row (four dots) becomes a Crush row of one and a half dots

### Requirement: Weapon card side rows
Turning the flip button SHALL rewrite the striking rows in place from the Slash row on: each non-zero byte of the side shown (slash, crush, pierce; the second side six bytes after the first) SHALL take the next row's label and its value (the byte × 0.05) and show it, and the rows with the ids after the last written one up to Pierce's SHALL be hidden but keep their place. A card without a Slash row SHALL be left as it is.

#### Scenario: Fewer rows on the other side
- **WHEN** a card built with Slash and Pierce rows flips to a side with only crush 20
- **THEN** the first row reads Crush at 1.0 and the Pierce row is hidden; flipping back shows Slash and Pierce again

## MODIFIED Requirements

### Requirement: Card refresh rows
The refresh SHALL write an apparel card's coverage, impact, slash, crush, pierce and encumbrance rows (encumbrance 8 at 0xa1 or above) and its value, or a weapon card's striking rows from Slash's bar on for each non-zero byte of the side the card's flip button shows, its thrust and its value, from the record's bytes; a row the card does not have SHALL be skipped.

#### Scenario: Apparel rows
- **WHEN** an apparel card with only a Slash row and value 1 is refreshed from slash 11 and a value of 4 copper
- **THEN** its Slash row reads 0.55 and its value 4
