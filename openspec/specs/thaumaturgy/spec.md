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
A power tree SHALL refuse a file whose first word is not the power magic. The node count SHALL be the word at byte 52. Each node SHALL carry a power id, the index of its name text, the index of its description text, an x, a y, and 16-byte links. A link count below 1 SHALL store no links. Every node after the first SHALL subtract the first node's x and y. A file that ends before the count SHALL be an error.

#### Scenario: Two nodes and a bad file
- **WHEN** a tree with a node at 10, 20 and a node at 15, 25 with one link is read, a link count below 1 is read, a file with a zero first word is read, and a file that ends after the magic is read
- **THEN** the first node stays at 10, 20, the second is at 5, 5 with that link, the low link count stores none, and the other two reads are errors

#### Scenario: Text indices
- **WHEN** a node whose two words after the power are 3 and 4 is read
- **THEN** its name text index is 3 and its description text index is 4

### Requirement: Tree text
A text chunk after the nodes SHALL give a count that is its last string's index, so it holds count + 1 offsets and strings. Each string SHALL be a length word and the bytes that follow it, at that string's offset in the text. A length below 1 SHALL be empty. A string whose offset or bytes lie outside the text SHALL be empty. A tree with no text chunk SHALL have no strings. A text chunk that ends before its table or text SHALL be an error.

#### Scenario: Blast, an empty string, and a short chunk
- **WHEN** a tree whose text chunk has the count 1, the offsets 0 and 9, and holds Blast and a length of 0 is read, the same nodes with no text chunk are read, and a text chunk that ends after its first word is read
- **THEN** the first text is Blast, the second is empty, there are two texts, the bare tree has no texts, and the short chunk is an error

#### Scenario: An offset past the text
- **WHEN** a text chunk has an offset that lies past its text
- **THEN** that string is empty and the others are read

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
A barrier ripple SHALL lose 0.05 of strength each step, the subtraction taken in double precision, and the shown value SHALL move toward that strength by at most 0.04. Below 0.07 on both it SHALL be gone. While it remains, its place SHALL ease 0.05 of the way toward the hit. The dome extent SHALL be 30 plus 1.25 times the ripple count.

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

### Requirement: Tree submit
The submit SHALL append one entry only in state 0x21 when the count is positive: the value and the second value over 5.0, then the count. It SHALL always clear the state and index and run the base once.

#### Scenario: The submitted entry
- **WHEN** the state is 0x21, the count is 3, the value is 10 and the second is 7.5
- **THEN** the entry is 2.0, 1.5 and 3
- **AND** the state and index clear and one base call runs
- **WHEN** the state is not 0x21 or the count is 0
- **THEN** no entry is appended and the base still runs

### Requirement: Tree keys
A non-zero state SHALL go to the base and return its result. A set hide flag with its deadline less 0x3b6 above the clock SHALL return without work. With flag 0x2 clear the next key SHALL step +1 and the previous key −1. Any other key SHALL dim unless the dim flag is set.

#### Scenario: The base and hide window
- **WHEN** the state is 3
- **THEN** the base runs and its result returns
- **WHEN** the hide flag is set, the deadline is 1000 and the clock is 10
- **THEN** the handler returns without work

#### Scenario: The steps and dim
- **WHEN** the flags lack 0x2 and the key is the next code
- **THEN** the step is +1
- **WHEN** the key is the previous code
- **THEN** the step is −1
- **WHEN** the key is any other and the dim flag is clear
- **THEN** the tree dims
- **WHEN** the dim flag is set
- **THEN** nothing runs

### Requirement: Tree hover
An event other than kind 0xae56ca01 SHALL be refused. A positive node index SHALL copy the previous entry when set, merge the event's first word into the entry's 0x700000 bits, store the event's 0x20 word, play the hover sound when ready and return true. A non-positive index SHALL copy the defaults when a previous entry is set and return false.

#### Scenario: The hover hit
- **WHEN** the kind is 0xae56ca01, the index is 3, the previous index is 2, the event's first word is 0x11, its 0x20 word is 7 and the entry's old flags are 0x123456
- **THEN** the previous entry is copied, the new flags are 0x100011 and the stored word is 7
- **AND** the hover sound plays and the result is true

#### Scenario: The miss and the guard
- **WHEN** the index is 0 and the previous index is 2
- **THEN** the defaults are copied and the result is false
- **WHEN** the kind differs
- **THEN** nothing happens

### Requirement: Tree decay fields
Mask 0x10 SHALL subtract `(2 + base) × first × 4` and `(2 + base) × second × 4` from fields 0x17c and 0x180, stamp the first start and return 3. Mask 0x20 SHALL call the 0x180 virtual with the runtime value × −10 rounded and return 3.

#### Scenario: The two terms
- **WHEN** the mask is 0x10, the base is 3, the first float is 1, the second is 2 and the fields are 100 and 200
- **THEN** the fields become 80 and 160, the start is stamped and the result is 3
- **WHEN** the mask is 0x20 and the runtime value is 2.5
- **THEN** the virtual receives −25 and the result is 3

### Requirement: Tree decay mode
In mode 3 mask 0x200 SHALL call 0x250 when present and mask 0x100 SHALL call 0x248 when present. Otherwise a mask carrying 0x300 SHALL dim once unless the dim bit is set.

#### Scenario: The mode callbacks
- **WHEN** the mode is 3 and the mask is 0x200 with a 0x250 callback
- **THEN** it runs
- **WHEN** the mode is 0, the mask carries 0x100 and the dim bit is clear
- **THEN** the tree dims once
- **WHEN** the dim bit is set
- **THEN** nothing runs

### Requirement: Spirit form charge
The spirit form SHALL charge at one value per 30 s from state 3, clamp at 1 and become held in state 1; held SHALL keep the value at 1.

#### Scenario: Charging to full
- **WHEN** a form at 0 charges for 15 s
- **THEN** the value is 0.5 and the state is still charging
- **WHEN** it reaches 1
- **THEN** the state is held and the value stays at 1

### Requirement: Spirit form fade
A fade SHALL lower the value by one per 30 s and end at 0 in state 0 once it falls below 0.002.

#### Scenario: Fading out
- **WHEN** a full form fades for 15 s
- **THEN** the value is 0.5 and the strength 500
- **WHEN** it fades for 30 s
- **THEN** the form is inactive with value 0

### Requirement: Spirit form publish
While fading the strength SHALL be 1000·value. Otherwise the smooth value SHALL be 1 − (1 − value)², the strength (1 − smooth)·rate + 1000·smooth, and the vector scale 1 − value.

#### Scenario: Half charged
- **WHEN** a form at 0.5 charges with rate 100
- **THEN** the smooth value is 0.75, the strength 775 and the vector scale 0.5

### Requirement: Mind-sight fade
The overlay SHALL be off at state 0; shown, it SHALL hold alpha 1 and start fading out once its end has passed; both fades SHALL move alpha by the milliseconds since the tracker's last stamp over 2 s (the span signed, worked in double precision); the fade-out SHALL end below 0.001, dropping every tracked mind, and the fade-in SHALL hold at 1. A start SHALL show it for the duration (to the end of time for 0), extend a showing one, and restart the alpha unless it is fading.

#### Scenario: Shown, fading and finishing
- **WHEN** the overlay starts for 4000 ms and is drawn 1000 ms after the stamp
- **THEN** alpha is 0.5
- **WHEN** it is drawn past its end
- **THEN** it switches to fading out
- **WHEN** it fades below 0.001
- **THEN** it is off and the tracked minds are dropped

### Requirement: Mind-sight pick
The pick SHALL take a character on screen, not the player, not hidden, not collapsed unless the dead powers ask, seen by the player's view unless Sense Minds runs, within `height·zoom·0.5·40/depth` pixels of the screen centre with an unordered compare passing; the nearest share below 1 SHALL win.

#### Scenario: Inside and outside
- **WHEN** two characters lie 5 and 10 px from the centre with a 12 px radius
- **THEN** the nearer one is picked
- **WHEN** a character lies 20 px out
- **THEN** nothing is picked

### Requirement: Mind node weights
The mood SHALL give anger `a = 0.2·anger` and fear `f = 0.2·((dread + fear) − courage)`, each product in double precision; a collapsed body or `f > 1` SHALL give 1 for all three values.

#### Scenario: Marked and ordinary
- **WHEN** the body is collapsed
- **THEN** the three values are 1
- **WHEN** anger is 2 and dread and fear are 1
- **THEN** the larger value is 0.4 and the share one half

### Requirement: Mind threshold and blend
Otherwise `f` SHALL be floored at 0 and `a` clamped to 0..1, the second value their larger and the third `f/(f + a)` when the larger is above 0; the first SHALL be the larger plus 0.2, raised to 0.6 of arousal and set to 0.015 of stress when below 0.02 of it, capped at 1. An entry SHALL keep `exp(−ln 2·dt/1000)` of its old values, a half past a second.

#### Scenario: Threshold and a half
- **WHEN** the larger value is 0.4 and stress is 60
- **THEN** the first value is 0.9
- **WHEN** the step is a second
- **THEN** a half of the old values is kept

