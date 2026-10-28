## ADDED Requirements

### Requirement: Tree node drag
While a node is dragged, its placed point SHALL be the pointer less the grab offset, and its base place SHALL be `(placed − shown pan)/shown zoom` in single precision, multiplied by 0.1 in double and rounded to single.

#### Scenario: A node held in place
- **WHEN** a node placed at (100, 200) at zoom 1 and pan (0, 0) is dragged with grab offset (3, 4) to the pointer (103, 204)
- **THEN** the node stays at (100, 200) with base (10, 20)

### Requirement: Tree link-turn drag
While a link's turn is dragged, the link SHALL be the one the focus id's bits 16 to 19 name (one-based) on the pressed node. With bit 21 clear the outgoing turn SHALL be the pointer's angle about the owner over 2π; with it set the incoming turn SHALL be the angle about the node the link ends on.

#### Scenario: Outgoing and incoming turns
- **WHEN** the first link of a node at (100, 200) ending on a node at (500, 500) is dragged with focus `1 << 16 | 1` to (100, 300), then with bit 21 also set to (500, 600)
- **THEN** both the outgoing and the incoming turn are 0

### Requirement: Tree flash end
The draw SHALL end a flash whose end time has come: the flashed node becomes none and the refusal flag is cleared. Until then the refusal SHALL stay set.

#### Scenario: A refused flash
- **WHEN** a refused flash on node 0 ends at 1300 and the tree is drawn at 1299 and then at 1300
- **THEN** the first draw shows the flash and the refusal, and the second shows neither

### Requirement: Tree spline sample count
A link's spline SHALL take `round((out rise + in rise)·6 + √closeness·4)` (rounding to even into a 64-bit integer), add one, and hold the low 32 bits to 2 through 30. Each rise SHALL be worked out in double and rounded to single before the sum.

#### Scenario: A wrapping span
- **WHEN** the span rounds to 4 294 967 040
- **THEN** the count is 2

### Requirement: Tree prompt caption
A prompt button's caption SHALL sit one unit past `(28 units − width) >> 1` from its button's left. A focused caption SHALL be drawn as the skin's glowing text: four copies a pixel left, right, below and above in the highlight group's first shade, and the text in the label group's eighth shade.

#### Scenario: Focused BEGIN
- **WHEN** the creation prompt on a 1000 by 700 window at unit 4 has BEGIN (50 pixels wide) focused
- **THEN** BEGIN's caption is at x 887, with four highlight copies of its five glyphs and one copy in the label ink

## MODIFIED Requirements

### Requirement: Tree domain wheel child
The tree SHALL carry the domain wheel at `(10, height − 290)`, 280 pixels square, with the opened domain as its choice, the focus share as its first share (48 with no record) and the bar's fade as its show. An icon click SHALL go through the wheel's confirm (known, different domain); a confirmed domain SHALL start the domain switch and play `ThaumShow02` at 0.25. A pick or drop SHALL start the wheel's 300 ms ease.

#### Scenario: Known and unknown domains
- **WHEN** a tree opened in Mind with a 700-pixel height is clicked at the Energy icon (150, 626) with no known domains, then at the Mind icon with all known, then at the Energy icon again
- **THEN** the first two clicks start no switch and the third chooses Energy, starts the 256 ms fade-out and plays `ThaumShow02`
