# Spec Delta

## ADDED Requirements

### Requirement: Node pictures
No columns SHALL draw no icon. Otherwise the cell is the power low byte, one less, across the sheet, and the square is centred on the node. A second byte of 1 SHALL use bias 1, and any other the other bias. A frame SHALL use the slot one less, four across and two down, at four fifths of the size. A wide picture SHALL drop the view fraction, and a narrow one use half. A glow SHALL be 32 spokes on a ring of 0.84 of the width plus the outer size, starting straight up.

#### Scenario: A cell, a frame and a glow
- **WHEN** an icon with no columns is drawn, power 6 is drawn at size 10 on a sheet of 4 by 2 centred at 100, 50, the same icon with a second byte of 1 is drawn, a view of 10.5 is spanned for a wide node and a narrow node, frame slot 5 is drawn at size 10 on the origin, and a glow of width 10 and outer 4 is drawn at the origin
- **THEN** the empty sheet draws nothing, the icon sits at 95, 45 in cell 0.25, 0 with the other bias and the marked icon uses bias 1, the spans are 10 and 5.25, the frame is an 8-unit square at -4, -4 in cell 0, 0, and the glow has 33 spokes with the first straight up on the inner ring