### Requirement: Mind-sight quad
The sphere SHALL sit 15 cm above the head node's height offset and the record's forward offset along the node's axes; its texture axes SHALL be the node's third and first axes made square to the view direction, blended by `1 − |dir·axis₀|` and 0.025 long, and expressed with the eye direction in the node's frame; the node's rows SHALL be scaled by the record's three scales.

#### Scenario: A level camera
- **WHEN** the camera looks along the node's third axis
- **THEN** the texture axes are 0.025 long and perpendicular and the eye direction is the negated third axis

### Requirement: Sight entry colours
A living entry SHALL take colour0 (0.1, 0.4, 0.6), colour1 (0.8, 0.6, 0) with alpha `(1 − first)·0.6 + 0.1` and colour2 from (0.4, 0, 0) toward (0.31, 0, 0.78) by its third value with alpha `(1 − second)·0.6 + 0.1`, in double precision; the overlay SHALL give colour0 its alpha less a positive entry fade and the glow `1 − fade`.

#### Scenario: Zero and full values
- **WHEN** an entry's third value is one half
- **THEN** colour2 is (0.355, 0, 0.39)
- **WHEN** its first value is 1
- **THEN** colour1's alpha is 0.1

### Requirement: Pale entry
An undead entry SHALL use (0.3, 0.3, 0.3) for colour0, (0.3, 0.3, 0.3, 1) for colour1 and (0.5, 0.5, 0.6, 0.5) for colour2.

#### Scenario: Pale palette
- **WHEN** an undead entry is coloured with alpha 0.5
- **THEN** colour0 is (0.3, 0.3, 0.3, 0.5)

### Requirement: Sight tick throttle
A sight tick SHALL be due at least four milliseconds after the last one.

#### Scenario: Three and four milliseconds
- **WHEN** three milliseconds have passed
- **THEN** the tick is not due
- **WHEN** four milliseconds have passed
- **THEN** the tick is due

### Requirement: Sight dwell
A collapsed body SHALL fade over 5000 ms and a body whose out word is 2 or more over 3000 ms, from when it was first seen so; the fade SHALL be the elapsed time over the window in double precision, and the entry SHALL be dropped once the clock passes seen plus window.

#### Scenario: Window and fade
- **WHEN** 2500 ms of a collapsed body's window have passed
- **THEN** its fade is 0.5
- **WHEN** the clock passes seen plus 5000 ms
- **THEN** its entry is dropped

### Requirement: Tree text chunk
The tree text chunk SHALL read a 16-byte header whose last word is the count, count + 1 little-endian u32 offsets, a u32 blob size, and the blob, each string a u32 length followed by its bytes. A short header, an out-of-range offset, a blob past the input, or invalid UTF-8 SHALL return nothing.

#### Scenario: Three strings
- **WHEN** the count is 2, the offsets are 0, 6 and 13, and the blob holds "ab", "cde" and the empty string
- **THEN** the parser returns those three strings in order
- **WHEN** the header is short or the blob size overruns
- **THEN** the parser returns nothing

### Requirement: Fresh tree text
A fresh tree text SHALL carry language 0x6e65 (en), flags 0x4e4, kind 1, marker 0 and one empty string. The parser SHALL read the language, flags, kind and marker from the first 12 header bytes before the count.

#### Scenario: A new chunk
- **WHEN** a fresh tree text is created
- **THEN** its language is 0x6e65, flags 0x4e4, kind 1, marker 0 and its strings are one empty string
- **WHEN** a chunk with those header words is parsed
- **THEN** the parser returns the same four fields

### Requirement: Kinesis scale
The kinesis scale SHALL be (1/speed + 0.015)·0.5.

#### Scenario: Two speeds
- **WHEN** the speed is 1
- **THEN** the scale is 0.5075
- **WHEN** the speed is 2
- **THEN** the scale is 0.2575

### Requirement: Kinesis activation
The effect SHALL move to removal when the event carries bit 1 or 3. Removal SHALL release a held target and return 2. The active state SHALL hook the owner when it accepts, raise the target's flag and register the callback.

#### Scenario: Active and removed
- **WHEN** the effect is active and the owner accepts the hook
- **THEN** the hook, target flag and registration are set and the result is 0
- **WHEN** an event with bit 3 arrives
- **THEN** the state is removal, the target is released and the result is 2

### Requirement: Barrier orientation
The orientation pass SHALL swap every point's y and z.

#### Scenario: Two points
- **WHEN** the points are (1, 2, 3) and (4, 5, 6)
- **THEN** they become (1, 3, 2) and (4, 6, 5)

### Requirement: Barrier pose advance
The pose advance SHALL make the current pairs previous, the next pairs current, and store the frame.

#### Scenario: One advance
- **WHEN** the current first is (1, 0, 0), the next first (2, 0, 0) and the frame argument 7
- **THEN** the new first is (2, 0, 0), its previous (1, 0, 0) and the frame 7

### Requirement: Barrier update fraction
The barrier update SHALL compute its fraction as (target frame − base frame) + extra and its position as a + (b − a)·t, with a the previous snapshot's point and b the current one.

#### Scenario: One tick
- **WHEN** the previous point is (0, 0, 0), the current (10, 0, 0), the frames 5 and 3, and the extra 0.1
- **THEN** the fraction is 2.1 and the position (21, 0, 0)

### Requirement: Barrier cast volume
The Barrier's cast sound SHALL play at volume 0.2.

#### Scenario: The constant
- **WHEN** the cast volume is read
- **THEN** it is 0.2

### Requirement: Cast state gate
The Force and Energy gate SHALL wait (−3) for an attacking body in a swing state of 0x34 and fail (−2) any other attacking body; fail a moving body that is not stopping, wait for one at a pace of 2 or more and pass one below with 1; ask a casting body for its release and wait; fail a collapsing body; pass a standing body with 0 unless it is switching weapons (wait); fail a body on the ground, and wait for one getting up past 0.9 of its clip.

#### Scenario: The gate's branches
- **WHEN** the gate reads an attacking body with swing bit 4, one with swing bit 2, a stopping body at pace 1.9 and at 2, a casting body and one getting up at 0.9 and 0.91
- **THEN** it answers −3, −2, 1, −3, −3 with the release set, −2 and −3

### Requirement: Displacement gates
The instant Displacement gate SHALL fail a vanished body and, at a clip rate of 1 or more, a collapsing one; the held gate SHALL fail, at that rate, a collapsing or dead one. Both SHALL test the swing as the state gate does, ask a casting body for its release, and otherwise pass on the word the caller last held, the id of the power being cast.

#### Scenario: The held word
- **WHEN** a standing body with power 0x201 charging meets the instant gate, a vanished body meets it, and a lying body meets both gates
- **THEN** the first passes with 0x201, the vanished body fails, and the lying body passes the instant gate with its word and fails the held gate

### Requirement: Persistent slots
Slots SHALL be the highest pick of 2 to 4 among the record's first `thaumaturgy word / 5000` tiers, one less without the focus item, less one for each running persistent power; a cast of a power that already runs SHALL mark it ending and return 2.

#### Scenario: Two tiers and a running bubble
- **WHEN** a record of word 10000 with picks 3, 1, 4 is read, and a body with two slots running a bubble and kinesis casts the shell and then the bubble
- **THEN** it has 2 slots, the shell has 1 free slot, and the bubble's cast returns 2 with the bubble marked ending

### Requirement: Cast motion start
A running cast's hook that answers 0 or more SHALL keep a new cast from starting; otherwise the hook SHALL clear, the power become the one charging, its release clear and, for a motion above 0, that motion play with the condition casting.

#### Scenario: A held cast
- **WHEN** a blast starts against a hook answering 0, then against one answering −1
- **THEN** the first does not start and the second charges 0x201 with motion 0x22 and condition 0x20

### Requirement: Domain prices
A front end SHALL halve an instant power cast again with the repeat technique, price a sustained power at the tick in milliseconds times its cost times 0.001 (Force and Displacement) or over 1000 (Energy), halved again with the sustain technique, and keep a persistent power's cost.

#### Scenario: A torrent tick
- **WHEN** a torrent costing 0.1 a second is priced for a 12 ms tick
- **THEN** it costs 12 × 0.1 × 0.001

### Requirement: Focus factors
Blast, bolt and shock SHALL need 1.2 times the cost and the long door 0.8 times, in double precision; barrier twice and vanish 80 times in single precision. The player's Force and Displacement front ends SHALL refuse while status bit 0x2_0000 is set or a possession runs.

#### Scenario: Short in double precision
- **WHEN** a body with focus 0.12 casts blast, one with 0.1201 casts it, and one with 0.16 casts the long door
- **THEN** the first is refused, the second starts, and the long door is refused

### Requirement: Force casts
Kinesis SHALL release while the controls drag something; a throw SHALL take what a charging kinesis holds, else be refused while the controls drag, and a pick that finds nothing SHALL still return 1; strikes SHALL need a weapon in hand and start motion 0x25; a running barricade SHALL end only when the cursor's ray meets its wall. A throw SHALL take a moving body off the level that nobody holds: a character only when dead, a prop of mobility 0.01 or more or one marked for any weight.

