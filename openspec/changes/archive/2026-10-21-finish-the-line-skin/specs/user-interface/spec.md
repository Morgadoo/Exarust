## MODIFIED Requirements

### Requirement: Line rail
At unit 5 and place 10, 20 of size 100 by 80 the rail SHALL draw a left cap from 0, 85 to 30, 105, a right cap from 120, 85 to 90, 105, a middle from 30, 95 to 90, 105, and a thumb from -5, 30 to -15, 40, whose x does not follow the place. The cap sample SHALL bind twice, the middle once and the thumb once. Sixteen colour writes SHALL be counted. Bit 0x10 SHALL select the first ink, otherwise the second. Mode 0x10000 SHALL rebind the thumb in the third ink with alpha 255.

#### Scenario: The track and the thumb draw
- **WHEN** the rail is drawn at unit 5, place 10, 20, size 100 by 80, with a clear state and distinct inks
- **THEN** the four quads are those above, the four samples bind in order, and sixteen colour writes are counted

#### Scenario: The thumb ink follows the mode
- **WHEN** the state has mode 0x10000
- **THEN** the thumb uses the third ink with alpha 255
- **WHEN** the state has no mode
- **THEN** the thumb uses the track ink

#### Scenario: Bit 0x10 selects the first ink
- **WHEN** the state has bit 0x10
- **THEN** the track uses the first ink
- **WHEN** the state is 0
- **THEN** the track uses the second ink

### Requirement: Line hit test
The hit test SHALL use a span of four units, or six when the wide switch is set. The trim test SHALL run when flags masked with 0x30 equal 0x10 and y is at or above the span. The strip test SHALL run when control bit 0x200 is set and x is past the left limit, after a trim test that returned nothing as well as otherwise. The edge sums SHALL be 64-bit. The returned code SHALL be the original's unchanged.

#### Scenario: The trim test returns its codes
- **WHEN** the flags are 0x10, the control word has bit 2, the right limit is 100, the span is 20 and the point is 90, 10
- **THEN** the code is -4
- **WHEN** the point is 50, 10 with control bit 2 and no other bit
- **THEN** the code is 0
- **WHEN** the point is 10, 10 and control bit 1 is set
- **THEN** the code is -3

#### Scenario: The strip test returns its codes
- **WHEN** the control word has bit 0x200, x is past the left limit, and y is between the top and bottom limits
- **THEN** the code is -8
- **WHEN** y is at or past the bottom limit but below it plus the span
- **THEN** the code is -10
- **WHEN** y is at or below the top limit and above it minus the span
- **THEN** the code is -9

#### Scenario: A title-band miss falls through to the strip
- **WHEN** the flags are 0x10, the control word is 0x202, the top limit is 20 and the point 50, 15 misses the close corner
- **THEN** the code is -9

#### Scenario: Every other point is 0
- **WHEN** the flags and control word select no test
- **THEN** the code is 0

### Requirement: Close-edge highlight
With the state's tab bit 0x200 on a bar frame, close mark -9 SHALL tint the upper tab, -10 the lower tab and -8 the square below the scroll thumb, each at the quad the frame draws there, in 255, 204, 153, 255. The tabs SHALL keep the tab sample and the square SHALL bind the marker's sample.

#### Scenario: The marks draw their tints
- **WHEN** a held bar frame at unit 5 has its tab bit, a client right of 10, a client top of 20, a client bottom of 80, a thumb at 40, room to spare, and close mark -9
- **THEN** the tint is (16, 40), (16, 30), (26, 30), (26, 40)
- **WHEN** the mark is -10
- **THEN** the tint is (16, 60), (16, 70), (26, 70), (26, 60)
- **WHEN** the mark is -8
- **THEN** the tint is (11, 55), (11, 35), (31, 35), (31, 55) with the marker's sample

#### Scenario: Other marks draw nothing
- **WHEN** the close mark is -4, or the tab bit is clear
- **THEN** no close-edge tint is drawn

## ADDED Requirements

### Requirement: Line unit lengths
The line skin SHALL keep five unit lengths, 2, 1, 2, 2 and 4 units, set by a setter the parent constructor calls through the object's table before the line constructor rewrites the first four. A once-only flag SHALL register the unit.

