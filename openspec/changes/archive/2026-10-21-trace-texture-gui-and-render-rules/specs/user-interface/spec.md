# Spec Delta

## ADDED Requirements

### Requirement: GUI blend switch
The GUI blend switch SHALL keep the last word it was given: a word equal to the last does nothing; the word `0x100` under the mask `0x3f0f` turns blending off; any other word turns it on (when the last word was the disabling one) and sets the source factor from bits 8 to 11 and the destination from bits 0 to 3 of the recorded ten-entry table, nibbles past the table reading zero except nibble 12, which reads the cached last word.

#### Scenario: Repeating a word
- **WHEN** the same blend word is applied twice
- **THEN** the second application changes nothing

### Requirement: Skin globals
The GUI globals SHALL make the default skin once, prepare a skin on its first activation only (the flag lives on the skin), and hand the activated skin the alpha and fade pair as last set (zero before the first set). Setting the alpha SHALL store it unclamped and the alpha byte SHALL be `alpha * 255` rounded to even in 64 bits, low byte.

#### Scenario: Switching back
- **WHEN** skin A is activated, then B, then A again
- **THEN** A is prepared once

### Requirement: Textured skin
Binding a skin texture different from the current one SHALL set `TexCol` to 1 when the texture word has a `0xc000` bit and 0 otherwise, writing the uniform only in renderer mode 2. The per-frame `TexScl` SHALL be the screen size over 512, set unless the renderer already is in mode 4, the mode that setup leaves behind. The game skin's atlas SHALL be resampled to 40 by 20 units.

#### Scenario: A texture word with the 0x8000 bit
- **WHEN** a different texture with bit `0x8000` is bound in renderer mode 2
- **THEN** `TexCol` is 1

### Requirement: Table view hover, centre and edit
Hovering a row SHALL make it the hovered row and answer 3; hovering no row SHALL leave the hovered row as it was and answer the form's reply with bits 3 added while a row is hovered. The default row SHALL be the first when none is selected, and the table's centre SHALL be half its client's width and height. A press inside an open edit SHALL put the caret and anchor together at the pointer; a drag SHALL move only the caret and scroll by the first column's width.

#### Scenario: Dragging a selection
- **WHEN** an edit holds the whole text selected, is pressed at its start and the pointer is dragged to the right
- **THEN** the anchor stays at the start and the caret follows

### Requirement: Table view draw
Each header cell SHALL report an inset rectangle two pixels in from its sides and top and two pixels short of six units down, and the table SHALL report a shadow starting at the header's bottom line, as wide as the client and as tall as the client's height less six units, negative for a shorter client.

#### Scenario: A short client
- **WHEN** the client is 20 tall at unit 5
- **THEN** the shadow's height is -10