#### Scenario: Strikes and the barricade
- **WHEN** fists cast strikes, a one-handed weapon casts them, and a running barricade is cast again away from and then under the cursor
- **THEN** the fists are refused, the weapon starts motion 0x25, the first barricade cast raises a new wall and the second ends the running one

### Requirement: Barricade placement
A barricade SHALL stand at the cursor's ground point (15 cm back along the hit's normal unless the surface keeps it), 10 cm up; a probe down of 210 cm that meets the floor SHALL move it there, with the wall's middle 100 cm above. It SHALL face away from the caster on the level and reach 300 cm each way; its placing sound's pitch SHALL be 0.8 + 0.4 times a roll.

#### Scenario: On the floor
- **WHEN** the cursor hits (100, 0, 0) with an upward normal, the floor lies 20 cm below and the caster stands at (0, 50, 0) with a roll of 0.5
- **THEN** the middle is (100, 75, 0), the facing +x and the pitch 1.0

### Requirement: AI advice
The AI SHALL be told 1 for a blast with the focus, facing within 1 and the target on its feet within 6 m; 0x10 for a barrier with half the focus above the cost, facing 1 or more, within 3 m; 3 for a shock within 5 m; and 0x10 for shell, strikes, constant displacement and shock weapons when the cost is within half the focus, facing within 0.5, the power not running and a slot free, else 0x400_0000.

#### Scenario: Range
- **WHEN** the AI weighs a blast at 360000 cm² and at 360001 cm²
- **THEN** it is told 1 and 0

### Requirement: AI cost words
The AI SHALL be told `round(160·cost) << 8 | 4` for the long door against a target on its feet and not vanished, and `round(200·cost) << 8 | 0x800_0000`, with 0x20 at three quarters health or less, for the summoning, and 0x400_0000 while a summoned creature lives.

#### Scenario: The words
- **WHEN** the AI weighs a long door costing 0.2 with focus 0.26, and a summoning costing 0.35 at three quarters health
- **THEN** it is told 0x2004 and 0x800_4620

### Requirement: Energy casts
Shock SHALL start motion 0x27 and be made only when the motion starts, returning 1 either way; its charge SHALL last the cast clip's milliseconds over 1200 and cost the cost times that times 1.2. Shock weapons SHALL need a weapon and a slot, arm at most four weapons, end a put-away weapon's arcs and arm a taken one while fewer than four are armed.

#### Scenario: A full clip and a fifth weapon
- **WHEN** a 1200 ms clip charges a 0.1 shock and four armed weapons see a fifth taken and one put away
- **THEN** the charge lasts 1 and costs 0.12, the fifth is kept off, and the put-away weapon's arcs end

### Requirement: Phasing
A constantly displaced body SHALL let a blow through when a percentage roll is at most `100 >> (s + 1)` for a streak s of 0 or more, or `100 − (100 >> (1 − s))` below 0; a pass SHALL raise a non-negative streak (a negative one restarts at 0) and a miss lower a non-positive one (a positive one restarts at 0). A passed weapon SHALL get its collisions back once more than 500 ms have passed.

#### Scenario: The odds
- **WHEN** the streak is 0, 1, 2, −1 and −2, and weapons passed at 1000 and 1200 ms are restored at 1500, 1501 and 1701
- **THEN** the thresholds are 50, 25, 12, 75 and 88, and the weapons come back at 1501 and 1701

### Requirement: Arc tendrils
Arcs SHALL keep `round(segments·life)` points a tendril, lose 0.1 of life a tick once their timer runs out, open a branch where the chord over a point is 0.6 of a live segment or more, grow one point a tick toward their length, keep 0.9 of their motion with a 0.02-segment jitter, and end at the level where a segment meets it. A respawning emitter SHALL start a free record half the ticks with a life of `life` to `2·life` ticks.

#### Scenario: A single arc
- **WHEN** a non-respawning arc of 4 segments with a 2-tick life ticks 3 times, and a tendril of 4 points along x meets the level at x = 6
- **THEN** its life is 0.9 after the third tick and the cut tendril keeps 3 points, the last at x = 6

### Requirement: Arc strikes
A segment that meets a limb of a character other than the caster SHALL shock that character with the emitter's strength along the line from the arcs' mean point to its chest, once with the shock-once flag, and send a branch toward the limb.

#### Scenario: Once only
- **WHEN** a tendril's first segment meets a character's limb twice with the shock-once flag
- **THEN** the first meeting shocks the character and branches toward the limb, the second does not shock

### Requirement: Arc frame
Each frame the light SHALL flicker within 0.75 to 1.2, the hum's gain be fill² times its flicker (0.5 to 1.5) times the volume and its pitch 0.5 plus half the fill times its flicker (0.8 to 1.1); the ribbon SHALL pair every point and Hermite midpoint, its first and last pair transparent, and drop its last triangle.

#### Scenario: A three-point ribbon
- **WHEN** a three-point tendril along x is drawn facing +z
- **THEN** it has 12 vertices, 27 indices, and its first quad is 0, 1, 3, 0, 3, 2

### Requirement: Barricade rise and fall
A barricade's reaches SHALL be trimmed 40 cm past the level; it SHALL rise over 1.6 s with its span `min(e·300, reach)` each way, `e = u(2 − u)·0.9 + 0.1`, `u = t(2 − t)`, its half width that less 15; told to end, it SHALL fall over 750 ms from where it stands with its half height `(1 − t⁴/2)·100 − 10`, and leave the grid and the scene at the end. Its power SHALL end once its caster is more than 20 m away.

#### Scenario: Half way up
- **WHEN** a barricade has risen for 800 ms
- **THEN** its half width is 268.125 and its half height 90

### Requirement: Barricade shove
A sphere inside the wall's rounded slab SHALL be pushed along its normal by `((r + 10) − |z|)·0.04 + 0.1`, at most 0.5, times `m/(m + 1)`, a tenth from behind, and strike a ripple of three times the push. A character's limb in front within 30 times its scale of the plane SHALL be knocked back by twice 40 along the normal (at most its step), its instability set to 100 and its energy above 0.25 lowered by `(30 − d)·0.0002 + 0.002`.

#### Scenario: A limb in front
- **WHEN** a sphere of 10 cm 5 cm in front of a standing wall, of mobility 1, belongs to a character whose chest is 20 cm in front
- **THEN** it is pushed 0.25 along the normal, knocked back 80, its energy lowered by 0.004 and a ripple of 1.5 starts

### Requirement: Wisp strands
Each strand SHALL keep a ring of eight nodes, start a new head once its head is more than 40 cm from its anchor, light it with its budget of 166 (ramped by `q(2 − q)` over 80 ticks for a body set), spend 50 of its budget a node for a blow's set, darken each node by 1 a tick and by 2 to 6 for the oldest four, grow it by 0.1 and keep 0.9998 of its motion.

#### Scenario: The darkening
- **WHEN** a stopped strand's eight nodes at light 100 tick once
- **THEN** from the head back they read 99, 99, 99, 99, 98, 97, 96, 95

### Requirement: Live effects
The game SHALL run live arcs, barricades and wisps after the bodies, an arc's shock reaching a fighter as the live shock timing states. A barricade's push SHALL move a part's particles by their weights of the struck sphere's centre and restore its edges past the push strain; each part of a character it strikes in front SHALL lower the character's energy while it is above 0.25. `--debug-arcs`, `--debug-barricade` and `--debug-wisps` SHALL put the effects on the player.

#### Scenario: Two shocks within the gap
- **WHEN** a fighter never shocked takes a shock of 0.05 at 0 ms with the generator's state at 0, and then at 348 ms and at 360 ms
- **THEN** its yellow health falls from 0.25 to 0.20, stays at 348 ms (its next shock is due at 500 ms), then falls to 0.15, its next shock due at 866 ms

#### Scenario: Three parts in front
- **WHEN** three parts of a character with energy 0.26 are struck in front, each losing 0.004
- **THEN** its energy is 0.248

### Requirement: Live shock timing
A shock SHALL reach a fighter once the game clock is within 150 ms of its next shock time, or when it was never shocked. Its next shock time SHALL then be the clock plus 500 ms plus the engine generator's draw below 200 ms, and a fighter that is not collapsing or dead SHALL lose the strength in yellow health, floored at 0.

#### Scenario: Two shocks within the gap
- **WHEN** a fighter never shocked takes a shock of 0.05 at 0 ms with the generator's state at 0, and then at 348 ms and at 360 ms
- **THEN** its yellow health falls from 0.25 to 0.20, stays at 348 ms (its next shock is due at 500 ms), then falls to 0.15, its next shock due at 866 ms

#### Scenario: A collapsing fighter
- **WHEN** a collapsing fighter with 0.1 yellow health takes a shock of 0.05
- **THEN** its yellow health stays 0.1 and its next shock time still moves

### Requirement: Power focus and ending
A running power SHALL hold the focus it takes in the body's reserve and spent focus, three quarters of the cost with the reserve technique, and SHALL return the held amount to the reserve only. A control event 2 SHALL mark a power ending and answer 0; the next tick SHALL return its focus and answer 2. A persistent power SHALL publish HUD word 2 while it runs and word 12 with the clock and a 200 ms fade once it ends.

