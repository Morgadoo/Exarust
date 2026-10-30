# Spec Delta

## ADDED Requirements

### Requirement: Icon rescale
When the interface unit changes, the item icon manager SHALL rebuild its atlas at forty units a side and render every cache slot that holds an id again; until its render finishes a slot SHALL have no cell, and the finished render SHALL set its half extents to half its measured size, rounded to even, plus the unit. An unchanged unit SHALL do nothing.

#### Scenario: Unit five
- **WHEN** the unit goes from 4 to 5 with one held icon
- **THEN** the atlas cell is 200, the icon has no cell and waits for a render, and its provisional half extents are 105

### Requirement: Card socket words
The card facts SHALL carry the four little-endian socket words of the section the card shows (the weapon section, else the apparel body); bytes past the section SHALL read as zero.

#### Scenario: A weapon's sockets
- **WHEN** the weapon section's socket bytes are 80 02 84 C8 91 07 00 00
- **THEN** the words are 0x0280, 0xC884, 0x0791 and 0
