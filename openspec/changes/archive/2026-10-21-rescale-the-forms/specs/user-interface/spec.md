## ADDED Requirements

### Requirement: Form rescale
When the GUI unit changes, every form SHALL scale from the old unit to the new, lengths divided by the old unit, then times the new, and a border of 4 pixels or more set to its design value times a quarter of the unit. On each axis a stretched form SHALL keep both edges' gaps; others SHALL scale their size (or keep their frame around a scaled client) and keep their right or bottom gap, their centring, or their scaled place.

#### Scenario: Unit 4 to 8
- **WHEN** a form at 40, 20 of 104 × 64 with a 2-pixel border rescales from unit 4 to 8 in an 800 × 600 parent
- **THEN** it is at 80, 40 and 204 × 124, its client 200 × 120
- **WHEN** it is anchored right and bottom 16 pixels in
- **THEN** its right and bottom gaps become 32

### Requirement: Form rescale order
A title-only form SHALL be as tall as its title and the skin's base. Child forms SHALL rescale inside the parent's old and new client, and the parent's control records after them.

#### Scenario: A title bar
- **WHEN** a title-only form with a 12-pixel title rescales with a 46-pixel base
- **THEN** it is 58 tall

### Requirement: Control record rescale
Control records SHALL rescale in 16-bit fields: stretched records keep both gaps; others scale their width (an odd width under 16 units grows by one) and keep centring, their right gap or their scaled place.

#### Scenario: A right-anchored record
- **WHEN** a 40-wide record 10 pixels from the right of a 100-pixel client rescales from unit 4 to 8 into 200 pixels
- **THEN** it is 80 wide, 16 from the right

### Requirement: Control record vertical rescale
A bottom record of a form that keeps them SHALL keep its scaled gap below (its odd height made even); other records SHALL move down by half their height less four old units (one unit when taller than eight), scale, and move up by half their new height less four new units (one unit when taller than eight).

#### Scenario: A button
- **WHEN** a 41 × 20 record at 10, 10 rescales from unit 4 to 8
- **THEN** it is at 16, 20 and 80 × 40

### Requirement: Glyph sheet
The large icon atlas SHALL take the common glyph sheet's nineteen 64-pixel squares in the original's order, the seventeenth (the item views' marker) being the square at 0, 256.

#### Scenario: The marker
- **WHEN** the sheet's icons are listed
- **THEN** the seventeenth is the square at (0, 256)

### Requirement: Message box layout
A message box SHALL be the text's width plus 64 wide (at least 320) plus twice the skin's frame, 104 tall, centred down the screen (or a quarter down) with its top turned into design pixels (divided by the unit, truncated, times 4); its label SHALL be at 12, 4 and 24 narrower; its Ok and Cancel buttons SHALL be 80 wide, 96 apart, centred, at grid row 13.

#### Scenario: Ok and Cancel
- **WHEN** a 100-pixel text gets both buttons on a 768-pixel screen at unit 4 with a 16-pixel frame
- **THEN** the box is 352 × 104 at y 280 with buttons at 72 and 168