#### Scenario: A persistent power ends over two ticks
- **WHEN** a power takes 0.5 of focus, receives event 2 and then ticks again
- **THEN** the event tick answers 0, the next answers 2, the reserve is back to 0 and the spent focus stays 0.5

#### Scenario: The reserve technique
- **WHEN** a body with the reserve technique gives a power a cost of 0.4
- **THEN** the power holds 0.3 and the reserve and spent focus each grow by 0.3

### Requirement: Timed Mind powers
Confusion and Enrage SHALL clear their status bit and answer 2 once the unsigned 32-bit elapsed game time passes 15 s or the power is ending, without writing the state. Trust and Suspicion SHALL mark themselves ending once the elapsed time passes 20 s.

#### Scenario: Confusion expires
- **WHEN** a confusion applied at 100 ms ticks at 15 100 ms and at 15 101 ms on a body that is confused and masked
- **THEN** the first tick keeps both bits and answers 0, the second clears the confusion bit only and answers 2, and the state stays 0

### Requirement: Instant Mask Mind
Instant Mask Mind SHALL keep the masked bit for 15 s with intensities `0.9·(1−((t−5 s)/10 s)²)` after 5 s and `0.8·(1−((t−10 s)/5 s)²)` after 10 s, worked in double, answering 4 as the second fade begins and 2 once it expires.

#### Scenario: The mask's two fades
- **WHEN** an instant mask applied at 1000 ms steps at 11 500 ms
- **THEN** the first intensity is 0.62775, the second 0.792, and the step answers 4

### Requirement: Loyalty strength
Once 800 ticks have passed, a loyalty target's strength SHALL blend `((relation+100)·0.005)²·100`, kept in double until it is narrowed, at 0.05 with 0.95 of its old value.

#### Scenario: A loyalty strength in double
- **WHEN** a loyalty target past its 800 ticks with strength 0 ticks with relation −98.15
- **THEN** its strength has the bits 971000813

### Requirement: Control Mind binding
Control Mind SHALL count its binding down by the double 0.002 and take the single 0.002 of focus each tick, and SHALL hand control over once 2 s have passed. Taking control back SHALL write the caster's status, less the control bits, over the released body.

#### Scenario: Control comes back
- **WHEN** control is taken back from a body while the caster's status holds the handover bit and bit 0x10
- **THEN** the released body's status is the handover bit and 0x10, and the caster keeps 0x10 only

### Requirement: Force shell and domes
Shell SHALL set the body's hit damping to 0.5 and clear it at the end. A bubble or barricade power SHALL end on an event with bit 1 or 3, and SHALL mark itself ending once its effect is more than 20 m from the caster.

#### Scenario: A dome left behind
- **WHEN** a bubble at the origin ticks with its caster 2000 cm and then 2000.1 cm away
- **THEN** it keeps running at 2000 cm, marks itself ending at 2000.1 cm and answers 2 on the next tick

### Requirement: Force Strikes
Strikes SHALL set the strike factor 1.0 and the swing factor 3.0, keep at most four weapon auras, and ripple the aura of a swung weapon at half the swing's strength.

#### Scenario: A swing
- **WHEN** a weapon with an aura is swung at strength 3
- **THEN** its aura ripples at 1.5

### Requirement: Kinesis steering
Kinesis SHALL steer its held body at its reach (half the number of parts whose parent is the held part, plus one) along the unit direction to the aim, or the direction times 0.02 within 50 cm or for a distance that is not a number, and SHALL turn it by `reach²` times the double 0.001 a tick, each key pair tested if/else.

#### Scenario: Both turn keys
- **WHEN** a held body with 100 parts linked to its held part has all four turn keys held
- **THEN** it turns forward on both axes by 2601 times the double 0.001, narrowed

### Requirement: Kinesis pick and aim
While the player's kinesis waits for its pick, a prop the hand could lift SHALL be in reach wherever it is and SHALL be handed to the kinesis, and any other press SHALL be out of reach. Each tick a cursor hit within 40 m SHALL push the held body toward the aim: the hit raised `100 − depth` (at least 10) back along the ray, kept when the head's line to it is clear, else the last aim. Without a hit the body SHALL NOT be pushed, but the keys SHALL still turn it.

#### Scenario: A lifted crate
- **WHEN** the player's kinesis runs and the hand lifts a crate
- **THEN** the kinesis holds the crate and aims at its centre, and the hook goes when the kinesis ends

#### Scenario: Only a prop is in reach
- **WHEN** the kinesis waits and the player presses a far crate, then a near corpse
- **THEN** the kinesis holds the crate and the corpse press is refused as out of reach

### Requirement: Throw flight
Throw SHALL hold no reserve. Its lift SHALL aim 100 cm above the body (at its centre, spending nothing, when a kinesis handed it) and spend tick ms × cost × 0.00125 every steered tick at `mass/(0.1+mass)` of the unit direction; 800 ms after the lift it SHALL drop the shell and coast at six times that share toward the caster's aim point, ending at once when that point is within 100 cm, and restore the body's materials 200 ms after the drop.

#### Scenario: A throw's flight
- **WHEN** a throw of cost 0.1 lifts a body of mass 0.9 at 1000 ms with a 12 ms tick and its caster aims 500 cm away along x
- **THEN** it aims 100 cm above the body, spends 0.0015 on each steered tick, drops the shell at 1801 ms, coasts at six times the mass share along x and restores the materials at 2002 ms

### Requirement: Throw body
A throw SHALL take the body the caster's current kinesis cast holds, and SHALL be refused when that kinesis holds none; without a kinesis cast it SHALL take the first body under the cursor past the level: a dead character, or an unheld prop whose mobility in double is at least 0.01. Each tick its velocity SHALL move the body's first part by its origin weights, less a fifth of its motion while steered, and wake it; a body gone SHALL end the throw.

#### Scenario: A kinesis crate thrown
- **WHEN** a throw is cast while the player's kinesis holds a crate
- **THEN** the kinesis ends and the throw holds the crate, aiming at its centre and spending nothing a tick

#### Scenario: An empty kinesis
- **WHEN** a throw is cast while the player's kinesis holds nothing
- **THEN** the throw is refused

### Requirement: Throw sounds
A throw from the cursor SHALL play `thmfrc_knsst` at its body at volume 0.3 and pitch `0.8 + 0.4·roll`, and every lift `thmfrc_throw` there at 0.6 and `1 + 0.2·roll`, each roll in double before the variant. A throw that ends before its drop SHALL fade the lift's voice.

#### Scenario: A throw ended in flight
- **WHEN** a throw from the cursor lifts its body and is ended before 800 ms
- **THEN** both sounds have played at the body and the lift's voice fades

### Requirement: Energy power classes
Shock's first control tick SHALL install its cast hook and play the charge; each later tick SHALL add the charge rate, spend the tick's focus while charging when the charge is not past 1, and past 1 fire one arc burst and clear the hand's arcs, the cast hook and the cast id; a second charge past 1 SHALL answer 2. Shock Weapons SHALL start cast motion 0x25 and clear its weapon arcs when it ends.

#### Scenario: One burst
- **WHEN** a shock with rate 0.5 ticks four times after its first tick
- **THEN** the second and third ticks spend focus, the fourth fires the burst and clears the cast id, and a later charge past 1 answers 2

### Requirement: Vanish phases
The vanish effect SHALL wait 400 ms spending the power's tick cost, hide the body with its vanished bit, fade it out over 300 ms spending the tick cost, hold (Vanish until released, spending half its cost; the Long Door 400 ms, spending nothing), move it, fade it in over 400 ms with the share narrowed before it is taken from 1, and finish 300 ms later.

#### Scenario: A vanish
- **WHEN** a vanish begun at 0 ms with a tick cost of 0.001 is released at 1101 ms
- **THEN** it spends 0.001 until the body vanishes at 400 ms, is half faded at 550 ms, holds from 700 ms spending 0.0005, moves at 1101 ms, is 0.25 shown at 1201 ms and is done at 1801 ms

#### Scenario: The Long Door holds by itself
- **WHEN** an AI's long door finds its place at 20 ms
- **THEN** the body vanishes at 420 ms, the hold spends nothing and the body moves at 1120 ms without a release

### Requirement: Constant Displacement and the Summon
Constant Displacement's hum SHALL rise by the double 0.002 each control tick while its volume is under the double 0.12. The summon SHALL wait 2 s after its demon is created, fade the caster in over 3 s and finish 1 s later.

#### Scenario: The summon's fade
- **WHEN** a summon's demon is created at 4 ms
- **THEN** the fade starts at 2004 ms, the caster is half shown at 3504 ms and the summon is done at 6005 ms

### Requirement: Power lists and dispatch
The known-power search SHALL return the index of an id in a record's known powers, or none; ending by id SHALL mark the first running power with that id ending. AI evaluation SHALL dispatch on the id's domain byte (0x200 Force, 0x400 Energy, 0x600 Displacement) and answer 0x400_0000 for any other.

#### Scenario: A Mind id
- **WHEN** an AI evaluation asks for id 0x2010C
- **THEN** it answers 0x400_0000

### Requirement: Saved power rebuild
A saved Force power SHALL be rebuilt with its raw id; with the reserve technique its held focus SHALL be taken down by 0.75 once more after the rebuild's own three quarters.

