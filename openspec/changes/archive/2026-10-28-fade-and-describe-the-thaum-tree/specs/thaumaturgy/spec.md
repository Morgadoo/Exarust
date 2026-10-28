## ADDED Requirements

### Requirement: Tree window fades
Showing the tree SHALL start a 1000 ms fade: with `r = 1 − (1 − left·0.001)` (the inner difference in double, rounded to single), the window SHALL be `2·(1 − r²)` held to one and the bar child `1 − r²`. Hiding SHALL cut the window by `left/256` and stop the draw at its stamp. A domain switch SHALL cut the content out over 256 ms, switch at the stamp and fade the new tree in over 1000 ms.

#### Scenario: Opening and closing
- **WHEN** the tree is shown at 1000 and drawn at 1001, 1500 and 2000, then hidden at 3000 and drawn at 3128 and 3256
- **THEN** the window is 0.003997922, then 1 with the bar child at 0.75, then 1 with the fade over
- **AND** the window is 0.5 at 3128 and the screen has faded out at 3256

### Requirement: Bar open and close animation
The bar's opening SHALL run 900 ms: height and size the smooth step `(3 − 2x)·x²` of `elapsed·0.0025` for the first 400 ms, the pitch the smooth step of the tree's ease of `elapsed/900`. Its closing SHALL run 900 ms from its own stamp: marked slots `1 − f⁴`, the rest `1 − 1.5f` held at zero, `f` the smooth step of `elapsed/600`, none after 600 ms. Both SHALL apply while both run.

#### Scenario: Closing during the opening
- **WHEN** the bar opens at 3000 and closes at 3200, and is stepped at 3500 and 3900
- **THEN** at 3500 both animations run, 500 and 300 ms in, and at 3900 only the closing runs, 700 ms in

### Requirement: Tree tooltip
The shown choice SHALL draw its name in a caption under the node at once, fading to 160/255 over 200 ms. From an age of 601 ms its description SHALL be wrapped at 59 units in a 60-unit box at `ROUND(x − 31 units) + unit` and `8 units + ROUND(base)` (above the node when it would leave the window), fading to 160/255 over 300 ms. Neither SHALL be drawn in label mode.

#### Scenario: Caption first, box later
- **WHEN** a node at (100, 100) with a fixed zoom of 75 has been the shown choice for 100 ms and then 601 ms, at a unit of 4
- **THEN** the caption's alpha is 80/255 and no box is drawn
- **AND** then the box is at (−20, 172), 240 by 8, with alpha 1/255

### Requirement: Tree label mode
In label mode every node with a name SHALL draw a 128-alpha box of its name's width less two units at `ROUND(x − ⌊box/2⌋)` and `ROUND(base + unit)`, and the name a unit right of `ROUND(x − ⌊(width + 2 units)/2⌋)` at `ROUND(base + 4 units)`, where `base` is the node's y plus 0.54 of the fixed zoom (0.32 framed).

#### Scenario: Two labels
- **WHEN** nodes at x 100 and 200 (y 100, fixed zoom 75, unit 4) carry names 40 and 24 pixels wide
- **THEN** the boxes start at x 84 and 192, 32 and 16 wide, at y 144, and the first name at x 80
