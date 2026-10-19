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

### Requirement: Learned marks
Each node mark SHALL be the last known pair whose id, with bits 20 to 23 cleared, equals the power, or zero. The first node SHALL set bit 0x10000. A mark that already has bit 0x1000 SHALL keep bit 0x4000 and set bit 0x10000. Any other mark SHALL clear bit 0x4000, and SHALL set bit 0x10000 when the top nibble is not 7 and every link end, one-based, has bit 0x1000. A missing end SHALL not.

#### Scenario: An opened link and a held mark
- **WHEN** a lone node is marked, a known id with bits 20 to 23 set is followed by a later pair, a link to a closed node is marked, a link to an opened node is marked, a top nibble of 7 is linked to that opened node, and a mark of 0x5000 names an end that is not a node
- **THEN** the lone node is 0x10000, the later pair wins as 0x10002, the closed link stays 0, the opened link is 0x10000, the nibble of 7 stays 0, and the held mark is 0x15000

### Requirement: Thick joint
A thick joint SHALL place two points around the middle, one to each side of the chord from the first point to the third, at the given width. Width 2 through 0, 10 and 20 SHALL sit at 10, -2 and 10, 2 with a join of zero. A zero-length chord SHALL stay on the point.

#### Scenario: A straight joint and a still point
- **WHEN** a width of 2 is placed through 0, 10 and 20, and the same width is placed where all three points are 1, 2
- **THEN** the straight joint is at 10, -2 and 10, 2 with a join of zero, and the still point stays at 1, 2

### Requirement: Blast charge
A tick of N milliseconds SHALL set the rate to N thousandths, then five sixths of that. While charging, the amount SHALL rise by the rate, and passing 1 SHALL clear it, set the state to 1 and count as a release. An amount of exactly 1 SHALL keep charging. While fizzling, the amount SHALL fall by twice the rate, and passing below 0 SHALL become 0 with state -2. After release the amount SHALL rise by three times the rate.

#### Scenario: A full charge, a fizzle and a release
- **WHEN** the rate of 12 ms and of 1000 ms is read, 0.5 is charged by 0.5, 0.6 is charged by 0.5, 0.5 is fizzled by 0.1, 0.3 is fizzled by 0.2, and 0.2 is stepped after release by 0.1
- **THEN** the rates are 0.01 and five sixths, the full amount stays at 1 and charging, the larger charge releases at 0 with state 1, the mild fizzle is 0.3, the deep fizzle is 0 with state -2, and the released amount is 0.5

### Requirement: Blast chunks
A chunk SHALL grow by the tick times 0.7 of its speed plus 3, times 0.0006. Two chunks closer than twice the first radius SHALL move apart by half the overlap. A pair already that far apart, or on the same point, SHALL stay.

#### Scenario: A still chunk and a close pair
- **WHEN** a chunk of radius 0 and speed 0 grows for one tick, chunks at 0 and 1 with radius 1 are separated, a pair 2 apart is separated, and a pair on the same point is separated
- **THEN** the still chunk is 0.0018, the close pair is at -0.5 and 1.5, and the other two pairs stay

### Requirement: Barrier ripple
A barrier ripple SHALL lose 0.05 of strength each step, and the shown value SHALL move toward that strength by at most 0.04. Below 0.07 on both it SHALL be gone. While it remains, its place SHALL ease 0.05 of the way toward the hit. The dome extent SHALL be 30 plus 1.25 times the ripple count.

#### Scenario: A held ripple and an empty dome
- **WHEN** a ripple at shown 1 and strength 1 is stepped toward a hit 10 ahead, ripples at 0.06 and at 0.07 are stepped, and the extent of no ripples and of four ripples is read
- **THEN** the held ripple eases to 0.5, the two faint ripples are gone, the empty dome is 30 and four ripples make 35

### Requirement: Wave front
A wave's rate SHALL be the tick in seconds, then five thirds of that. While charging or fizzling, the radius SHALL grow by 0.0015. Once released, the reach SHALL grow by 0.6, the radius by 0.0006 of the speed, and the trail step SHALL be 0.07 of the speed plus 1. An amount of 0.99 with a 12 ms wave rate SHALL release. The blast rate for that same tick SHALL leave the amount at 1.

#### Scenario: A 12 ms wave and a still front
- **WHEN** 0.99 is charged by a 12 ms wave rate and by the blast rate, a radius grows while charging, and a released front of speed 0 and of speed 10 is stepped
- **THEN** the wave releases, the blast amount stays at 1, the charging radius is 0.0015, the still front has reach 0.6 and trail 1, and speed 10 grows the radius by 0.006 with trail 1.7

### Requirement: Shell ripple
A shell ripple SHALL lose 0.04 times its scale from the strength. The shown value SHALL then move toward that strength by at most 0.06 times the scale upward and 0.04 times the scale downward. Below 0.05 on both it SHALL be gone.

#### Scenario: A held shell and a faint one
- **WHEN** shown 1 and strength 1 are stepped at scale 1 and at scale 2, shown 0.09 and strength 0.09 are stepped at scale 1, and shown 0.05 and strength 0.05 are stepped at scale 1
- **THEN** scale 1 is 0.96, scale 2 is 0.92, the 0.09 pair stays at 0.05, and the 0.05 pair is gone

### Requirement: Torrent mix
While the phase is below 1, a torrent level SHALL be (2 minus the charge) times the charge times 0.4. Otherwise the level SHALL fall by 0.01, and below 0.02 it SHALL stop. Phase 1 SHALL ease the two channels toward 0.5 and 0.6 by 0.06. Phase 2 SHALL multiply both channels by 0.96.