#### Scenario: A rebuilt shell with the technique
- **WHEN** a body with the reserve technique loads a saved Force Shell of cost 0.35
- **THEN** the reserve grows by 0.2625 and the rebuilt power holds 0.196875

### Requirement: Power Glean
Power Glean SHALL teach Vanish, or else Shock, from a dying shaper's known powers to a player who knows at least one power and none of that domain, unless the death is more than 20 m away; a distance that is not a number SHALL pass.

#### Scenario: Glean prefers Displacement
- **WHEN** a shaper knowing Shock and Vanish dies 1 m from a player who knows only a Force power
- **THEN** the player learns Vanish with word 0x9fff and `Power_Glean` shows for 3 s

### Requirement: Mind cast gates
A Mind cast SHALL first ask for focus of 0.3 times its cost, in double precision, a NaN passing, and then a picked target; record kinds 1 and 2 SHALL be living, 4 and 5 undead, and any other SHALL refuse after what the cast already spent. An undead target SHALL take the cost (half for Control Mind) and answer 0. Instant casts SHALL add their cost to the spent focus; persistent casts SHALL hold it through their power class's reserve.

#### Scenario: Calm on the three kinds
- **WHEN** Calm costing 0.14 is cast on a living, an undead and a kind-3 target by a caster with focus 1
- **THEN** it answers 1, 0 and −1, and the spent focus is 0.42
- **WHEN** the caster has focus 0.04
- **THEN** it refuses and spends nothing

### Requirement: Calm
Calm SHALL settle every contact's concern and hostility into its regard (over 10 losing 10 and giving 30, otherwise, or a NaN, all of it three times over), lower alert by 10, stress by 50 and fear by 1, each floored at 0 with a NaN kept, and set the calm factor to 1.

#### Scenario: A worried contact
- **WHEN** a contact with regard 1, concern 12 and hostility 4 is calmed
- **THEN** its concern is 2, its hostility 0 and its regard 43

### Requirement: Timed Mind effects
Enrage SHALL add 50 alert and the enraged bit with its timed class stamped on the scene clock; Confusion SHALL add 50 alert, clear every contact and the AI's level and focus, and set the confused bit; Trust SHALL place its marker toward the caster, refused on a body already loyal to it, and halve the contact toward the caster with its regard set to 30; Suspicion SHALL need the whole cost and place its marker; Fear SHALL add 20 fear up to 40 and Embolden 50 courage up to 80.

#### Scenario: Trust toward a loyal body
- **WHEN** Trust is cast on a body already bound to the caster by Loyalty
- **THEN** it refuses and spends nothing

### Requirement: Loyalty cast
Loyalty on a body bound to the caster SHALL end the binding and answer 2, and on a body bound to another caster SHALL refuse; otherwise, with a free slot and the whole cost, the target SHALL take the marker with strength `((relation + 100)·0.005)²·100` (the sum in single precision), its contact toward the caster SHALL settle with regard 30 and its AI re-arm, and the caster SHALL run Loyalty holding the cost.

#### Scenario: A relation of 20
- **WHEN** Loyalty is cast on a target whose relation to the caster is 20
- **THEN** the marker's strength is 36 and the caster's reserve holds 0.29

### Requirement: Control Mind cast
Control Mind cast while it runs SHALL end it unless it is still starting; it SHALL need a free slot (none while Spirit Form runs) and the whole cost; a target with technique bit 0x10000 SHALL resist, taking half the cost and answering −1; otherwise the binding SHALL start without spending.

#### Scenario: A resisting target
- **WHEN** Control Mind costing 0.35 is cast on a resisting target
- **THEN** it answers −1 and the spent focus is 0.175

### Requirement: Dead powers
Raise Dead and Command Dead SHALL need a collapsed body not yet raised whose character allows it; a body with no decay SHALL be revived with decay 0.2, gait scale `1.25 − decay²/4`, pace `round((1 − decay/4)·1660/83)`, most energy `1 − decay/10` and stamina `(1 − 0.3·decay)·(1 − 0.2·load)·80·build`, and its yellow and red health SHALL become 0.5 and 1. Command Dead SHALL place its marker and run the caster's power, and cast on a standing body the caster commands SHALL end the command.

#### Scenario: A revival
- **WHEN** a corpse with load 0.5 and build 1 is revived
- **THEN** its gait scale is 1.24, its pace 19 ticks, its most energy 0.98 and its stamina 67.68

### Requirement: Sense and mask casts
The instant Sense Minds SHALL last 4000 ms or the focus's share of it, rounded to even, on the real clock, and show the overlay that long, refused while the persistent one runs; the instant Mask Mind SHALL stamp its class with the scene clock and set the masked bit, refused while the persistent one is slotted or runs; a persistent cast SHALL end its running self or end the instant one beside it and start its class holding the cost.

#### Scenario: Half the focus
- **WHEN** a caster with focus 0.065 casts the instant Sense Minds costing 0.13
- **THEN** the overlay shows for 2000 ms

### Requirement: Spirit Form cast
Spirit Form SHALL need 0.05 of focus in double precision; a running form SHALL end unless its screen is still coming up; otherwise the form SHALL start with the cost as its drain, set status bit 0x100000 and ask for the spirit screen, its music at 0.1 (0.15 with the music option) and the spirit body.

#### Scenario: A second cast
- **WHEN** Spirit Form is cast twice
- **THEN** the first starts the form and the second ends it

### Requirement: Mind front end and restore
While the body runs Control Mind only Control Mind SHALL be cast, on the held body; rows SHALL be priced as the shared front end prices with the tick over 1000. A saved persistent Sense Minds, Mask Mind, Loyalty or Command Dead SHALL be rebuilt holding its whole cost, its cost reserved (three quarters with the reserve technique), and the load SHALL take three quarters of what it holds with the technique.

#### Scenario: A restored Loyalty with the technique
- **WHEN** a saved Loyalty is loaded on a body with the reserve technique
- **THEN** it holds 0.2175 against a reserve of 0.2175

### Requirement: Mind-sight tracker
At most every 4 ms the tracker SHALL walk the scene's characters but the player and the body it controls; a living or undead kind SHALL get an entry, reusing a freed slot, its values eased toward the mood's by `exp(−ln 2·dt/1000)` (a half past a second); a collapsed body SHALL fade over 5 s and an out one over 3 s from when first seen so and then be dropped; a collapsed body without an entry SHALL not be added.

#### Scenario: A body collapses
- **WHEN** a tracked body collapses at 1000 ms and the tracker runs at 3500 ms and 6001 ms
- **THEN** its fade is 0.5 and then its entry is dropped

### Requirement: Blast charge and release
A blast SHALL hold sixteen chunks on the icosahedral directions 16 cm out in the hand's frame, aimed within 45° of the facing; charging, the chunks SHALL follow the hand and grow `0.0015·swell`, their trails easing a tenth toward 9 cm back, the caster SHALL pay the drain (running out fizzles) and the charge rise five sixths of the tick; past 1 the characters within 1800 cm SHALL be alerted by `0.25·(1 − (d/1800)⁴)·(aim·dir + 3)` and the chunks released along their forward-pushed directions.

#### Scenario: A charge passing 1
- **WHEN** a blast started on a 1300 ms tick charges once
- **THEN** it is thrown, its release sound pitch lies in 0.9..1.1 and "PowerUsed" is raised

### Requirement: Blast thrown chunks
A thrown chunk SHALL grow its reach by `growth`, its radius by `swell·(0.7·|move| + 3)·0.0006` and its trail by `(0.7·|move| + 3)·0.07 + 1`, move by its last move plus `weight·(1 − charge)²·2.4` of its direction, times `retention`, spend `round(|move|²·wear)` life when worn, and push apart from chunks closer than twice its reach.

#### Scenario: The first thrown tick
- **WHEN** a released blast at charge 0.5 ticks with its chunks still
- **THEN** each chunk moves `weight·0.6·0.96` along its direction, its reach is 4.4, its radius 0.0018 and its trail head has moved on

### Requirement: Torrent feed
A torrent SHALL hold up to 24 streams of sixteen points, sixteen born 16 cm out at the blast's directions, a point living 66 ticks; charging SHALL feed each head 15 cm and the point before it 30 cm along the aim, and past a charge of 1 the torrent SHALL run.

#### Scenario: A charge passing 1
- **WHEN** a torrent started on a 1300 ms tick charges once
- **THEN** it runs, each head 9 cm on from where it was, and "PowerUsed" is raised

### Requirement: Torrent stream ticks
A running torrent SHALL spawn a head after `8 − roll(3)` ticks along a direction drawn round the aim, age every live point, pull neighbours by `0.1·(1/2 − 2500/(d² + 2500))`, keep 0.999 of each point's move, split a stream where two points lie more than 1000 m apart, and remove a dead stream other than the first by moving the record before the last into its place.

#### Scenario: A dead stream
- **WHEN** stream 5 of sixteen dies while running
- **THEN** fifteen streams remain and record 5 holds what record 14 held