#### Scenario: Unit 5
- **WHEN** the lengths are set at unit 5
- **THEN** they are 10, 5, 10, 10 and 20

### Requirement: Line centred icon
The line skin's centred icon SHALL be placed as the parent's is. A marked state SHALL bind no ink. Otherwise state bit 0x10 SHALL bind the label group's fourth shade as stored; with style bit 0x80000000 clear, state bit 0x40 SHALL bind the eighth shade and any other state the fourth, both with alpha 255; with it set, state bit 0x40 SHALL bind the seventh shade with its alpha raised by 0x40 in a byte, and any other state the sixth as stored. Style bit 0x40000000 SHALL pick the mirrored draw.

#### Scenario: The inks
- **WHEN** icon 16 is centred at 10, 20 in 100 by 80 with a clear state, then with state bit 0x40, then with style bit 0x80000000 and state bit 0x40
- **THEN** it sits at 52, 52 and binds the fourth shade opaque, then the eighth opaque, then the seventh raised

#### Scenario: The raised alpha wraps
- **WHEN** the seventh shade's alpha is 0xd0
- **THEN** the bound alpha is 0x10

### Requirement: Line scroll refresh
The scroll refresh SHALL take the edge mask 0xf without style bit 4, and otherwise set bit 1 for a top above the unit, bit 4 for a right edge within a unit of the owner's room and bit 8 for the bottom, never bit 2. The rail SHALL start five units below the client top with bits 1 and 4, else four, end five or four units above the client bottom with bits 4 and 8, and place the thumb at the start plus the vertical fraction of the span less two units, rounded half to even.

#### Scenario: A NaN fraction
- **WHEN** the fraction is NaN
- **THEN** the thumb is at the start

#### Scenario: Room to spare
- **WHEN** a window at 10, 20 of 100 by 80 with client top 30 and bottom 90 in a 400 by 300 owner refreshes at unit 5 with fraction one half
- **THEN** the mask is 0, the rail runs from 50 to 70, and the thumb is 55

#### Scenario: A window at its owner's corner
- **WHEN** the window is at 2, 2 in a 100 by 80 owner
- **THEN** the mask is 0xd and the rail runs from 55 to 65

### Requirement: Line progress strip
A titled window without state bit 0x800000, and an untitled open bar, SHALL draw a strip between the skin's first and third side lengths: a cap at each end eight units in and the track between them. A titled window SHALL split the track at the title's gap, half of the width less the title width and five units, with a cap on the inner side of each piece.

#### Scenario: A titled window
- **WHEN** a 100-wide window at unit 5 has a 40-wide title
- **THEN** the strip has six quads and its first track piece runs from 40 back to 17

#### Scenario: An open bar
- **WHEN** an untitled open bar 100 wide is drawn at unit 5
- **THEN** the strip has three quads and the track runs from 40, 10 to 60, 20

### Requirement: Maximised window frame
A window style with the maximised state bit SHALL draw its own frame: the top, left and right bars, the bottom bar and four corner arms reaching two units outside and six in, with side bars four units from the corners, or two with style bit 4. Style bit 4 SHALL also bind the hot ink on style bit 0x40, else the normal ink, and add an outer top bar unless the window sits within a unit of a focused owner's top.

#### Scenario: A plain maximised window
- **WHEN** a 100 by 80 maximised window without style bit 4 is drawn at unit 5
- **THEN** eight quads are drawn, the top bar from 30, 0 to 70, 10

#### Scenario: A held maximised window
- **WHEN** style bits 0x44 are set and the owner is not focused
- **THEN** nine quads are drawn in the hot ink

### Requirement: Integer span bar
The compact skill screen's span bar SHALL draw the capped span from integer arguments with no ink bind: caps from two units outside to four units inside each end, four units tall from the top, and the middle from two units down.

#### Scenario: Unit 5
- **WHEN** the bar is drawn at 10, 20 with width 100
- **THEN** the caps run from 0, 20 to 30, 40 and from 120, 20 to 90, 40 around a middle from 30, 30 to 90, 40
