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

### Requirement: Shell index room
A skinned shell index room SHALL add one for each triangle of a section with bit 2, and nothing without that bit. A hidden section with bit 2 SHALL still count. The room SHALL ask for 12 bytes plus 8, plus 4 bytes a triangle. The kind SHALL be 3 and the base 0. The end SHALL be one less than the triangle count, and the vertex end one less than the kept count. A range one early SHALL add nothing. A range two early SHALL subtract one.

#### Scenario: Three triangles and a short range
- **WHEN** a skinned range of two triangles, a plain range, a hidden skinned triangle, and a range that ends one early are counted for two kept vertices, and a range that ends two early is counted for none
- **THEN** the room has 3 triangles, 32 bytes, width 4, one channel, kind 3, base 0, end 2 and vertex end 1, and the short range has -1 triangles, 16 bytes and vertex end -1

### Requirement: Shell size fade
A shell SHALL store a size of 800. Opening SHALL divide the elapsed time by that size times 1.5. Above 1 it SHALL become 1 and leave, or enter the hold state when held. The hold state SHALL move on and stamp the size times the remainder, rounded to the nearest even integer. Closing SHALL store 1 minus the elapsed time divided by the size. A value not above 0 SHALL become 0 and finish. A held close SHALL store 3 times that value, then show 1. Any other state SHALL stay.

#### Scenario: Open, hold, and close
- **WHEN** the stored size opens for 0, for 1200 and for 1201, a held open of 1201 is stepped, a hold at 0 and a half are stamped, a close of 400 and of 800 are stepped, a held close of 400 is stepped, and a resting fade is stepped
- **THEN** the size is 800, the opens stay at 0, stay at 1, and finish at 1, the held open enters the hold state, the stamps are 200 and 10, the closes are one half and finished at 0, the held close shows 1 with a gain of 1.5, and the resting fade stays

### Requirement: Shell light
A shell light SHALL follow the shown value. While opening, a shown value whose double is above 0.666 SHALL become one half, and any other SHALL become 0.75 minus the absolute gap from 0.75 to the shown value times 1.5. Any other state SHALL take half the shown value. The light SHALL then be one tenth of that. While closing, the gain SHALL become the shown value times 2.8. Any other state SHALL keep its gain.

#### Scenario: A bend, a flat half, and a close
- **WHEN** an opening light is taken at 0, at one half, at the float just under 0.666 and at the next float, a resting light at 1 is taken, and a closing light at one half is taken
- **THEN** the opening levels are 0, a tenth of 0.75, the bend, and a twentieth, the resting light is a twentieth and keeps its gain, and the closing light stores a gain of 1.4

### Requirement: Shell light pair
A shell's second light SHALL store two values. A resting state SHALL store 0.1 and 0.75. While opening, a shown value under 0.25 SHALL store one tenth of its times-4 product with 2 minus that product, and 1 minus that product times 0.25. At 0.25 or more it SHALL store 0.1 and 0.75. While closing, the first value SHALL be one fifth of 2 minus the absolute gap from 1.5 to the shown value times 3.5, and the second SHALL be the shown value times 0.75. A hold SHALL keep the pair it was given.

#### Scenario: Rest, open, hold, and close
- **WHEN** a resting pair is taken, an opening pair is taken at 0, at the float just under 0.25 and at 0.25, a hold keeps a pair, and a close is taken at 0 and at 1
- **THEN** the resting pair is 0.1 and 0.75, the open at 0 is 0 and 1, the float just under 0.25 is just under 0.1 and 0.75, the open at 0.25 is 0.1 and 0.75, the hold keeps its pair, the close at 0 is 0.1 and 0, and the close at 1 is 0 and 0.75

### Requirement: Shell index upload
A shell index upload SHALL ask for a header of 20 bytes plus 8 a channel, and a payload of the triangle count times the width. The target SHALL be 0x8893, the usage 0x88E4, and the access 0x88B9. A finish SHALL unmap only when the buffer name is above 0.