### Requirement: Force limb push
A limb struck by a blast chunk moving into it SHALL take `move·f·(−k)·0.2` with `k = (−inverse/(inverse + share))·push` and `f` clamped to 0..1, and the chunk SHALL move by a quarter of its share of the contact less a twentieth of its move; a torrent point's limb impulse SHALL be `k·f·straightness` of its move plus `(contact − point)·(1 − straightness)·(−k)`.

#### Scenario: A chunk into a limb
- **WHEN** a chunk moving 10 cm hits a limb of inverse mass 1 with share 0.2 and push 0.5
- **THEN** the limb takes an impulse of 10·(0.5/1.2)·0.2 along the move

### Requirement: Force character knock
A struck character SHALL be knocked by its gathered impulse, normalised, capped at 30 and doubled, when the impulse is longer than 1, with a stun of `(1 − k)·0.08` that is 0 below 0 and 0.008 above 0.08.

#### Scenario: A heavy impulse
- **WHEN** a character of mass 80 gathers an impulse of 100 along x
- **THEN** it is knocked 60 along x with a stun of 0.078

### Requirement: Arc level clip
A tendril SHALL end at the hit on the first segment that meets the level, losing the branches from that segment's end point on.

#### Scenario: A wall across the third segment
- **WHEN** a tendril's points run 0, 4, 8, 12 cm along x and the level is met at 6 cm
- **THEN** the tendril keeps three points, its last at 6 cm

### Requirement: Arc character query
Each tendril segment but the last SHALL be given to the character's query whole (start, end, unit direction and length) with a value of 20 cm; the first segment that meets a limb of another character SHALL shock it, once per character when the emitter shocks once, and the tendril SHALL send a branch toward the limb from the point before.

#### Scenario: The first segment meets
- **WHEN** the first 4 cm segment meets a limb
- **THEN** the query received a length of 4 and the value 20, and the tendril branches from its first point

### Requirement: Force shell frame
A force shell SHALL open over 1.5 times its size in ms, rest, close over its size from what it shows and, the frame after it closes, leave the scene with its light; a one-shot shell SHALL close as soon as it opens and, while closing, stay fully shown. While a shell has its light the light SHALL be (0.14, 0.08, 1) times a tenth of the light curve and a closing shell's colour 2.8 times what it shows.

#### Scenario: The frame after closing
- **WHEN** a closing shell reaches zero
- **THEN** that frame still draws and the next one removes the shell

### Requirement: Rigid shell copy
A rigid shell copy SHALL move every mesh vertex out by twice the unit sum of the normals of the faces it belongs to, and SHALL keep the triangles of the visible sections through the mesh's vertex map.

#### Scenario: A vertex without faces
- **WHEN** a vertex belongs to no face
- **THEN** it stays where it is and its normal is zero

