# thaumaturgy Specification

## Purpose
One playable effect for each thaumaturgy domain, paid for with focus.

## Requirements

### Requirement: Four effects
Displacement SHALL place the body at the chosen point and spend 0.20 focus. Energy SHALL lower yellow health by the arc strength, not again until 150 ms have passed, and spend 0.10 focus. Force SHALL knock 60 along the horizontal aim and spend 0.12 focus. Mind SHALL lower each concern by 30, aggression by 10 and fear by 50, floored at zero, and spend 0.3 times 0.14 focus. A cast without the focus SHALL change nothing.

#### Scenario: Each domain
- **WHEN** each domain is cast with enough focus, and a cast is tried below its cost
- **THEN** the body moves, yellow health falls, the knock is 60, calm lowers the mind, and the short cast leaves focus and the target unchanged

### Requirement: Power prices
The power table SHALL record each listed id and its focus cost, including the four effects already cast. A sustained cost SHALL be that cost times the tick in seconds, and half of that when the sustain technique is known. Repeating the same instant power with the repeat technique SHALL halve the cost. Blast and bolt SHALL spend 1.2 times the cost, barrier twice, a mind power 0.3 times, vanish 80 times, and the long door once.

#### Scenario: Barrier tick and a repeated blast
- **WHEN** barrier's 0.07 is priced for a 500 ms tick, then again with the sustain technique, and blast's 0.10 is repeated with the repeat technique
- **THEN** the costs are 0.035, 0.0175 and 0.05, and calm spends 0.3 times its cost while the long door spends its cost once

### Requirement: Power tree
A power tree SHALL refuse a file whose first word is not the power magic. The node count SHALL be the word at byte 52. Each node SHALL carry a power id, two more words, an x, a y, and 16-byte links. A link count below 1 SHALL store no links. Every node after the first SHALL subtract the first node's x and y. A file that ends before the count SHALL be an error.

#### Scenario: Two nodes and a bad file
- **WHEN** a tree with a node at 10, 20 and a node at 15, 25 with one link is read, a link count below 1 is read, a file with a zero first word is read, and a file that ends after the magic is read
- **THEN** the first node stays at 10, 20, the second is at 5, 5 with that link, the low link count stores none, and the other two reads are errors

### Requirement: Tree text
A text chunk after the nodes SHALL store each string as a length word and the bytes that follow it, at that string's offset. A length below 1 SHALL be empty. A tree with no text chunk SHALL have no strings. A text chunk that ends after its first word SHALL be an error.

#### Scenario: Blast, an empty string, and a short chunk
- **WHEN** a tree whose text chunk holds Blast and a length of 0 is read, the same nodes with no text chunk are read, and a text chunk that ends after its first word is read
- **THEN** the first text is Blast, the second is empty, the bare tree has no texts, and the short chunk is an error

### Requirement: Tree lines
A node SHALL sit at ten times its place times the zoom, plus the pan, and round to the nearest even integer unless dragged. A factor SHALL scale that point about the origin. Both marks at zero SHALL draw nothing. Otherwise each end leaves its node along the sine and cosine of its turn times a full circle, by 0.514 or 0.295 minus 0.003 times the zoom, times the zoom rounded to an even 256. The smaller reach applies when the power high mark is set. A zero source mark SHALL dim the line.

#### Scenario: A snapped node and a dim line
- **WHEN** a node at 2, -1 is placed at zoom 1 with no pan and snapped, a node at 0.25 is placed the same way and again while dragged, a factor of one half scales 20, -10 about the origin, and a link from a large node at the origin to a small node 400 to the right is drawn at zoom 1 with both turns at zero, the source mark zero and the dest mark one, and again with both marks zero
- **THEN** the snapped node is at 20, -10, the quarter node snaps to 2, 0 and stays at 2.5 while dragged, the scaled point is at 10, -5, the line starts at the large reach above the origin and ends at the small reach above 400 in 19 points and is dim, and the unmarked link draws nothing

### Requirement: Node pictures
No columns SHALL draw no icon. Otherwise the cell is the power low byte, one less, across the sheet, and the square is centred on the node. A second byte of 1 SHALL use bias 1, and any other the other bias. A frame SHALL use the slot one less, four across and two down, at four fifths of the size. A wide picture SHALL drop the view fraction, and a narrow one use half. A glow SHALL be 32 spokes on a ring of 0.84 of the width plus the outer size, starting straight up.

#### Scenario: A cell, a frame and a glow
- **WHEN** an icon with no columns is drawn, power 6 is drawn at size 10 on a sheet of 4 by 2 centred at 100, 50, the same icon with a second byte of 1 is drawn, a view of 10.5 is spanned for a wide node and a narrow node, frame slot 5 is drawn at size 10 on the origin, and a glow of width 10 and outer 4 is drawn at the origin
- **THEN** the empty sheet draws nothing, the icon sits at 95, 45 in cell 0.25, 0 with the other bias and the marked icon uses bias 1, the spans are 10 and 5.25, the frame is an 8-unit square at -4, -4 in cell 0, 0, and the glow has 33 spokes with the first straight up on the inner ring
