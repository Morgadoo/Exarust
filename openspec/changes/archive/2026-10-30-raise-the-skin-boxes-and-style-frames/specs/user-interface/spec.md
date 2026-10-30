## MODIFIED Requirements

### Requirement: Skin selection band
A selection band SHALL draw an outline whose middle is half the width less one in, then glow fans reaching into the row from its top and bottom edges, wider when selected. Its outline ink SHALL be the highlight group's fourth shade and its glow ink the seventh, or the fifth and eighth when selected.

#### Scenario: The lower glow
- **WHEN** a band spans 10, 20 to 110, 100 at unit 4, unselected
- **THEN** the outline's middle is at x 59.5
- **AND** the lower fan reaches from y 99 up to 94 at the middle and 95 at the sides

#### Scenario: A selected band
- **WHEN** the same band is selected
- **THEN** its outline starts in the highlight group's fifth shade at alpha 25 and its glow in the eighth at alpha 200
- **AND** the upper fan reaches from y 21 to 29 at the middle

### Requirement: Skin sliders and scroll bars
A slider's track SHALL fade the hot group's fifth and first shades at 0.6 alpha, its thumb SHALL be the raised box and its filled bar SHALL be one unit wide; a scroll bar SHALL be drawn a pixel lower and two shorter, drawing its style frames on that rectangle, the accent gradient for the trough and the raised box for the thumb, its striped arm at style 0x1000_0000.

#### Scenario: A scroll bar
- **WHEN** a scroll bar spans 0, 0 to 100, 24 at unit 4
- **THEN** its trough is 1, 2 to 99, 22 and its thumb 83, 3 to 98, 21

#### Scenario: A styled scroll bar
- **WHEN** that scroll bar has style 0x10
- **THEN** its first primitive is a double frame starting at 99, 1.5 in the hot-form group's first shade

#### Scenario: A filled slider
- **WHEN** a filled slider of width 100, inset 4 and unit 6 is at one half
- **THEN** the bar's shift is 43

### Requirement: Form background scrollbars
The default background's vertical scrollbar SHALL draw its frame, shadows, arrows −9 and −10 as raised boxes pressed while the button is down on the stored pressed code, and one gradient bar for the thumb with the texture shifted by its place; the horizontal one SHALL draw arrows −13 and −14 the same way and a raised-box thumb with the texture shifted.

#### Scenario: The vertical scrollbar
- **WHEN** the thumb is at 30 with length 20 on a 100-high form with client top 24
- **THEN** the arrow frames are 180, 24 to 195, 39 and 180, 80 to 195, 95 and the thumb 180, 30 to 195, 50
- **AND** one shadow is 180, 50 of size 15 by 11

#### Scenario: The held arrow
- **WHEN** the mouse is over arrow −9 and the button is held on arrow −10
- **THEN** arrow −9 draws the idle box and arrow −10 the pressed box

## ADDED Requirements

### Requirement: Skin panel and shell style frames
The panel and the control shell SHALL draw the style frames their style word selects on their own rect before their box; the control box, panel and shell SHALL draw the raised box, pressed by state bit 0x40 and hot by bit 0x10.

#### Scenario: A styled panel
- **WHEN** a panel spanning 0, 0 to 100, 40 with inset 4 has style 0x10
- **THEN** a double frame starting at 99, 0.5 comes first, then the raised box from 94, 6

### Requirement: Skin frame hit-test order
The frame hit test SHALL probe the close, large and small marks at −4, −5 and −6 for style bits 2, 4 and 8 in that order, the vertical arrows −9 then −10 and the horizontal arrows −13 then −14 before their fallbacks.

#### Scenario: Both horizontal arrows under the point
- **WHEN** a point lies in both horizontal arrows
- **THEN** the hit test answers −13