### Requirement: Shell fade plane
The shell's fade plane SHALL be the node frame's height column with an offset of the frame's height less the bottom of the node's box (the skin's for a skinned copy), all divided by the box's height and multiplied by the plane scale; a rigid box's height SHALL be its world height, lowered to its radius when greater.

#### Scenario: An upright box
- **WHEN** an upright node frame stands at height 100 over a box centred at 90 with a half height of 50 and a radius of 60
- **THEN** the plane is (0, 1/50, 0, 60/50)

### Requirement: Power grant target
A grant SHALL need the seventh insight technique. The target SHALL be the last node whose power, with bits 20 to 23 cleared, equals the granted power, and it SHALL not be the first node. Its first link SHALL name a prerequisite node, one-based; zero names none, and a name past the nodes reads as unlearnt.

#### Scenario: Refusals
- **WHEN** the first node is granted, a node with no link is granted, a power no node holds is granted, a node whose link names a node past the list is granted, and a linked node is granted without the technique
- **THEN** every grant is refused and the known list is unchanged

### Requirement: Power grant write
A node's flags SHALL be the flags of the last known pair whose id, with bits 20 to 23 cleared, equals the node's word, or zero. The prerequisite SHALL have bit 0x1000. The power SHALL then overwrite the first known pair that matches it with the flags 0x9fff, or be appended.

#### Scenario: A prerequisite and a granted chain
- **WHEN** power 0x102, linked to the learnt node 0x101, is granted with the technique, then 0x103 linked to the granted 0x102
- **THEN** the first grant appends 0x102 with 0x9fff, the second succeeds because the granted flags carry the learnt bit, and the list holds three pairs

### Requirement: Tree opening
A tree opened by a character other than the last one SHALL open the mind domain; the same character SHALL reopen the last domain. A record that knows no powers SHALL be seeded with two pairs: 0x101 with 0x1000, and 0x01020101 with 0x2800. A record that knows powers SHALL not be seeded.

#### Scenario: First and repeated opening
- **WHEN** the domain is chosen for a new character after the displacement domain, for the same character after it, and an empty record and a starter record are seeded
- **THEN** the domains are mind and displacement, the empty record gets the two pairs, and the starter record keeps its two

### Requirement: Running power limit
Running powers SHALL be counted in list order, skipping any whose id has bit 0x100000. Each power counted beyond the allowed slots SHALL be set to state 2, and the check SHALL report failure when it set one.

#### Scenario: One slot
- **WHEN** powers 0x20207, 0x20208 and 0x102020b run in state 1 and one slot is allowed, and then power 0x20207 alone runs with one slot
- **THEN** the first stays in state 1, the other two are set to state 2 and the check fails, and the lone power passes

### Requirement: Power bar slots
The bar SHALL lie ten slots across, 68 pixels apart, the first centred 194 pixels from the left at scale 4. A press within the slot's half-width SHALL name the slot, and a press left of the first slot SHALL name none. The enlarged row SHALL take the nearest slot within 48 pixels.

#### Scenario: First, last and outside
- **WHEN** the bar is pressed at 194, at 194 plus nine pitches, 29 pixels left of the first slot, and, enlarged, 10 pixels short of the second slot
- **THEN** the slots are 0, 9, none and 1

### Requirement: Effect scroll
A centred texture scroll SHALL give each layer `((layer + frame) + 0.5) * 0.002 / divisor + base - 0.001`, taken in double precision and narrowed once, and any other `((layer + frame) + 0.5) * step / divisor + base - step * 0.5` in single precision. The divisor SHALL be the effect shader's global, which holds 1. Zero layers SHALL give no offsets.

#### Scenario: Three layers
- **WHEN** a centred scroll of 3 layers at frame 10 and base 0.5 and a plain scroll of 2 layers at frame 4, base 0.25 and step 0.75 are taken
- **THEN** the centred offsets are 10.5, 11.5 and 12.5 times 0.002, plus 0.5, less 0.001, and the plain first offset is 4.5 times 0.75, plus 0.25, less 0.375

### Requirement: Force creator looks
Each Force creator SHALL look up its sound ids and store them without playing them. The blast family SHALL wear the force material, the domes and the barricade the barrier material and the shell the aura material. The barricade SHALL carry its sound ids, then a pitched play of `thmfrc_ss` at 0.8 + 0.4 of a draw, and the shell and the strikes a pitched play of `thmfrc_ss` at 0.6 + 0.4 of a draw, at volume 0.3. The materials SHALL bind the parameters 0.25, 0.25, 0.25 and 1.

#### Scenario: Materials and sounds
- **WHEN** the blast, bolt, barrier and shell creators are read, the shell start pitch is taken for a draw of 0.5, and the bubble's fourth sound is read
- **THEN** the blast sounds are `thmfrc_cu01` and `thmfrc_bb`, the bolt's second sound is `thmfrc_blt`, the barrier wears the barrier material with a 0xb8-byte block, the shell wears the aura material and its active power is 0x20208, the pitch is 0.8, and the bubble's fourth sound is `thmfrc_brf`

### Requirement: Constant displacement veil
A constantly displaced body SHALL start its hum at the product of 2, 0.8 and a draw, taken in double precision and narrowed. The displaced bit SHALL be set on the body. The first hit SHALL pass when the streak is zero and clear bits 0xC000 of the hit node and its children. The restore after half a second SHALL give the phased nodes back and end the shell.

#### Scenario: A hit and its restore
- **WHEN** a veil over a body with one child starts, is hit at 1000 ms on its first child, and is restored at 1500 and at 1501 ms
- **THEN** the hum start is the double-precision product, the hit passes and records its wisp, the 1500 ms restore gives nothing, and the 1501 ms restore gives that child back with the shell event ended

### Requirement: Bar cast entry in the game
The game SHALL hand the chosen slot's word, unchanged, to the bar's cast entry, which SHALL dispatch by the word's domain byte (Force, Energy, Displacement; a Mind word is refused) and create the started power's class after the front end answers a start.

#### Scenario: A power the tree added
- **WHEN** the record's second slot holds the Shock id although the player was granted only the barricade, and that slot is picked and used
- **THEN** the cast starts a Shock

#### Scenario: A barricade
- **WHEN** the barricade is cast with a pool of 1.0
- **THEN** the cast answers started, 0.25 of focus is reserved and spent, and one wall entity exists

### Requirement: Active power walk
Each body tick SHALL pass control event 0 to every active power, and event 2 to a power marked ending, and SHALL take a power that answers with bit 2 out with the last entry in its place. The release flag SHALL end no power by itself; only through the cast hook while a cast motion plays.

#### Scenario: A shell pressed again
- **WHEN** the shell runs and is cast again
- **THEN** the cast answers ended, the shell is marked ending and the next tick removes it, clears the cast id and returns its reserve

#### Scenario: The release flag
- **WHEN** the body's release flag is set while a shock runs, first without and then with its cast motion playing
- **THEN** the shock keeps running, then its hook is called, the cast ends and the charge clears

### Requirement: Vanish pressed again
Casting vanish while it runs SHALL answer released and ask the body's hook event 0x10, ignoring its answer and starting nothing; the running vanish's hook SHALL take the event as its end on its next tick.

#### Scenario: Second press
- **WHEN** vanish is cast twice and a tick runs
- **THEN** the second answer is released, the hook was asked event 0x10, and the running vanish is marked released after the tick

### Requirement: One record for the tree and the caster
The caster SHALL take the screen's record and bar when the tree closes, and the screen's closed bar SHALL close the caster's bar.

#### Scenario: A tree session
- **WHEN** the tree opens, fills slot 3 and a learnt pair, and closes
- **THEN** the caster's record holds both and its known powers number three

### Requirement: Duellist casts
Before a duellist's combat state runs, the game SHALL answer each of its record's ten slots with that power's domain AI evaluator over its target's distance, bearing, condition and status. The pool SHALL be the caster's focus. The power the combat state chooses SHALL start through its domain's front end as an AI caster and become an active power as a bar cast does.

#### Scenario: A shock within reach
- **WHEN** a duellist knowing only the shock, with 0.5 focus, faces a target 3 m ahead, and then 6 m ahead
- **THEN** its first slot answers 3, and then 0

#### Scenario: The chosen shock
- **WHEN** the combat state has chosen the shock
- **THEN** the cast answers started, a Shock and its arcs exist, the choice is taken and the body reads casting

### Requirement: Power factors on blows
A caster's fighter SHALL carry its body's shell damping and strike factor, which the hit formula reads. A blow the fighter takes SHALL keep 1 minus the damping of its impact, and a blow it deals SHALL be multiplied by 1 plus half the strike factor. An ended shell or strikes SHALL leave 0.

#### Scenario: A running shell
- **WHEN** the shell is cast, and then cast again and the next tick ends it
- **THEN** the fighter's shell factor is 0.5, and then 0

#### Scenario: A shelled blow
- **WHEN** the same cut lands on a fighter with a shell of 0.5 and on one without
- **THEN** the shelled blow's impact is half, and the shelled fighter loses less yellow

### Requirement: Capacity settle
Each caster's body tick SHALL, before its power walk and while the reserve's change is beyond 0.001 either way, move a share of that change into the capacity (not below 0), the share climbing by 0.001 a tick to at most 10; otherwise the share SHALL be 0. A hold SHALL add to the change and a release SHALL take the hold away from it.

#### Scenario: A hold lowers the capacity and a release restores it
- **WHEN** a persistent power holds 0.25 and many ticks pass, and then its hold is released and many ticks pass
- **THEN** the capacity falls to 0.75 and comes back to 1

### Requirement: Focus settle
After its power walk, with more than 0.0001 spent, a body tick SHALL take a share climbing by 0.001 a tick (to 10) of the spent focus from both it and the pool, neither below 0. With nothing spent the share SHALL fall by 0.01 a tick to 0 and the pool SHALL regain the square root of the capacity times the rate times 0.1, up to the capacity: 0.000301205, half again with technique bit 0x20_0000, plus the body's bonus.

#### Scenario: Spent focus leaves the pool
- **WHEN** 0.2 is spent on a full body and a tick settles
- **THEN** the share is 0.001, the pool is 1 − 0.0002 and 0.2 − 0.0002 is still spent

#### Scenario: The pool regenerates
- **WHEN** nothing is spent on a body of capacity 0.25 with technique bit 0x20_0000
- **THEN** its pool gains 0.5 × 1.5 × 0.000301205 × 0.1 a tick, up to 0.25

### Requirement: HUD focus
The player's HUD focus bar SHALL show its caster's pool and capacity, and 1 and 1 for a body that casts nothing.

#### Scenario: A caster's bar
- **WHEN** the player's caster has a pool of 0.4 and a capacity of 0.75
- **THEN** the HUD's focus is 0.4 and its capacity 0.75

### Requirement: A character's caster
A duellist's or placed character's caster SHALL take its body's technique word from the record's learnt techniques, each perk at the bit the original assigns it (Sustain 0x10_0000, Clarity 0x20_0000, Abstraction 0x40_0000, Iteration 0x80_0000, Volition 0x1_0000), and its persistent slots from Concentration's points and techniques by the slot count. A record with no known power SHALL make no caster.

#### Scenario: Concentration's techniques
- **WHEN** a record with powers has 15000 Concentration points and Sustain, Continuity II and Abstraction
- **THEN** its caster's technique word is 0x50_0000 and it has three persistent slots with the focus item, and with 5000 points none

### Requirement: A cast blast
A blast or bolt cast SHALL make a blast entity whose start plays cast motion 0x22 with the setup's power id and, when that motion starts, sets the caster's cast hook to answer -1. While it charges, any call of that hook SHALL fizzle it, and it SHALL pay its drain each tick whether or not it holds the hook.

#### Scenario: A duellist's blast
- **WHEN** a duellist with focus casts the blast at a body whose limb sits in front of its hand, while its body still reads casting
- **THEN** it charges on the hook for 101 ticks, is thrown with the hook and power id cleared, moves that limb and leaves the scene once its chunks are spent

#### Scenario: A later cast fizzles it
- **WHEN** another cast starts while the blast charges
- **THEN** the blast fizzles

### Requirement: A cast barrier
A barrier cast SHALL make a dome entity of radius 100 that pays the power's cost a tick (half once standing) and whose start plays cast motion 0x23 with power id 0x10205, its hook ending the dome and answering -1. A later cast, the release flag or a focus shortfall SHALL call that hook; the dome SHALL push other bodies' limb spheres out of its shell and leave the scene once shrunk.

#### Scenario: A duellist's barrier
- **WHEN** a duellist with focus casts the barrier with a body's limb inside its shell
- **THEN** the dome grows on the hook, drains focus, moves that limb and opens a ripple; the release clears the hook and the dome shrinks and leaves

### Requirement: A cast bubble
A bubble cast SHALL make a dome entity of radius 120 at the cast place whose start plays cast motion 0x24 with power id 0x20207 and takes a hook answering 0 to events with bits 0x28, refusing a cast's start (0x20). Standing SHALL clear the hook and power id, leaving the cast motion playing; the power's ending, or its caster being gone, SHALL break the dome once, and it shrinks and leaves. It SHALL hold its caster's parts in and push other bodies' covered parts out.

#### Scenario: A duellist's bubble
- **WHEN** a duellist with focus casts the bubble with a body's limb inside its reach, then casts it again once it stands
- **THEN** the dome grows on the hook, moves that limb, stands with the hook cleared, then shrinks and leaves the scene

#### Scenario: The caster is gone
- **WHEN** a bubble's caster is removed
- **THEN** the dome breaks once, shrinks over 700 ms and leaves the scene

### Requirement: A cast wave
A wave cast SHALL make a wave entity round the caster's body root whose start plays cast motion 0x24 with power id 0x204 and sets the blast's hook. While it charges it SHALL pay the cost times the tick's seconds; any call of its hook SHALL fizzle it, and a shortfall SHALL end the cast through the cast end, fizzling the wave only while it holds the hook.

#### Scenario: A duellist's wave
- **WHEN** a duellist with focus casts the wave at a body whose limb sits in front of its hand
- **THEN** it charges on the hook, is thrown with the hook and power id cleared, moves that limb and leaves the scene once spent

#### Scenario: A wave runs short
- **WHEN** a charging wave that holds the hook cannot pay its tick
- **THEN** it fizzles, the cast ends and the caster returns to its stance

### Requirement: A cast torrent
A torrent cast SHALL make a torrent entity at the caster's right hand whose start plays cast motion 0x26 with power id 0x10203 and sets a hook answering -1 that fizzles a charging torrent and ends a running one. It SHALL pay its drain while charging and half while running; a later cast, the release flag or a shortfall SHALL call the hook. It SHALL leave the scene once a frame would build no triangles.

#### Scenario: A duellist's torrent
- **WHEN** a duellist with focus casts the torrent at a body ahead of its hand, then lets go once it runs
- **THEN** it charges and runs with the hook kept, moves that body's limb, ends on the release with the hook and power id cleared, and leaves the scene once its streams are spent

### Requirement: Cast hooks answer by event
A cast hook SHALL answer 0 to an event carrying any of its kept bits and -1 to any other, except that a hook keeping every bit SHALL answer 0 to every event and the vanish's hook from its hold on SHALL answer -1 to any event with a 0x120 bit and 0 to any other. A cast SHALL NOT start while the running hook answers 0x20 with 0 or more.

#### Scenario: A shell's raise
- **WHEN** a shell starts its raise and another cast asks to start before the Force Raise clip ends
- **THEN** that cast is refused, and once the clip ends into the stance the hook and power id are dropped and casts start again

#### Scenario: Strikes take their hook with the motion
- **WHEN** Force Strikes or a barricade starts its cast motion
- **THEN** its hook refuses another cast until the clip ends, and a strikes cast whose motion is refused makes no strikes

#### Scenario: A kinesis hands its body to a throw
- **WHEN** a throw is cast while a kinesis holds a body
- **THEN** the kinesis ends without ending the cast, its hook refuses other casts, and the throw plays on in the kinesis's motion

#### Scenario: A vanish's hook lets a cast through once it holds
- **WHEN** another cast asks to start while a vanish runs
- **THEN** it is refused before the vanish's hold and allowed from the hold on, and either way the vanish is let go

### Requirement: The player's buttons and the release ask the hook
Each tick a living player's controller SHALL ask the hook with 1 while the left button is down, with 8 outside combat mode, and in combat mode, with the body casting and its tone not under 0.85, with 2 while the right button is down or the stick's length² passes 0.1 with Sprint held, else 1 for the left. A negative answer SHALL end the cast, and a casting body SHALL ask no attack.

#### Scenario: Strikes held through the buttons
- **WHEN** a strikes hook is asked by the left button out of combat mode, then by both buttons in combat mode while casting
- **THEN** it keeps the cast for 1 and 8, and the right button's 2 ends it

#### Scenario: A shock outside combat mode
- **WHEN** the player leaves combat mode while a shock charges
- **THEN** the controller's 8 ends the cast and the charge clears

#### Scenario: A weak caster
- **WHEN** the player, casting in combat mode at tone 0.8, holds the right button
- **THEN** the hook is not asked with 2

### Requirement: An attack start asks the hook
An attack start, the player's past its energy check or a duellist's, SHALL first ask a set cast hook with event 0x100: an answer of 0 or more SHALL refuse the attack, and a negative one SHALL end the cast and let the attack go on.

#### Scenario: Attacking during a vanish
- **WHEN** an attack starts while a vanish runs before its hold
- **THEN** the attack is refused and the vanish is let go

### Requirement: A duellist lets its cast go
A casting duellist's combat state, past 1000 ms from its cast's start, SHALL ask its power id's domain evaluator and, only when that answers 0, the hook with 0x10; a negative answer SHALL end the cast and clear the AI's casting. Before its state the AI SHALL clear its casting when the body does not read casting past 500 ms from the start.

#### Scenario: A shock the evaluator no longer answers
- **WHEN** the combat state asks while the shock's evaluator answers, then once the target is out of its reach
- **THEN** the hook is not asked first, and then the cast ends with its hook

### Requirement: A body's user stamp
A kinesis pick, and each tick a throw coasts after its shell drops, SHALL stamp the body with the caster until 2 s on. A limb the body hurts while the stamp holds SHALL add to the struck brain's concern toward the caster with amount 0.9 and gain three times the damage, when the caster is a contact.

#### Scenario: A thrown crate hurts a guard
- **WHEN** a body thrown within the last 2 s hurts a character who knows the caster
- **THEN** that character's concern toward the caster grows

### Requirement: Blast shortfall and throw
A charging blast whose focus falls short while a cast hook is set SHALL end the cast through the cast end, fizzling the blast only while that hook is its own. The blast's throw SHALL clear the hook and the power id itself, leaving the cast motion playing, and a thrown blast SHALL push the limbs its chunks reach.

#### Scenario: A refused motion's blast
- **WHEN** a blast whose cast motion was refused, so no hook is set, charges with too little focus
- **THEN** it pays its drain and keeps charging, and the cast is not ended

#### Scenario: The throw keeps the motion
- **WHEN** a charged blast is thrown
- **THEN** the hook and power id are clear, the condition still reads casting and the cast motion plays on

### Requirement: Wave throw
A wave's throw SHALL clear the hook and the power id itself, leaving the cast motion playing, and a thrown wave SHALL push the limbs its chunks reach.

#### Scenario: A paid wave thrown
- **WHEN** a paid wave is released
- **THEN** it is thrown with the hook and power id cleared and the cast motion still playing

### Requirement: The cast end
The cast end SHALL clear the hook, the hook owner and the power id and, when the condition word is exactly casting (0x20), set it to 0 and return the body to its stance, blended over 30 frames, in the same tick. A cast motion started before that stance return plays SHALL drop it. A live effect's focus shortfall, kinesis, throw and the power drain SHALL end a cast through it.

#### Scenario: A running torrent runs short
- **WHEN** a looping torrent's focus falls short while it runs
- **THEN** the condition becomes 0, the hook and power id are clear and the stance starts with a 30-frame blend in that tick

#### Scenario: A new cast before the stance
- **WHEN** a cast motion starts after a cast end and before its stance return plays
- **THEN** the stance return is dropped and the new cast motion plays

### Requirement: A replaced motion asks the hook
When a motion other than the walk, the run or an attack is replaced, including a one-shot cast clip ending into the stance, the hook SHALL be asked with event 4 and a negative answer SHALL drop it and the power id. A looping cast clip SHALL loop.

#### Scenario: A one-shot clip ends
- **WHEN** a shell's Force Raise clip ends into the stance with its hook set
- **THEN** the hook is asked with 4 and, answering −1, is dropped with the power id

### Requirement: Vanish let-go
A running vanish SHALL be let go by any hook call carrying a bit of 0x1b1, whatever it answers.

#### Scenario: Events once it holds
- **WHEN** a vanish that holds is asked with 0x110, then 0x80, then 4
- **THEN** 0x110 answers −1 and lets it go, 0x80 answers 0 and lets it go, and 4 answers 0 and keeps it

### Requirement: A later cast is a new hook
A live effect SHALL take its hook as called once a later cast has started, even when that cast has the same power id and hook.

#### Scenario: A blast recast while charging
- **WHEN** the blast is cast again while the first blast charges
- **THEN** the first blast fizzles

### Requirement: The release flag asks the hook
With a cast motion playing, a set release flag SHALL ask the hook with 0x10, end the cast whatever it answers, and clear.

#### Scenario: A shock released
- **WHEN** the release flag is set while a shock charges in its cast motion
- **THEN** the hook is asked with 0x10, the cast ends and the charge clears

### Requirement: The player's caster
The player's caster SHALL keep technique word 0 and one persistent slot. The slot count SHALL read only the record's six technique bytes, however far Concentration passes 30000.

#### Scenario: The player and a high Concentration
- **WHEN** the player's caster is made, and a duellist's record has 35000 Concentration points
- **THEN** the player's technique word is 0 with one slot, and the duellist's slots come from its six technique bytes alone

### Requirement: A hidden duellist casts nothing
A duellist out of the scene SHALL run its combat state, but the game SHALL NOT answer its slots, cast its chosen power or let its cast go until it is back in the scene (Exarust's own).

#### Scenario: A duellist out of range
- **WHEN** a duellist that knows the shock is out of the scene with a target 3 m ahead
- **THEN** its slots are not answered and no shock starts

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

### Requirement: Tree zoom about the cursor
Each drawn frame SHALL show the zoom `(value − previous)·share + previous` for the frame's share of a 17 ms step, SHALL rescale the pan's goal, position and previous point about the cursor by the shown zoom over the zoom shown the frame before, and SHALL show the pan blended from previous to position by the same share. Placement, the fixed zoom and the node hit tests SHALL use the shown zoom.

#### Scenario: Half a step
- **WHEN** the zoom spring is at 1.2 from 1.0, the last shown zoom is 1.0, the cursor is at (50, 60) and the pan's goal, position and previous point are (100, 200), (300, 400) and (200, 300), at half a step
- **THEN** the shown zoom is 1.1, the points become (105, 214), (325, 434) and (215, 324), and the shown pan is (270, 379)

### Requirement: Tree opening scale
While the window's or a domain's fade-in runs, the drawn nodes SHALL be scaled about the first node's placed point from before the frame's placement by `ease(s) = 1 + 0.111·s − 1/(8s + 1)`, `s` the running fade's `1 − r²`; otherwise the scale SHALL be one.

#### Scenario: Halfway through the opening
- **WHEN** the window fade-in is half run (`s = 0.75`)
- **THEN** the scale is 0.9403929

### Requirement: Tree first zoom level
The zoom level search SHALL compare the distance to the level just reached with the distance to the previous one and step back a level only when it is greater; below the first threshold the previous level is not a number, the compare is unordered and the first level (−8) SHALL answer.

#### Scenario: Below the first threshold
- **WHEN** the level is asked for 0.25, 0.26 and 0.29
- **THEN** each answers −8

### Requirement: Tree domain wheel child
The tree SHALL carry the domain wheel as a child at `(10, height − 290)`, 280 pixels square, with the opened domain as its choice, the focus share as its eased value (48 with no record) and the bar child's fade as its show. A click on one of its icons SHALL go through the wheel's confirm (known domain, different from the choice) and a confirmed domain SHALL start the tree's domain switch; a pick or drop SHALL start the wheel's 300 ms ease.

#### Scenario: Known and unknown domains
- **WHEN** a tree opened in Mind with a 700-pixel height is clicked at the Energy icon (150, 626) with no known domains, then at the Mind icon with all known, then at the Energy icon again
- **THEN** the first two clicks start no switch and the third chooses Energy and starts the 256 ms fade-out