#### Scenario: Three triangles and a name
- **WHEN** an upload is asked for one channel, the triangle count of two skinned spans, and a width of 4, and a finish is asked for a name of 0 and a name of 1
- **THEN** the header is 28 bytes, the payload is 12 bytes, the target is 0x8893, the usage is 0x88E4, the access is 0x88B9, the name of 0 stays, and the name of 1 unmaps

### Requirement: Shell index submission
A play draw of a skinned shell SHALL count its room and, when the upload flag is set, SHALL submit that upload. The mesh SHALL hold one index per triangle of the payload. A name above 0 SHALL unmap. A clear flag SHALL keep the counted room and SHALL not unmap.

#### Scenario: Three triangles, uploaded and kept
- **WHEN** two skinned spans are submitted with the flag set and a name of 1, and the same spans are submitted with the flag clear and a name of 1
- **THEN** the upload has a header of 28, a payload of 12 and 3 indices, and it unmaps, and the clear draw keeps a room of 32 bytes and does not unmap

### Requirement: Tree close
Close SHALL clear the global qword, run the inner reset once, clear the global byte and run the base once.

#### Scenario: The close
- **WHEN** the tree closes
- **THEN** the global qword and byte are cleared and one reset and one base call run

### Requirement: Tree show
Show SHALL call the focus once with `(word & 7) | 0x800` when the focus count is positive, refresh once when the pane word carries 0x20, OR bit 2 into 0x21d, write the clock plus the offset to 0x224 and store 0x1bc globally.

#### Scenario: The show
- **WHEN** the focus count is 1, the global word is 0xF, the pane word carries 0x20, the clock is 1000, the offset 250 and 0x1bc is 7
- **THEN** the focus word is 0x807 and one focus and one refresh call run
- **AND** 0x21d gains bit 2, 0x224 is 1250 and the stored value is 7
- **WHEN** the focus count is 0
- **THEN** no focus call runs

### Requirement: Tree charge
The charge SHALL add `tick / 120` in double precision, clamp to −9 through 5 and set the gain to `(5 + charge) × 0.2` at or above 0, else `(4 − √−charge) × 0.25`. A clear started byte SHALL be set and the clock stamped only on the first call.

#### Scenario: The rising branch
- **WHEN** the charge is 0 and the tick is 120
- **THEN** the charge is 1 and the gain is 1.2
- **WHEN** the charge is 0 and the tick is −120
- **THEN** the charge is −1 and the gain is 0.75

#### Scenario: The clamps
- **WHEN** the charge is −9 and the tick is −120
- **THEN** the charge stays −9 and the gain is 0.25
- **WHEN** the charge is 5 and the tick is 120
- **THEN** the charge stays 5 and the gain is 2

#### Scenario: The start stamp
- **WHEN** the started byte is clear
- **THEN** it is set and the clock is stored
- **WHEN** it is already set
- **THEN** the clock is unchanged

### Requirement: Tree selection roots
With neither word positive the selection SHALL return the base result. In mode 3 a new or current word of 0x1000000 or 0x2000000 SHALL store the new word and return 3.

#### Scenario: No selection
- **WHEN** the new and current words are 0
- **THEN** the base result returns and nothing else changes

#### Scenario: The special roots
- **WHEN** the mode is 3 and the new word is 0x1000000
- **THEN** it is stored and the result is 3

### Requirement: Tree selection candidate
The candidate SHALL be 0 for bit 0x400000, the low 16 bits for a word in 1..0xfffe with a valid table entry, else 0. The new word SHALL be stored. An equal last focus SHALL return 0; an equal previous selection SHALL return 3; a fresh candidate SHALL return 3, store the focus, set the previous when positive and stamp the clock.

#### Scenario: The selection paths
- **WHEN** the new word is 5, the table is valid and the last focus is 5
- **THEN** the result is 0 and the focus is unchanged
- **WHEN** the last focus is 0 and the previous selection is 5
- **THEN** the result is 3 and the focus stays 0
- **WHEN** the last focus and the previous selection are 0
- **THEN** the result is 3, the focus and previous become 5 and the clock is stamped
- **WHEN** the new word carries 0x400000
- **THEN** the candidate is 0
