# Spec Delta

## ADDED Requirements

### Requirement: Map parchment
The item card builder SHALL answer a card request for an item whose record holds the map section (`0x80000B03`) by opening a parchment of the image the section names (the 16-byte name at body `0x10`) instead of a card, leaving any shown card open. A parchment already open for that item SHALL be left alone. The parchment SHALL fade in over 256 ms.

#### Scenario: The first level's map
- **WHEN** the card of `exanima01`'s map (local 9, image `Map_Exanima01`) is asked
- **THEN** its parchment opens and no card is built

### Requirement: Map parchment placement
The parchment SHALL be the texture's size times the interface unit over six (each rounded half to even), placed centred across on the pointer and kept on the screen, its bottom eight units below the pointer and its top at least one unit down. A right press where the texture's alpha is at or above 0xF0 SHALL fade it out and go no further.

#### Scenario: Size and place
- **WHEN** a 768 by 816 texel map opens at unit 4 with the pointer at (640, 400) on a 1280-pixel screen
- **THEN** the parchment is 512 by 544 at (384, 4)

### Requirement: Divination sheet
Opening the divination reading SHALL load its card sheet `divcardsa1.rfi` into the interface's textures when it is not there, and the reading SHALL be painted after the screens' draw starts the frame's queue.

#### Scenario: A deck's double click
- **WHEN** a class-0 kind-5 deck in the pack is double clicked
- **THEN** the reading opens with argument 0 and its cards are drawn