#### Scenario: A full charge, an ease, and a stop
- **WHEN** a charge of 1 and of one half is stepped below phase 1, phase 1 eases channels that start at 0 while the level is 0.4, and phase 2 dims channels that start at 1 while the level is 0.4 and again while it is 0.03
- **THEN** the levels are 0.4 and 0.3, the eased channels are 0.03 and 0.036, the dimmed channels are 0.96, and the 0.03 level stops

### Requirement: Torrent streams
A torrent SHALL round eight tenths of its span to the nearest even integer. A stream SHALL sit at 16 times its direction, then its height times 1.5 minus 30. The mesh SHALL ask for 13 groups for each stream plus 8, 12 bytes a group, and 8 indices a group.

#### Scenario: Two streams and a dropped birth
- **WHEN** spans of 1, 2 and 3 are counted, a stream along height 1 is placed, and a mesh of 2 streams is sized
- **THEN** the counts are 1, 2 and 2, the stream is at height -6, and 2 streams ask for 10 records, 130 groups, 1560 bytes and 1040 indices

### Requirement: Torrent push
A torrent reach SHALL be the radius times the scale, plus the margin, plus 10. A hit SHALL raise the first channel by 0.005 of the age over the sample count, and the second by 0.01 of that, while the second is under 1.8. A point inside the reach SHALL move by 0.01 of one minus its distance squared over the reach squared. A point on the reach SHALL stay. A point past it SHALL be missed.

#### Scenario: A reach, a hit, and a push
- **WHEN** a radius of 2 at scale one half and margin 1 is reached, age 1 of 2 samples raises channels at 0, the second channel at 1.8 still rises and the next float does not, and a point at the origin is pushed along a unit step inside a reach of 10, on that reach, and past it
- **THEN** the reach is 12, the channels are 0.0025 and 0.005, the inside point moves to 0.01, the point on the reach stays, and the point past it is missed

### Requirement: Shell vertex
A shell vertex SHALL sit at its position plus 2 times the normalized sum of its bone. A zero sum SHALL add nothing. A hit of 0.1 SHALL store about 3.333. A hit of 0.3 SHALL store 6. An eighth hit SHALL be refused.

#### Scenario: A raised vertex and a capped hit
- **WHEN** a position at 1, 2, 3 is offset by a sum of 0, 3, 0 and by a zero sum, a hit of 0.1 is stored, a hit of 0.3 is stored while seven are held, and an eighth hit is offered
- **THEN** the offset vertex is raised on its height, the zero sum stays at 1, 2, 3, the small hit is about 3.333, the 0.3 hit is 6, and the eighth hit is refused

### Requirement: Shell indices
A shell index list SHALL copy each section that is not hidden, from its first triangle through its last, inclusive. Each triangle SHALL keep its three indices in order, after the remap. A hidden section SHALL be skipped. A range that ends before it starts SHALL copy nothing. A short index list or a missing remap SHALL be an error.

#### Scenario: A kept winding and a hidden section
- **WHEN** two visible ranges and a hidden range are copied, a reversed triangle is copied, a range that ends early is copied, a short list is copied, and a missing remap is copied
- **THEN** the kept list is 14, 15, 16, 11, 12, 10, 17, 18, 19, the reversed triangle stays reversed, the early range is empty, the short list is an error, and the missing remap is an error

### Requirement: Skinned shell sections
A skinned shell SHALL copy a section only when its flags include bit 2. Each copied triangle SHALL keep its three indices in order, after the remap. A section without that bit SHALL be skipped. A hidden section that also has bit 2 SHALL still be copied. A range that ends before it starts SHALL copy nothing. A short index list or a missing remap SHALL be an error.

#### Scenario: A marked section and a plain one
- **WHEN** a section with bit 2, a section without it, and a hidden section that also has bit 2 are copied, a reversed triangle with bit 2 is copied, a range that ends early is copied, a short list is copied, and a missing remap is copied
- **THEN** the kept list is 14, 15, 16, 11, 12, 10, the plain section is absent, the reversed triangle stays reversed, the early range is empty, the short list is an error, and the missing remap is an error

### Requirement: Compacted shell vertices
A shell mark list SHALL start at 255. A skinned section SHALL write its index over each triangle from the first through the last. Any other section SHALL write 255. A vertex SHALL take the next compacted id when a link in its half-open range names a mark other than 255, and stop at the first such mark. A vertex with none SHALL be absent. An index SHALL become that id. A short list or an absent id SHALL be an error.

#### Scenario: Two kept vertices and a gap
- **WHEN** marks are built for a skinned range, a plain range, and a later skinned range, three vertices are numbered, the kept indices are applied, an absent index is applied, and a short mark list is built
- **THEN** the marks are 0, 255, 2, the vertices are 0, absent, and 1, the indices become 1, 0, 1, the absent index is an error, and the short mark list is an error

### Requirement: Shell section slots
A kept shell vertex SHALL store its section and the corner index minus that section's base. The first link in its half-open range whose mark is not 255 supplies the section and the corner. A later link SHALL be ignored. A vertex with none SHALL be absent. A short list or a missing base SHALL be an error. A base of -1 subtracted from 1 SHALL store 2.

#### Scenario: Two slots and a gap
- **WHEN** three vertices are slotted, a later link is ignored, a base of -1 is subtracted from 1, a short corner list is slotted, and a missing base is slotted
- **THEN** the slots are section 0 local 1, absent, and section 2 local 2, the later link is ignored, the local index is 2, the short corner list is an error, and the missing base is an error
