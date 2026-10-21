# user-interface Specification

## Purpose
Shows the game's on-screen interface in the original's look, starting with
its bitmap font and the frame-rate readout.

## Requirements

### Requirement: Game font
The system SHALL draw interface text with the game's base font atlas (`fontbase24r`), read from the installation at run time: a 16 × 16 Latin-1 grid with glyphs stored upside down in their cells.

#### Scenario: Missing font
- **WHEN** the installation has no base font
- **THEN** the interface text is left out and the app keeps running

### Requirement: Frame-rate readout
The system SHALL show the frame rate in the window's top-right corner as the original does ("57.25 FPS"): the mean rate over each half second, two decimals, in the game's font at its 24-point size in physical pixels. A command-line option hides it.

#### Scenario: Running a level
- **WHEN** a level is shown
- **THEN** the top-right corner reads the current frame rate followed by "FPS"

### Requirement: Loading screen
The system SHALL show a loading screen while a level loads: a dark page with "Loading <level> - <stage>" in the game's base font and a progress bar. It SHALL go when the load ends, and the frame-rate readout SHALL show over the game's view after it.

#### Scenario: Story level
- **WHEN** `exarust play exanima01` starts
- **THEN** the loading screen names each stage as it runs (walls and floors, terrain, props, collision, dressing characters, lighting) until the level shows

#### Scenario: Missing font
- **WHEN** the installation has no base font
- **THEN** the loading screen shows its progress bar without text

### Requirement: Long-frame log
The system SHALL time every frame of a load and of the 10 s after it (or after the start, when nothing loads), log a frame longer than 250 ms as a warning with the stage it was spent in, and log the load's length and its longest frame when it ends.

#### Scenario: Load summary
- **WHEN** a level has loaded
- **THEN** the log states how long the load took, over how many frames, and its longest frame with its stage

### Requirement: Interface scale
The system SHALL size the interface by the original's GUI scale: 3 for a window under 700 physical pixels tall, then 4, 5, 6, 8 and 12 from 700, 870, 1051, 1390 and 2080 pixels, the step offset by a GUI size setting that is 0 by default, and follow the window when it changes size.

#### Scenario: Default window
- **WHEN** play mode runs in a 1600 × 900 window at scale factor 1
- **THEN** the GUI scale is 5

### Requirement: World cursor
The system SHALL draw the world cursor in play mode at the pointer's position, in place of the system pointer: a frame of the installation's `cursors` atlas (2 × 4 frames) round(3 × GUI scale) × 2 physical pixels square (at most 128) over four black copies one pixel aside, red (255, 76, 76, alpha 180) in combat mode and amber (255, 205, 76) outside it. It SHALL hide the system pointer only while drawn and never move it.

#### Scenario: Combat mode
- **WHEN** Tab turns combat mode on in a 900-pixel-tall window with the pointer by the player's feet
- **THEN** a red ring 30 pixels across shows at the pointer, and the system pointer is hidden

#### Scenario: Pointer outside the window
- **WHEN** the pointer leaves the window
- **THEN** no world cursor is drawn

#### Scenario: Missing atlas
- **WHEN** the installation has no `cursors` atlas
- **THEN** the system pointer stays and the app keeps running

### Requirement: World cursor shapes
The system SHALL pick the cursor's shape as the original does. In combat mode: chevrons while walking to the cursor (two beyond 300 cm), else the arrowhead while Ctrl is held, else the ring, also down or collapsing; the arrowhead once dead. Outside it: the cross while down, collapsing or dead, chevrons while walking to the cursor, else the arrowhead over a point within the hand's reach and the cross beyond. Ring and chevrons SHALL turn away from the player.

#### Scenario: Running to the cursor
- **WHEN** the right button is held over a point 5 m from the player outside combat mode
- **THEN** two amber chevrons show at the pointer, pointing away from the player

#### Scenario: Death in combat mode
- **WHEN** the duellist kills the player in combat mode
- **THEN** the cursor becomes a red arrowhead tilted as a system pointer, its tip at the pointer

#### Scenario: Between fights
- **WHEN** combat mode is off and the pointer is over the floor a metre from the player
- **THEN** the cursor is an amber arrowhead, and over ground 5 m away an amber cross

### Requirement: Health bars
In play mode the system SHALL draw the player's health and focus bars as the original: two framed bars 48 × GUI scale wide, 52 × scale apart, from 50 × scale left of the centre, 5 × scale above the bottom; health yellow to the yellow share, brown to the red share, red beyond; focus blue to the focus, grey to its capacity, purple beyond. Exarust's focus and capacity SHALL be 1.

#### Scenario: Wounded player
- **WHEN** the player has yellow health 0.176 and red 0.226 of 0.25 in combat mode in a 1600 × 1014 window
- **THEN** the health bar fills x 550–789 yellow to 718, brown to 766 and red beyond, and the focus bar x 810–1049 blue

#### Scenario: Full health
- **WHEN** the player is unhurt in combat mode
- **THEN** both bars are full, yellow and blue

### Requirement: Health bar fade
The system SHALL show the bars while combat mode is on and in a frame where the yellow health changes or the focus drops, fading in over 500 ms, then holding 200 ms and fading out over 2 s (game time); fills SHALL blend at the fade squared, frames and shadows at the fade. With a shield in combat mode the block-stamina gauge SHALL show above them by the same rule.

#### Scenario: Combat mode on and off
- **WHEN** Tab turns combat mode on and, later, off
- **THEN** the bars appear over half a second and, after combat mode ends, stay 200 ms and fade out over 2 s

#### Scenario: Exploring
- **WHEN** the player walks outside combat mode, unhurt
- **THEN** no bars are drawn

#### Scenario: Healing
- **WHEN** the player, wounded, turns combat mode off and its yellow health regenerates
- **THEN** the bars show while the yellow segment refills over the brown, and fade out (200 ms, then 2 s) once it reaches the red health

### Requirement: Low-health heartbeat
In story levels the system SHALL start the heartbeat when a blow leaves the player's yellow health below 0.05 without killing it (the damage's low-health check), at most once a minute: for 7 s a dark red (80, 0, 0) border from the window's edges to a fifth of its height in, pulsing to alpha 96 every 800 ms, with the `heartbeat` sound looping at gain 0.3. Arena levels SHALL have no heartbeat.

#### Scenario: Low blow in a story level
- **WHEN** a blow leaves the player's yellow health at 0.03 in a story level
- **THEN** a pulsing dark red border and a heartbeat play for 7 s

#### Scenario: Arena
- **WHEN** the same blow lands in the arena
- **THEN** no border shows

#### Scenario: Already low
- **WHEN** a blow that deals nothing lands on the player at yellow 0.04 in a story level, a minute after the last heartbeat
- **THEN** the heartbeat starts again

### Requirement: Interface compositing
The system SHALL lay the interface over the game's view so that a translucent interface pixel blends once by its opacity: the interface camera's texture, drawn over transparent black, SHALL be composited premultiplied.

#### Scenario: Half-transparent white over the floor
- **WHEN** an interface pixel of white at opacity 0.28 lies over the arena floor (about 42 of 255)
- **THEN** the window shows it at about 147 of 255, not 87

### Requirement: Death fade
When the player falling to its death hears the pit, the system SHALL cover the game's view with black, over the heartbeat's border and under the health bars, its opacity of 255 the share of 3 s of game time gone, rounded half to even, and full after; hidden while a level loads.

#### Scenario: Three seconds
- **WHEN** the fade has run 0, 12, 1,500 and 3,000 ms
- **THEN** its opacity is 0, 1, 128 and 255 of 255

#### Scenario: Paused mid-fade
- **WHEN** `exarust --at 0,200,0 --pause-at-tick 300 play arena` pauses 1.5 s into the fade
- **THEN** the falling body is drawn at about half its brightness, against the same body unfaded at tick 150

### Requirement: Draw skin
Opening a skin with a unit SHALL store twice that unit, the unit, and twice the unit twice more. It SHALL store the pads 8, 4, 8 and 8. Its three colours SHALL each be 255, 250, 230, 100. Its mark SHALL be 1. Argument 1 SHALL count an allocation, and a failed allocation SHALL return nothing. A non-zero argument SHALL count the call after the fill. Argument 0 SHALL not. The base fill and that later call are counts, not their bodies.

#### Scenario: Unit 5 stores the spans and the colour
- **WHEN** a skin is opened with unit 5 and argument 0, and again with argument 1 after a successful allocation, and again with argument 1 after a failed allocation
- **THEN** the first stores spans 10, 5, 10 and 10 and the colour three times without the later call, the second counts the allocation and the later call, and the third returns nothing

### Requirement: Flat slider
A flat slider whose high mode bits are clear SHALL draw four quads. Both caps SHALL use 0.2, 0.8, 0.2, 1. The track SHALL use 0.25, 0.8, 0.25, 1. The thumb SHALL use 0.4, 0.8, 0.4, 1 and sit at the unit plus the spare width times the value, ties rounded even. State bit 0x10 selects ink 0, bit 0x200000 selects ink 1, and any other state selects ink 2. State bit 0x10000 counts a mark. Any other mode selects the ink and draws no flat quads.

#### Scenario: Unit 5 places the thumb
- **WHEN** unit 5 draws a slider at 10, 20 of width 100 with value 0, then with value one half, then with state bits 0x10 and 0x10000, and a scroll mode is drawn
- **THEN** the first thumb is 5 with pad 10 and ink 2, the half value sets the thumb to 40, the marked state selects ink 0, and the scroll mode draws no flat quads

### Requirement: Scroll bar
A scroll bar whose high mode bits are 0x10000000 SHALL draw a trough and twelve quads. The trough starts one past the unit and is the size minus twice the unit, then minus 2. End caps SHALL use 0.7, 0.1, 0.7, 0.2. Bars between them SHALL use 0.3, 0.1, 0.3, 0.2. The thumb sits at x plus the spare width times the value, ties rounded even, and a second thumb sits three units on. State bit 0x10000 counts a mark. State bit 0x200000 selects the bar ink. Any other high mode draws neither.

#### Scenario: Unit 5 places twelve quads
- **WHEN** unit 5 draws a scroll bar at 10, 20, size 100 by 80, lip 4, value 0, and again at value one half
- **THEN** the trough is 16, 26, 88, 68, the first thumb is 10 with a second thumb at 25, and the half value sets the thumb to 52

### Requirement: Frame
A frame SHALL draw four corner squares of three times the unit and two edge bars one unit thick. Corners SHALL use 0.4, 0.3, 0.4, 0.45. Edges SHALL use 0.475, 0.3, 0.475, 0.35. Side bars SHALL appear only when four times the unit is below the height. State bit 0x10 selects ink 0. A clear mark and a clear wide bit select ink 2. Any other clear hot mask selects ink 0, and a set hot mask selects ink 3.

#### Scenario: Unit 5 places the corners
- **WHEN** unit 5 draws a frame at 10, 20 of size 100 by 20, and again at height 80 with state bit 0x20
- **THEN** the first stores six quads and ink 2, and the second stores eight quads and ink 3

### Requirement: Centred icon
A centred icon SHALL sit at the origin plus half the spare, dividing a negative spare toward zero. State bit 0x10000 marks it and passes the scale bits of 1. Inside that mark, bits 0x60 equal to 0x40 add one to the height. A wide bit selects the wide draw, and an own list is kept. Otherwise bit 0x10 selects ink 0, a clear high bit with bit 0x40 selects ink 1, a clear high bit selects ink 2, a set high bit with bit 0x40 selects ink 3, and a set high bit selects ink 4. The glyph call is not in.

#### Scenario: Icon 16 sits at 52, 52
- **WHEN** icon 16 is centred in a widget at 10, 20 of size 100 by 80, and again with state bits 0x10000 and 0x40
- **THEN** the first sits at 52, 52 with ink 2, and the second sits at 52, 53 and passes the scale bits of 1

### Requirement: Form dispatch
A dispatch SHALL store twice the unit, three times the unit, and four times the unit. Bit 0x40 counts a shade. Bit 0x20000 with bit 4 clear draws a quad from the origin to the size. Bit 0x800 leaves the shift at 0. Otherwise the shift is the spare times the value, ties rounded even, and the origin subtracts it. Bit 0x100 skips the shift call. Shown children are visited up to the count. The rest of the dispatcher is not in.

#### Scenario: Unit 5 shifts by the spare
- **WHEN** unit 5 dispatches a form of size 100 by 80 with bit 0x40 and bit 0x20000, a spare of 30 at value 1, and children marked 0, 1, 0, 1
- **THEN** the spans are 10, 15 and 20, the quad reaches 100, 80, the shift is 30, and the shown children are 1 and 3

### Requirement: Title and corners
After the children, a positive title measure SHALL be centred in the width, toward zero. A tint byte of 200 SHALL stay 200. Style bits equal to 0x10 SHALL draw four corners of six times the unit. Bit 2 SHALL add a four-unit square unless the mark is -4, and that top corner SHALL use 0, 0.4, 0, 0.7. Bit 0x400000 SHALL count a wide call and draw no corners. The later edge bars are not in.

#### Scenario: A title of 40 sits at 30
- **WHEN** unit 5 dispatches a form of width 100 and height 80 with style bits 0x10 and a title measure of 40
- **THEN** the title sits at 30 and the four corners have span 30

### Requirement: Edge bars
After the four corners, the same style SHALL draw edge bars. With no title, unit 5 on a form of 100 by 80 SHALL draw a top bar from 33, 0 to 67, 10. A title of width 40 SHALL split that bar at gap 17 and add two caps. A style that is not 0x10, and bit 0x400000, SHALL draw no edges. The drawing after those bars is not in.

#### Scenario: A top bar runs from 33 to 67
- **WHEN** unit 5 dispatches a form of width 100 and height 80 with style bits 0x10 and no title
- **THEN** the top bar runs from 33, 0 to 67, 10, and a title of width 40 splits the top at gap 17

### Requirement: Wide top
A wide form whose style word has bits 0xc0 equal to 0x80 SHALL draw three top quads. At unit 5 and width 100 the left cap SHALL run from 0, -5 to 10, 5, the bar from 10, -5 to 90, 5, and the right cap from 90, 5 to 100, -5. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The side bars after that top are not in.

#### Scenario: The bar runs from 10 to 90
- **WHEN** unit 5 dispatches a wide form of width 100 whose style word is 0x80
- **THEN** the bar runs from 10, -5 to 90, 5 and the two caps meet it

### Requirement: Wide side
The same wide form SHALL then draw a left side. At unit 5 and height 80 the bar SHALL run from -5, 10 to 5, 70. A cap SHALL run from y 0 to 10, and a cap from y 70 to 80. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The bottom after that side is not in.

#### Scenario: The side runs from 10 to 70
- **WHEN** unit 5 dispatches a wide form of height 80 whose style word is 0x80
- **THEN** the side bar runs from -5, 10 to 5, 70 and the two caps meet it

### Requirement: Wide bottom
The same wide form SHALL then draw a bottom. At unit 5, width 100 and height 80 the bar SHALL run from 10, 75 to 90, 85. A cap SHALL run from 0, 75 to 10, 85, and a cap from 90, 75 to 100, 85. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The right side is not in.

#### Scenario: The bottom runs from 10 to 90
- **WHEN** unit 5 dispatches a wide form of width 100 and height 80 whose style word is 0x80
- **THEN** the bottom bar runs from 10, 75 to 90, 85 and the two caps meet it

### Requirement: Wide right
The same wide form SHALL then draw a right side. At unit 5, width 100 and height 80 the bar SHALL run from 95, 10 to 105, 70. A cap SHALL run from y 0 to 10, and a cap from y 70 to 80. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The mark after that side is not in.

#### Scenario: The right side runs from 10 to 70
- **WHEN** unit 5 dispatches a wide form of width 100 and height 80 whose style word is 0x80
- **THEN** the right bar runs from 95, 10 to 105, 70 and the two caps meet it

### Requirement: Ink mark
On a wide form whose style word has bits 0xc0 equal to 0x80, bit 0x40 of the first flag word SHALL count one ink copy. A clear bit SHALL count none. A form that is not wide, and style 0x10 without those bits, SHALL count none. The join after that copy is not in.

#### Scenario: Bit 0x40 counts one copy
- **WHEN** unit 5 dispatches a wide form whose style word is 0x80 and whose first flag word has bit 0x40 set
- **THEN** the ink copy is counted once, and a clear bit counts none

### Requirement: Late close
A close mark of -4 SHALL draw one square after the other quads. At unit 5 and width 100 the square SHALL run from 100, 0 to 80, 20. A mark of 0 or -3 SHALL draw none. The hold-bit edge test is not in.

#### Scenario: A mark of -4 draws the square
- **WHEN** unit 5 dispatches a form of width 100 whose close mark is -4
- **THEN** one square runs from 100, 0 to 80, 20, and a mark of 0 draws none

### Requirement: Held edges
With the hold bit, style bits 0xc0 equal to 0x80, and no wide bit, a y below the unit SHALL set mask bit 1 and an x below the unit SHALL set bit 2. A right sum past the room SHALL set bit 4, and a bottom sum past the room SHALL set bit 8. An equal sum SHALL set neither. A clear hold bit, a wide bit, and style 0x10 without those bits SHALL set none. The quads after that mask are not in.

#### Scenario: A y of 4 sets bit 1
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, x 4 and y 4, and a right room of 100
- **THEN** the mask is 7, and an equal sum leaves bits 4 and 8 clear

### Requirement: Held bar
The same held form SHALL then draw one bar. At unit 5 and width 100 the bar SHALL run from 30, 0 to 70, 10. A y below the unit with owner bit 1 and without bit 0x800000 SHALL draw none. Owner bit 0x800000 SHALL draw the bar. A clear hold bit and a wide bit SHALL draw none. The drawing after that bar is not in.

#### Scenario: The bar runs from 30 to 70
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80 and a y of 5
- **THEN** the bar runs from 30, 0 to 70, 10

### Requirement: Held tab
Bit 0x200 on a held form that is not wide SHALL draw one tab. A grip of 10, 20 at unit 5 SHALL sit from 16, 30 to 26, 40. Mask bits 1 and 4 together SHALL raise it to 35. A clear bit 0x200 and a clear hold bit SHALL draw none. The next anchor after that tab is not in.

#### Scenario: A grip of 10, 20 sits at 16, 30
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, bit 0x200, and a grip of 10, 20
- **THEN** the tab runs from 16, 30 to 26, 40

### Requirement: Lower tab
The same bit SHALL then draw a lower tab from the seat. A seat of 80 at unit 5 SHALL sit from 16, 60 to 26, 70. Mask bits 4 and 8 together SHALL raise it so the top is 65. A clear bit 0x200 and a clear hold bit SHALL draw none. The anchor after that lower tab is not in.

#### Scenario: A seat of 80 sits at 16, 60
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, bit 0x200, a grip of 10, and a seat of 80
- **THEN** the lower tab runs from 16, 60 to 26, 70

### Requirement: Post quad
The same bit SHALL then draw one quad from the grip and a post. A grip of 10 and a post of 40 at unit 5 SHALL run from 11, 35 to 31, 55. A clear bit 0x200 and a clear hold bit SHALL draw none. The compare after that quad is not in.

#### Scenario: A post of 40 sits at 11, 35
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, bit 0x200, a grip of 10, and a post of 40
- **THEN** the quad runs from 11, 35 to 31, 55

### Requirement: Held left
A held form that is not wide SHALL then draw a left strip. At unit 5 and height 80 it SHALL run from 0, 30 to 10, 50. A clear bit 0x200 SHALL still draw it. A clear hold bit and a wide bit SHALL draw none. The mask compare after that strip is not in.

#### Scenario: Height 80 sits at 0, 30
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80 and bit 0x200 clear
- **THEN** the strip runs from 0, 30 to 10, 50

### Requirement: Held cap
Mask bits 1 and 2 together SHALL then draw a cap under that strip. At unit 5 it SHALL run from 0, 10 to 10, 30. An edge of 5, 5 SHALL draw none. A clear hold bit SHALL draw none. The flag test after that cap is not in.

#### Scenario: An edge of 4 draws the cap
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80 and an edge of 4, 4
- **THEN** the cap runs from 0, 10 to 10, 30

### Requirement: Held ledge
Mask bits 1 and 2 together SHALL then draw a ledge. At unit 5 it SHALL run from 10, 0 to 30, 10. Bit 1 of the second flag word, or owner bit 1, SHALL draw none unless the owner also has bit 0x800000. An edge of 5, 5 and a clear hold bit SHALL draw none. The sign flip after that ledge is not in.

#### Scenario: An edge of 4 draws the ledge
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80 and an edge of 4, 4
- **THEN** the ledge runs from 10, 0 to 30, 10

### Requirement: Open frame
With the band bits and no wide bit, a clear hold bit SHALL force mask 15 and draw the wider strip. At unit 5 and height 100 the strip SHALL run from 10, 40 to 20, 60, the cap from 10, 20 to 20, 40, and the ledge from 20, 10 to 40, 20. A clear bit 1 SHALL count one call, and a set bit SHALL count none. The held mask SHALL stay 0. The helper body is not in.

#### Scenario: Height 100 sits at 10, 40
- **WHEN** unit 5 dispatches a form of height 100 that is not wide, with style bits 0x80 and the hold bit clear
- **THEN** the strip runs from 10, 40 to 20, 60 and the held mask is 0

### Requirement: Sign flip
When the edge bits are not both set, a held form SHALL draw one quad. At unit 5 it SHALL run from -10, -10 to 30, 30. An edge of 4, 4 SHALL draw none, and the cap SHALL remain. A clear hold bit SHALL draw none. The next mask test is not in.

#### Scenario: An edge of 5 sits at -10, -10
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80 and an edge of 5, 5
- **THEN** the quad runs from -10, -10 to 30, 30

### Requirement: Held end
Mask bits 1 and 4 together, with bit 0x200, SHALL draw one cap. At unit 5 and width 100 it SHALL run from 70, 10 to 100, 30. A clear bit 0x200 SHALL draw none. An edge that misses either bit, and a clear hold bit, SHALL draw none. The quad after a clear tab bit is not in.

#### Scenario: Width 100 sits at 70, 10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200, an edge of 4, 4, and a right room of 100
- **THEN** the cap runs from 70, 10 to 100, 30

### Requirement: Plain cap
Mask bits 1 and 4 together, with bit 0x200 clear, SHALL draw one narrower cap. At unit 5 and width 100 it SHALL run from 90, 10 to 100, 30. A set bit 0x200 SHALL draw the wider cap instead. An edge that misses either bit, and a clear hold bit, SHALL draw none. The flag test after that cap is not in.

#### Scenario: A clear tab bit sits at 90, 10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200 clear, an edge of 4, 4, and a right room of 100
- **THEN** the cap runs from 90, 10 to 100, 30

### Requirement: Held shelf
The same clear tab bit SHALL then draw a shelf. At unit 5 and width 100 it SHALL run from 70, 0 to 90, 10. Bit 1, or owner bit 1, SHALL draw none unless the owner also has bit 0x800000. A set bit 0x200, an edge that misses either mask bit, and a clear hold bit SHALL draw none. The quad after that shelf is not in.

#### Scenario: Width 100 sits at 70, 0
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200 clear, an edge of 4, 4, and a right room of 100
- **THEN** the shelf runs from 70, 0 to 90, 10

### Requirement: Held over
When mask bits 1 and 4 are not both set, bit 0x200 SHALL draw one quad. At unit 5 and width 100 it SHALL run from 70, -10 to 110, 30. A clear bit 0x200 SHALL draw none. An edge of 4, 4 with a right room of 100 SHALL draw the wider cap instead. A clear hold bit SHALL draw none. The quad after a clear tab bit on that path is not in.

#### Scenario: An edge of 5 sits at 70, -10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200, and an edge of 5, 5
- **THEN** the quad runs from 70, -10 to 110, 30

### Requirement: Held bare
When mask bits 1 and 4 are not both set, a clear bit 0x200 SHALL draw one quad. At unit 5 and width 100 it SHALL run from 70, -10 to 110, 30. A set bit 0x200 SHALL draw the other sample instead. An edge of 4, 4 with a right room of 100 SHALL draw none of either. A clear hold bit SHALL draw none. The drawing after that join is not in.

#### Scenario: An edge of 5 sits at 70, -10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200 clear, and an edge of 5, 5
- **THEN** the quad runs from 70, -10 to 110, 30

### Requirement: Held right
Bit 0x200 SHALL then draw one strip. At unit 5, width 100 and height 80 it SHALL run from 80, 30 to 90, 50. A clear bit 0x200 SHALL draw none. A clear hold bit with that bit set SHALL draw the wider strip, from 70, 40 to 80, 60 at height 100. The drawing after that strip is not in.

#### Scenario: Width 100 sits at 80, 30
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80 and bit 0x200
- **THEN** the strip runs from 80, 30 to 90, 50

### Requirement: Held beam
With the tab bit set and held-edge mask bit 4 clear, a clear bit 0x200 of the plain word SHALL draw one bar. At unit 5, width 100 and height 80 it SHALL run from 96, 20 to 106, 60. That plain bit SHALL draw none, and the right strip SHALL remain. Mask bit 4 SHALL draw none. A clear tab bit and a clear hold bit SHALL draw none. The drawing after that bar is not in.

#### Scenario: Width 100 sits at 96, 20
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit, an edge of 5, 5, and the plain bit 0x200 clear
- **THEN** the bar runs from 96, 20 to 106, 60

### Requirement: Held caps
The same clear plain bit SHALL then draw two caps. At unit 5, width 100 and height 80 the upper cap SHALL run from 96, 0 to 106, 20 and the lower cap SHALL run from 96, 60 to 106, 80. That plain bit, mask bit 4, a clear tab bit, and a clear hold bit SHALL draw none. The drawing on the clear tab path is not in.

#### Scenario: Width 100 sits at 96, 0
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit, an edge of 5, 5, and the plain bit 0x200 clear
- **THEN** the upper cap runs from 96, 0 to 106, 20 and the lower cap runs from 96, 60 to 106, 80

### Requirement: Held trim
A clear tab bit SHALL then draw one strip. At unit 5, width 100 and height 80 it SHALL run from 90, 30 to 100, 50. A set tab bit SHALL draw none. A clear hold bit with the tab bit clear SHALL draw the wider strip, from 80, 40 to 90, 60 at height 100. The next mask test is not in.

#### Scenario: Width 100 sits at 90, 30
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80 and the tab bit clear
- **THEN** the strip runs from 90, 30 to 100, 50

### Requirement: Held low
Mask bits 2 and 8 together SHALL then draw one strip. At unit 5 and height 80 it SHALL run from 0, 50 to 10, 70. A missing bit SHALL draw none. A set tab bit SHALL still draw it. A clear hold bit SHALL draw the wider strip, from 10, 60 to 20, 80 at height 100. The next quad is not in.

#### Scenario: Height 80 sits at 0, 50
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80, an edge of 4, 4, and a bottom room of 80
- **THEN** the strip runs from 0, 50 to 10, 70

### Requirement: Held sole
The same bits SHALL then draw one ledge. At unit 5 and height 80 it SHALL run from 10, 70 to 30, 80. A missing bit SHALL draw none. A set tab bit SHALL still draw it. A clear hold bit SHALL draw the wider ledge, from 20, 80 to 40, 90 at height 100. The other arm is not in.

#### Scenario: Height 80 sits at 10, 70
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80, an edge of 4, 4, and a bottom room of 80
- **THEN** the ledge runs from 10, 70 to 30, 80

### Requirement: Held skirt
When mask bits 2 and 8 are not both set, a held form SHALL draw one quad. At unit 5 and height 80 it SHALL run from -10, 50 to 30, 90. Both bits set SHALL draw the ledge instead. A clear hold bit SHALL draw none. The next quad is not in.

#### Scenario: A tall room sits at -10, 50
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80, an edge of 4, 4, and a bottom room of 400
- **THEN** the quad runs from -10, 50 to 30, 90

### Requirement: Held floor
A form on that band SHALL then draw one bar. At unit 5, width 100 and height 80 it SHALL run from 30, 70 to 70, 80. A set tab bit SHALL still draw it. A clear hold bit SHALL draw the wider bar, from 40, 80 to 60, 90 at height 100. A style that is not that band SHALL draw none. The next mask test is not in.

#### Scenario: Width 100 sits at 30, 70
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80
- **THEN** the bar runs from 30, 70 to 70, 80

### Requirement: Held rear
Mask bits 4 and 8 together, with the tab bit, SHALL draw one strip. At unit 5, width 100 and height 80 it SHALL run from 80, 50 to 100, 70. A missing bit SHALL draw none. A clear tab bit SHALL draw none. A clear hold bit with the tab bit SHALL draw the wider strip, from 70, 60 to 90, 80 at height 100. The clear-tab arm is not in.

#### Scenario: Width 100 sits at 80, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit, an edge of 4, 4, a right room of 100, and a bottom room of 80
- **THEN** the strip runs from 80, 50 to 100, 70

### Requirement: Held lip
The same bits with a clear tab bit SHALL draw one narrower strip. At unit 5, width 100 and height 80 it SHALL run from 90, 50 to 100, 70. A set tab bit SHALL draw the wider strip instead. A missing bit SHALL draw none. A clear hold bit with a clear tab bit SHALL draw the wider clear strip, from 80, 60 to 90, 80 at height 100. The next quad is not in.

#### Scenario: Width 100 sits at 90, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit clear, an edge of 4, 4, a right room of 100, and a bottom room of 80
- **THEN** the strip runs from 90, 50 to 100, 70

### Requirement: Held sill
Both tab arms then draw one quad. At unit 5, width 100 and height 80 it SHALL run from 70, 70 to 90, 80. A set tab bit still draws it, and the wider strip remains. A missing bit SHALL draw none. A clear hold bit SHALL draw the wider quad, from 60, 80 to 80, 90 at height 100. The other arm is not in.

#### Scenario: Width 100 sits at 70, 70
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit clear, an edge of 4, 4, a right room of 100, and a bottom room of 80
- **THEN** the quad runs from 70, 70 to 90, 80

### Requirement: Held drape
When mask bits 4 and 8 are not both set, the tab bit SHALL draw one quad. At unit 5, width 100 and height 80 it SHALL run from 70, 50 to 110, 90. Both bits set SHALL draw the bottom quad instead. A clear tab bit SHALL draw none. A clear hold bit SHALL draw none. The clear-tab arm is not in.

#### Scenario: Width 100 sits at 70, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit set, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the quad runs from 70, 50 to 110, 90

### Requirement: Held fold
When mask bits 4 and 8 are not both set, a clear tab bit SHALL draw one quad. At unit 5, width 100 and height 80 it SHALL run from 70, 50 to 110, 90. A set tab bit SHALL draw the other sample instead. Both bits set SHALL draw none. A clear hold bit SHALL draw none. The join after that quad is not in.

#### Scenario: A clear tab sits at 70, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit clear, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the quad runs from 70, 50 to 110, 90

### Requirement: Band close
On that band, a close mark of -4 SHALL draw one square. At unit 5 and width 100 it SHALL run from 80, 0 to 100, 20. The earlier square remains. A mark of 0 or -3 SHALL draw none of this square. A style that is not that band SHALL draw none of it. A wide bit SHALL draw none of it. The call after that square is not in.

#### Scenario: Width 100 sits at 80, 0
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, a close mark of -4, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the square runs from 80, 0 to 100, 20

### Requirement: Band call
On that band the dispatcher SHALL count one call with argument 0. A close mark of -4 still counts one, and the square remains. A style that is not that band SHALL count none. A wide bit SHALL count none. The bytes the jump skips are not in.

#### Scenario: A held band counts one
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, a close mark of 0, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the call count is 1

### Requirement: Frame return
Every dispatch SHALL pass the frame to one call, restore the saved register, then set the stack to the frame and pop the frame. A held band, a wide form, and a style that is not that band each SHALL do so. The helper body is not in.

#### Scenario: A plain form restores the frame
- **WHEN** unit 5 dispatches a form that is not wide, with no style bits and a close mark of 0
- **THEN** the frame call count is 1 and the frame is restored

### Requirement: Frame text
The frame helper SHALL release one string slot. That slot SHALL start empty. An empty slot SHALL not be freed. A held band, a wide form, and a style that is not that band each SHALL release it once. The bytes the jump skips SHALL not run.

#### Scenario: A plain form releases an empty slot
- **WHEN** unit 5 dispatches a form that is not wide, with no style bits and a close mark of 0
- **THEN** the string slot is empty, it is released once, and it is not freed

### Requirement: Span bar
A span bar SHALL draw three quads and count one preparation. Stored word 4 SHALL double to thickness 8. At unit 5, origin 10, 20 and span 100, the middle SHALL be (15, 20), (15, 28), (105, 28), (105, 20). The left cap SHALL be (15, 20), (15, 28), (10, 28), (10, 20). The right cap SHALL be (105, 20), (105, 28), (110, 28), (110, 20). The middle sample SHALL bind once and the cap sample twice. Stored word -3 SHALL use thickness -6. The view scale and the colour word are not in.

#### Scenario: Unit 5 draws the three quads
- **WHEN** unit 5 draws a span from 10, 20 with span 100 and stored word 4
- **THEN** one preparation is counted, the middle sample is bound once, the cap sample is bound twice, and the three quads have the corners above

#### Scenario: A negative stored word doubles
- **WHEN** unit 5 draws a span from 10, 20 with span 100 and stored word -3
- **THEN** the thickness is -6

### Requirement: Span view
Each span-bar corner SHALL be placed as x plus the shift, times the scale, and as the shift minus y, times the other scale. All twelve corners SHALL store the colour word. With shifts 2 and 100, scales one half and 2, and colour 17, the first corner SHALL place at 8.5, 160. An unset view SHALL place that corner at positive zero and negative zero, and store colour 0. The raw quads SHALL stay. The mark written when bit 4 is set is not in.

#### Scenario: A shifted view places the first corner
- **WHEN** unit 5 draws a span from 10, 20 with span 100, stored word 4, shifts 2 and 100, scales one half and 2, and colour 17
- **THEN** the first raw corner stays 15, 20, the placed corner is 8.5, 160, and colour 17 is stored twelve times

#### Scenario: An unset view stores zeros
- **WHEN** the same span is drawn with shifts 0 and 0, scales 0 and 0, and colour 0
- **THEN** the first placed corner is positive zero and negative zero, and colour 0 is stored twelve times

### Requirement: Span mark
Bit 4 of the flag SHALL store the mark word once per corner, at the cursor, and the cursor SHALL then advance by one. Twelve corners from cursor 3 with mark 9 SHALL use indexes 3 through 14 and finish at 15. Flag 5 and a flag with the high bit and bit 4 set SHALL still store them. Flag 0 and flag 1 SHALL store none and still finish 12 ahead. Cursor 4294967294 SHALL store 4294967294, 4294967295 and 0, then finish at 10. An unset flag and mark read as 0. The other line-class methods are not in.

#### Scenario: Cursor 3 stores twelve marks
- **WHEN** unit 5 draws a span from 10, 20 with span 100, stored word 4, an unset view, flag 4, mark 9 and cursor 3
- **THEN** indexes 3 through 14 each store 9, and the cursor finishes at 15

#### Scenario: A clear bit still advances
- **WHEN** the same span is drawn with flag 0 and with flag 1, mark 9 and cursor 3
- **THEN** no mark is stored, and the cursor finishes at 15

### Requirement: Span square
Stored word 4 at 10, 20 SHALL draw (3, 29), (3, 13), (19, 13), (19, 29). Stored word 6 SHALL draw (-1, 33), (-1, 9), (23, 9), (23, 33). Stored word -1 SHALL place the first corner at bits 0xdf000000, 0xdf000000. The cursor SHALL start at 0. Flag 4 and mark 9 SHALL store indexes 0 through 3 and finish at 4. Flag 0 SHALL store none and still finish at 4. Colour 17 SHALL be stored four times. The scale rebuild is not in.

#### Scenario: Word 4 draws the square
- **WHEN** a square is drawn at 10, 20 with stored word 4, colour 17, flag 4, mark 9 and cursor 99
- **THEN** the corners are those above, indexes 0 through 3 store 9, the cursor finishes at 4, and colour 17 is stored four times

#### Scenario: A clear flag still finishes at 4
- **WHEN** the same square is drawn with flag 0
- **THEN** no mark is stored, and the cursor finishes at 4

### Requirement: Line chip
At unit 5 and place 10, 20 the chip SHALL draw (10, 40), (10, 20), (30, 20), (30, 40). The first sample SHALL be 0.75, 0.8, 0.75, 1, then 0.85, 1, 0.85, 0.8. Flag 0x40 SHALL bind the second sample, 0.85, 0.8, 0.85, 1, then 0.95, 1, 0.95, 0.8, and draw the same corners again. Flag 0x10 SHALL select the first ink. Mode 0x10000 SHALL select the second ink with alpha 255. Any other flag word SHALL select the third ink.

#### Scenario: A plain chip draws one sample
- **WHEN** the chip is drawn at unit 5, place 10, 20, with a clear flag word and three distinct inks
- **THEN** the corners are those above, the first sample binds once, the second is absent, one draw is counted, and the third ink is used

#### Scenario: Bit 0x40 draws the second pass
- **WHEN** the same chip is drawn with flag 0x40
- **THEN** the second sample binds after the first, and the same corners are drawn twice

#### Scenario: The inks follow the flag word
- **WHEN** the chip is drawn with flag 0x10
- **THEN** the first ink is used
- **WHEN** the chip is drawn with flag 0x10000
- **THEN** the second ink is used with alpha 255

### Requirement: Bevel box
The bevel SHALL draw two gradient fill quads, six border lines and four outline lines. Bit 0x40 of the state SHALL select the raised arm, else bit 0x10 the pressed arm, else the idle arm. The insets SHALL be literal pixels and SHALL NOT follow the unit. Three begins, twelve binds and three ends SHALL be counted.

#### Scenario: The idle bevel draws both quads, six lines and four lines
- **WHEN** a bevel is drawn at 10, 20 of size 100 by 80 with a clear state and distinct idle inks
- **THEN** the fill runs 12, 22 to 108, 98 as two quads with the seam at 60
- **AND** the fill stops are 0.6 and 0.4
- **AND** the border runs 11.5, 21.5 to 108.5, 98.5 as six lines with stops 0.65 and 0.35, the left seam at 60.5 and the right seam at 60
- **AND** the outline runs 10.5, 20.5 to 109.5, 99.5 as four lines
- **AND** the counts are 3, 12 and 3

#### Scenario: The state selects the arm
- **WHEN** the bevel is drawn with bit 0x40 set
- **THEN** the raised inks are used
- **WHEN** it is drawn with bit 0x10 and bit 0x40 clear
- **THEN** the pressed inks are used

#### Scenario: The shipped inks shade only the pressed arm
- **WHEN** the bevel is drawn with the shipped inks
- **THEN** the raised and idle arms are fully transparent, and the pressed arm is black with alpha 0xE0

### Requirement: Capped span
At unit 5 with left 10, right 110 and centre 28 the capped span SHALL draw a left cap from 0, 18 to 30, 38, a right cap from 120, 18 to 90, 38, and a middle from 30, 28 to 90, 38. The cap sample SHALL bind twice and the middle sample once. Twelve colour writes SHALL be counted. The retail method's second positional float SHALL be dead and SHALL NOT be taken.

#### Scenario: The caps and the middle draw
- **WHEN** the capped span is drawn at unit 5 with left 10, right 110 and centre 28, colour 17 and a set view
- **THEN** the three quads are those above, the cap sample binds twice, the middle sample binds once, colour 17 is stored twelve times, and the placed corners follow the view

#### Scenario: A zero centre still splits at the extent
- **WHEN** the centre is 0 and the unit is 5
- **THEN** the top is -10 and the bottom is 10

### Requirement: Line label placement
The label SHALL align its measured width and line height from the style word and SHALL choose one ink from the state word. Exactly the listed style values SHALL select their arm; any other value SHALL use the default. One text call SHALL be counted.

#### Scenario: The style places the label
- **WHEN** a label of width 40, line height 24 and base 18 is placed at 10, 20 of size 100 by 80 at unit 5 with style 0x1000
- **THEN** x is -35 and y is 23
- **WHEN** style is 0x2000
- **THEN** x is 39
- **WHEN** style is 0x3000
- **THEN** x is 65
- **WHEN** style is 0x4000
- **THEN** x is 115
- **WHEN** style is 0
- **THEN** x is 13 and y is 23

#### Scenario: The vertical arms
- **WHEN** style is 0x100
- **THEN** y is 18
- **WHEN** style is 0x200
- **THEN** y is 73
- **WHEN** style is 0x300
- **THEN** y is 117
- **WHEN** style is 0x400
- **THEN** y is 100

#### Scenario: The state chooses the ink
- **WHEN** the state has bit 0x10
- **THEN** the first ink is used with no nudge
- **WHEN** the state has mode 0x10000 and bit 0x40
- **THEN** the coloured renderer is used and y gains one
- **WHEN** the state has bit 0x60
- **THEN** the third ink is used and y gains one
- **WHEN** the state is 0
- **THEN** the second ink is used with no nudge

### Requirement: Line rail
At unit 5 and place 10, 20 of size 100 by 80 the rail SHALL draw a left cap from 0, 85 to 30, 105, a right cap from 100, 85 to 90, 105, a middle from 30, 95 to 90, 105, and a thumb from 105, 30 to 95, 40. The cap sample SHALL bind twice, the middle once and the thumb once. Sixteen colour writes SHALL be counted. Bit 0x10 SHALL select the first ink, otherwise the second. Mode 0x10000 SHALL rebind the thumb in the third ink with alpha 255.

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

### Requirement: Line heights
The scale method SHALL store three heights from 80, 104 and 128 times the vertical scale, rounded to nearest even and keeping the low byte. It SHALL count one call to the parent's scale.

#### Scenario: Unit scale leaves the heights
- **WHEN** the vertical scale is 1
- **THEN** the heights are 80, 104 and 128

#### Scenario: A half scale halves the heights
- **WHEN** the vertical scale is one half
- **THEN** the heights are 40, 52 and 64

#### Scenario: A double scale wraps the low byte
- **WHEN** the vertical scale is 2
- **THEN** the heights are 160, 208 and 0

### Requirement: Line hit test
The hit test SHALL use a span of four units, or six when the wide switch is set. The trim test SHALL run when flags masked with 0x30 equal 0x10 and y is at or above the span. The strip test SHALL run otherwise when control bit 0x200 is set and x is past the left limit. The returned code SHALL be the original's unchanged.

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

#### Scenario: Every other point is 0
- **WHEN** the flags and control word select no test
- **THEN** the code is 0

### Requirement: Line shade
The shade SHALL draw six outer quads with the binds cap, bar, cap, cap, bar, cap, then a solid inner rectangle inset by the unit. The outer bar SHALL halve the unit with an integer shift. State band 0x10000 SHALL select the special ink with alpha 255, and the inner ink SHALL force alpha 255. Three mode calls, two begins, two ends, twenty-eight vertices and twenty-eight colour writes SHALL be counted.

#### Scenario: The outer quads and the inner rectangle draw
- **WHEN** the shade is drawn at unit 5, place 10, 20, size 100 by 80, side inset 15, with a clear state
- **THEN** the top quads span y 13 to 23 and the bottom quads y 97 to 107, the inset edges are 25 and 95, and the inner rectangle is 15, 25 to 105, 95
- **AND** the binds are cap, bar, cap, cap, bar, cap
- **AND** the counts are 3 modes, 2 begins, 2 ends, 28 vertices and 28 colour writes

#### Scenario: The state band selects the special ink
- **WHEN** the state band is 0x10000
- **THEN** the outer ink is the special ink with alpha 255
- **WHEN** it is not
- **THEN** the outer ink is the normal ink
- **AND** the inner ink forces alpha 255

#### Scenario: The unit is written back as a 16-bit place field
- **WHEN** the shade is drawn at unit 5
- **THEN** the place's unit slot holds 5 afterwards

### Requirement: Fifth line extent
Opening a skin at unit 5 SHALL store a fifth extent of 20.

#### Scenario: The fifth extent is four units
- **WHEN** a skin is opened at unit 5
- **THEN** the fifth extent is 20

### Requirement: Close-edge highlight
With the still bit set, close mark -9 SHALL draw a strip from x0 plus one unit to x0 plus three units between y0 and y0 plus two units. Mark -10 SHALL draw the same strip below y0. Mark -8 SHALL draw a four-unit square at x0, y0. The tint SHALL use the tab sample and the colour 255, 204, 153, 255.

#### Scenario: The marks draw their tints
- **WHEN** the still bit is set, the close x is 10, the close y is 20, the unit is 5 and the close mark is -9
- **THEN** the strip is (16, 40), (16, 30), (26, 30), (26, 40)
- **WHEN** the mark is -10
- **THEN** the strip is (16, 20), (16, 30), (26, 30), (26, 20)
- **WHEN** the mark is -8
- **THEN** the square is (11, 50), (11, 30), (31, 30), (31, 50)

#### Scenario: Other marks draw nothing
- **WHEN** the still bit is set and the close mark is -4
- **THEN** no close-edge tint is drawn

### Requirement: Line opening wrapper
Opening the line SHALL call the parent's open once and the texture load once.

#### Scenario: The wrapper makes both calls
- **WHEN** the line is opened
- **THEN** the parent call count is 1 and the texture call count is 1

### Requirement: Skin renderer preparation
When the global guard is not 1, preparation SHALL make two setup calls and return 1. When the guard is already 1, it SHALL make no calls and return 1.

#### Scenario: The first preparation calls setup
- **WHEN** preparation runs with a clear guard
- **THEN** the setup call count is 2 and the result is 1

#### Scenario: A repeated preparation skips setup
- **WHEN** preparation runs with the guard already set
- **THEN** the setup call count is 0 and the result is 1

### Requirement: Skin alpha scale
The method SHALL scale the first stored alpha by the horizontal scale into 14 entries and the second by the vertical scale into 48 entries. Each product SHALL round to nearest even and keep the low byte.

#### Scenario: Unit scales leave the alpha
- **WHEN** both stored alpha bytes are 255 and both scales are 1
- **THEN** all 14 and all 48 entries are 255

#### Scenario: A half scale halves the alpha
- **WHEN** the first alpha is 100, the second is 200, and both scales are one half
- **THEN** the 14 entries are 50 and the 48 entries are 100

#### Scenario: A double scale wraps the low byte
- **WHEN** the second alpha is 200 and its scale is 2
- **THEN** the 48 entries are 144

### Requirement: Skin label placement
The label SHALL align from style bits 0x7000 and 0x700 exactly as the line label. Bit 0x10 SHALL select the first ink. Mode 0x10000 SHALL select the coloured renderer with a one-pixel nudge on bit 0x40. Otherwise bit 0x40 SHALL draw a shadow one pixel left in the second ink and a second call one pixel down in the third ink. Any other word SHALL draw once in the fourth ink.

#### Scenario: The shadow arm draws twice
- **WHEN** a label of width 40, line height 24 and base 18 is placed at 10, 20 of size 100 by 80 at unit 5 with state 0x40
- **THEN** the first call is one pixel left at 12, 23 in the second ink
- **AND** the second call is one pixel down at 13, 24 in the third ink

#### Scenario: The single arms
- **WHEN** the state has bit 0x10
- **THEN** one call is made at 13, 23 in the first ink
- **WHEN** the state is 0
- **THEN** one call is made at 13, 23 in the fourth ink
- **WHEN** the state has mode 0x10000 and bit 0x40
- **THEN** the coloured renderer is used at 13, 24

### Requirement: Parent skin shade
At place 10, 20 of size 100 by 80 and inset 5 the shade SHALL draw one rectangle from 15, 25 to 105, 95, with the colour taken from the place and alpha forced to 255. It SHALL count one prepare, two mode calls and one rectangle batch.

#### Scenario: The inset rectangle draws
- **WHEN** the shade is drawn at place 10, 20, size 100 by 80, inset 5, with colour 1, 2, 3, 4
- **THEN** the corners are (105, 25), (15, 25), (15, 95), (105, 95)
- **AND** the colour is 1, 2, 3, 255
- **AND** the counts are one prepare, modes 1 and 0x10, four vertices and four colour writes

### Requirement: Skin cross
At unit 5 and point 10, 20 the cross SHALL draw eight corners: (8, 15), (13, 15), (8, 25), (13, 25), (6, 18), (6, 23), (16, 18), (16, 23). It SHALL count one begin, eight vertices and one line end, and SHALL bind no colour.

#### Scenario: The cross draws four segments
- **WHEN** the cross is drawn at unit 5 and point 10, 20
- **THEN** the eight corners are those listed, in that order
- **AND** the counts are one begin, eight vertices and one end

### Requirement: Control-code rectangle
The skin SHALL derive four metrics from the unit U and trim T: close 4U + 1, rail 4U − 1, base 6U − T, and tab U / 2 + close. Code −4 SHALL return the close band; −6 and −5 SHALL shift it left by the tab metric selected by flag bits 1 and 3; −9, −10, −13 and −14 SHALL return the rail squares; any other code SHALL return nothing.

#### Scenario: The close band
- **WHEN** unit 5 and trim 2 give close 21, rail 19, base 28 and tab 23, and the place is x 100, y 50, size 20, field a 7 and field b 90
- **THEN** code −4 returns 74, 4, 95, 26
- **AND** code −6 with flag bit 1 returns 51, 4, 72, 26
- **AND** code −5 with flag bits 1 and 3 returns 28, 4, 49, 26

#### Scenario: The rail squares
- **WHEN** the same place and metrics are used
- **THEN** code −9 returns 61, 7, 80, 26
- **AND** code −10 returns 61, 11, 80, 30
- **AND** code −13 returns 20, 11, 39, 30
- **AND** code −14 returns 71, 11, 90, 30
- **AND** any other code returns nothing

### Requirement: Skin frames
At place 10, 20 of size 100 by 80, style bits 0x30 SHALL select one double frame: 0x10 from 10, 20 to 110, 100 in the hot group's first then eighth shade, and 0x20 the same rectangle in its seventh then first. Style bits 0xc0 SHALL independently select a second double frame inset by one on every side, with 0x40 using the first then eighth shade and 0x80 the seventh then first. Any other value in a group SHALL select nothing.

#### Scenario: Each group selects its frame
- **WHEN** the style is 0x10
- **THEN** one frame is 10, 20 to 110, 100 in the hot group's first and eighth shades
- **WHEN** the style is 0x20
- **THEN** one frame is the same rectangle in the seventh and first shades
- **WHEN** the style is 0x40
- **THEN** one frame is 11, 21 to 109, 99 in the first and eighth shades

#### Scenario: Both groups run
- **WHEN** the style is 0x50
- **THEN** the first frame is 10, 20 to 110, 100 and the second is 11, 21 to 109, 99
- **WHEN** the style is 0, 0x30 or 0xc0
- **THEN** no frame is drawn

### Requirement: Skin control image
An image of size 16 in a place at 10, 20 of size 100 by 80 SHALL be centred at 52, 52, and at 52, 53 when flags & 0x60 is 0x40. Mode 0x10000 SHALL draw with scale 1 and no ink. Otherwise the ink SHALL be slot 0xd8 for flag bit 0x10 or flags & 0x11, slot 0xf4 for flag bit 0x40, and slot 0xe4 otherwise. Style bit 0x1000000 SHALL add the raised or plain colour to the 0xf4 and 0xe4 binds. Style bits 0x70000000 SHALL select frame index (style >> 28) & 7 in every arm.

#### Scenario: The image centres and nudges
- **WHEN** an image of size 16 is drawn in a place at 10, 20 of size 100 by 80
- **THEN** its position is 52, 52
- **WHEN** flags & 0x60 is 0x40
- **THEN** its position is 52, 53

#### Scenario: The inks and frames
- **WHEN** the style is 0x10000
- **THEN** no ink is bound and the scale is 1.0
- **WHEN** flag bit 0x10 is set
- **THEN** slot 0xd8 is bound without a colour
- **WHEN** flag bit 0x40 is set and style bit 0x1000000 is set
- **THEN** slot 0xf4 is bound with the raised colour
- **WHEN** neither flag bit is set and style bit 0x1000000 is set
- **THEN** slot 0xe4 is bound with the plain colour
- **WHEN** the style is 0x30000000
- **THEN** the frame index is 3

### Requirement: Parent bevel arms
`draw_skin_bevel` SHALL take two booleans and select the raised arm when the first is true, the pressed arm when the first is false and the second true, and the idle arm otherwise. It SHALL delegate the fills, six border lines and four outline lines to the shared bevel implementation.

#### Scenario: The first arm wins
- **WHEN** the first boolean is true and the second is true
- **THEN** the raised arm is used
- **WHEN** the first boolean is false and the second is true
- **THEN** the pressed arm is used
- **WHEN** both are false
- **THEN** the idle arm is used

### Requirement: Parent skin hit test
The hit test SHALL return 0x80000011 or 0x80000012 for flag 0x4000, and OR 0x80000014 or 0x80000018 for flag 0x8000, before any probe. Otherwise flag bit 0 SHALL probe −4, −5 and −10 above its y threshold and fall back to −3; flag 0x200 SHALL probe −9 and −10 from x 0x44 and fall back to −8; flag 0x100 SHALL probe −14 and −13 from y 0x3c and fall back to −12. A probe SHALL hit only when the supplied rectangle contains the point inclusively.

#### Scenario: The edge codes
- **WHEN** flag 0x4000 is set with size 10 and x 0
- **THEN** the result is 0x80000011
- **WHEN** flag 0x8000 is set with size 10, y0 100 and y 95
- **THEN** the result is 0x80000018
- **WHEN** both flags are set with x 0 and y 0
- **THEN** the result is 0x80000011 | 0x80000014

#### Scenario: The threshold probes
- **WHEN** flag bit 0 and bit 1 are set, unit 5, limit 0x40 50, and y 40, with a rectangle for −4
- **THEN** the result is −4
- **WHEN** the same point has no rectangle for any probed code
- **THEN** the result is −3

#### Scenario: The right and bottom edges
- **WHEN** flag 0x200 is set and x is 90 at limit 0x44 90, with no rectangle for −9 but one for −10
- **THEN** the result is −10
- **WHEN** neither has a rectangle
- **THEN** the result is −8
- **WHEN** flag 0x100 is set and y is 90 at limit 0x3c 90, with no rectangle for −14 or −13
- **THEN** the result is −12

### Requirement: Parent skin detail
At place 10, 20 of size 100 by 80 with inset 5 and unit 5 the method SHALL prepare once and draw the inset rectangle 15, 25, 90, 70 once. Style bit 0x20000000 with flag bit 0x40 clear SHALL add the horizontal helper at 10, 20, 100, 80 with offset −2 and mode 0x50. Style bit 0x40000000 with flag bit 0x40 set, or flag bit 0x80, SHALL add the vertical helper at the inset rectangle.

#### Scenario: The inset rectangle always draws
- **WHEN** the style is 0 and the flags are 0
- **THEN** one prepare and one rectangle call are made, and no detail helper runs

#### Scenario: The horizontal detail
- **WHEN** style bit 0x20000000 is set and flag bit 0x40 is clear
- **THEN** the horizontal helper runs at 10, 20, 100, 80 with offset −2 and mode 0x50
- **WHEN** flag bit 0x40 is set
- **THEN** it does not run

#### Scenario: The vertical detail
- **WHEN** style bit 0x40000000 and flag bit 0x40 are set
- **THEN** the vertical helper runs at 15, 25, 90, 70
- **WHEN** flag bit 0x80 is set
- **THEN** the vertical helper runs at 15, 25, 90, 70

### Requirement: Parent skin rectangle
At place 10, 20 of size 100 by 80 with inset 5 the method SHALL draw one rectangle from 15, 25 to 105, 95 through the virtual, passing flag bits 0x10 and 0x40 as two booleans. Flag bit 0x40 SHALL add one cornered overlay call at the same rectangle.

#### Scenario: The rectangle always draws
- **WHEN** the flags are 0
- **THEN** one rectangle call is made and the overlay does not run

#### Scenario: The flags pass through
- **WHEN** flag bit 0x10 is set
- **THEN** the first boolean is true and the second is false
- **WHEN** flag bit 0x40 is set
- **THEN** the second boolean is true and one overlay call runs at 15, 25, 105, 95

### Requirement: Skin textured strip
With floats 10, 20, 30 and 40 the strip SHALL draw eight vertices: (30, 20), (10, 20), (30, 30), (10, 30), (30, 30), (10, 30), (10, 40), (30, 40). It SHALL count one begin, four binds in slot order 0x12c, 0x134, 0x138, 0x130, and one end.

#### Scenario: The four bands
- **WHEN** the strip is drawn with 10, 20, 30 and 40
- **THEN** the middle y is 30
- **AND** the eight vertices are those listed, in that order
- **AND** the counts are one begin, four binds and one end

### Requirement: Parent skin slider
At place 10, 20 of size 100 by 80 with inset 5, thumb 10 and fraction one half, style 0 SHALL draw the track at 12, 22 of size 96 by 76 and the thumb at 55, 25 of size 10 by 70, translating by −40 then back. Style 0x10000000 SHALL fill 16, 26 of size 88 by 68 and draw a bar at 57, 20 of size 5 by 80. Any other style SHALL draw nothing.

#### Scenario: The plain track
- **WHEN** the style is 0 and unit is 5
- **THEN** the track is 12, 22, 96, 76
- **AND** the thumb is 55, 25, 10, 70
- **AND** the translation is −40
- **AND** flag bit 0x10 selects the first thumb boolean

#### Scenario: The filled track
- **WHEN** the style is 0x10000000 and unit is 5
- **THEN** the fill is 16, 26, 88, 68
- **AND** the bar is 57, 20, 5, 80
- **AND** one begin, one bind and one end are counted

#### Scenario: Another style
- **WHEN** the style's two bits are neither 0 nor 0x10000000
- **THEN** nothing is drawn

### Requirement: Skin measures
The method SHALL return the first, second and third measured words followed by four units plus four. With measures 2, 2 and 24 at unit 4 it SHALL return 2, 2, 24, 20, and at unit 5 it SHALL return 2, 2, 24, 24.

#### Scenario: The four measures
- **WHEN** the measures are 2, 2, 24 and the unit is 4
- **THEN** the result is 2, 2, 24, 20
- **WHEN** the unit is 5
- **THEN** the fourth word is 24

### Requirement: Skin shader programs
With the guard clear the method SHALL return three programs in order: kind 5, the flat program, with `MPrj` set to 2; kind 0x15, the rounded program, with `MPrj` set to 2 and a `Scale` lookup; and kind 0x15, the textured program, with `MPrj` set to 2, `Tex` set to 0 and a `TexCol` lookup. It SHALL count six uniform operations. With the guard set it SHALL return none.

#### Scenario: The three programs
- **WHEN** the guard is clear
- **THEN** three programs are returned with kinds 5, 0x15 and 0x15
- **AND** the first is the flat program and sets `MPrj` to 2 and nothing else
- **AND** the second is the rounded program, sets `MPrj` to 2 and looks up `Scale`
- **AND** the third is the textured program, sets `MPrj` to 2, sets `Tex` to 0 and looks up `TexCol`
- **AND** six uniform operations are counted

#### Scenario: The guard
- **WHEN** the guard is set
- **THEN** no program is returned and no uniform operation is counted

### Requirement: Inset bevel
At place 10, 20 of size 100 by 80 with inset 5 and unit 5, flag bit 0x40 SHALL draw the inner fill at 17, 27 of size 86 by 66 with slots 0x74 and 0x80, the border at 16, 26 of size 88 by 68 with slots 0x7c and 0x88, bind slot 0x6c, draw the outline at 15, 25 of size 90 by 70, and run the corner detail at 16, 26 of size 88 by 68 with mode 0xa0. Without bit 0x40 it SHALL draw one virtual rectangle at 15, 25 of size 90 by 70 with booleans from flag bit 0x10 and false.

#### Scenario: The bevel arm
- **WHEN** flag bit 0x40 is set
- **THEN** the four rectangles and the slot bind are those listed
- **AND** one prepare call is counted

#### Scenario: The plain arm
- **WHEN** flag bit 0x40 is clear and bit 0x10 is set
- **THEN** the first boolean is true and the second is false
- **AND** the rectangle is 15, 25 of size 90 by 70
- **AND** the extra height argument is 70

### Requirement: Second skin slider
At place 10, 20 of size 100 by 80 with inset 5, thumb 10, fraction one half and unit 5, style 0 SHALL draw the track at 11, 22 of size 98 by 76 and the thumb at 55, 26 of size 10 by 68, translating by −40 then back. Style 0x10000000 SHALL fill 16, 26 of size 88 by 68 and draw a bar at 57, 20 of size 5 by 80. Any other style SHALL draw nothing.

#### Scenario: The plain arm
- **WHEN** the style is 0
- **THEN** the track is 11, 22, 98, 76
- **AND** the thumb is 55, 26, 10, 68
- **AND** the translation is 40 with flag bit 0x10 as the first boolean

#### Scenario: The filled arm
- **WHEN** the style is 0x10000000
- **THEN** the fill is 16, 26, 88, 68
- **AND** the bar is 57, 20, 5, 80
- **AND** one begin, one bind and one end are counted

#### Scenario: Another style
- **WHEN** the style is 0x20000000
- **THEN** nothing is drawn

### Requirement: Skin scroll region
With origin 10, 20, inset 5, 4, extent 100, 80, totals 150, 120 and fractions one half and one quarter, the region SHALL clip at 15, 24 of size 95 by 76 and scroll by 28 on x and 11 on y. Flag 0x800 SHALL zero both offsets. Mode bit 0 or a rejected clip SHALL stop after the first begin. Flag 0x80 SHALL add a 0x30 detail at the scroll offsets with offset one unit; flag 0x100000 SHALL add a 0x50 detail with offset four units.

#### Scenario: The clip and scroll
- **WHEN** the mode is 0 and the clip is accepted
- **THEN** the clip is 15, 24, 95, 76
- **AND** the scroll is 28, 11
- **AND** two begins, two scissor calls and one line end are counted
- **AND** every quad and every visible child is replayed

#### Scenario: The early stops
- **WHEN** mode bit 0 is set
- **THEN** only one begin runs and nothing is clipped
- **WHEN** the clip is rejected
- **THEN** only one begin runs and nothing is clipped

#### Scenario: The details and translation
- **WHEN** flag 0x800 is set
- **THEN** the scroll is 0, 0
- **WHEN** flag 0x80 is set
- **THEN** the detail is at 28, 11 with mode 0x30 and offset 5
- **WHEN** flag 0x100000 is set
- **THEN** the detail has mode 0x50 and offset 20
- **WHEN** the translation applied
- **THEN** it moves by −10 and 20

### Requirement: Menu clock scale
The main menu SHALL convert a clock word by multiplying it by 1/135, rounding to nearest even, then multiplying by 1/8. A clock of 0 SHALL give 0, 135 SHALL give 1/8, 270 SHALL give 1/4, 67 SHALL give 0 and 68 SHALL give 1/8.

#### Scenario: Whole steps
- **WHEN** the clock is 135
- **THEN** the result is 0.125
- **WHEN** the clock is 270
- **THEN** the result is 0.25

#### Scenario: The rounding boundary
- **WHEN** the clock is 67
- **THEN** the result is 0
- **WHEN** the clock is 68
- **THEN** the result is 0.125

### Requirement: Menu close key
A key mask carrying both 0x2000 and 0x2 SHALL close the form with argument 0 and return 1 without running the base handler. Any other mask SHALL return the base handler's result and SHALL NOT close.

#### Scenario: The close mask
- **WHEN** the mask is 0x2002
- **THEN** the form closes and the result is 1
- **AND** the base handler does not run

#### Scenario: Any other mask
- **WHEN** the mask is 0x2000 or 0x2 and the base handler returns 7
- **THEN** the result is 7 and the form does not close

### Requirement: Class select index
The class-select screen SHALL store the index at field 0x6c and return 3. It SHALL play one selection sound only when the index and the sound flag are both above zero.

#### Scenario: A positive index
- **WHEN** the index is 4 and the sound flag is positive
- **THEN** field 0x6c is 4, one sound plays and the result is 3

#### Scenario: The guard
- **WHEN** the index is 0 or negative
- **THEN** no sound plays and the result is still 3
- **WHEN** the index is positive but the sound flag is zero
- **THEN** no sound plays

### Requirement: Skill tooltip
At tooltip 100, 50 and unit 5 the icon SHALL draw at 10, 25 and the label SHALL occupy 80, 50 of size 30 by 10. The draw SHALL count one begin, one bind, two setup calls and two draws.

#### Scenario: The icon and label
- **WHEN** the tooltip is at 100, 50 and the unit is 5
- **THEN** the icon is at 10, 25
- **AND** the label is at 80, 50 of size 30 by 10
- **AND** the counts are one begin, one bind, two setup calls and two draws

### Requirement: Transition fade
On page 2 at 200, 50 with a glyph of 8 by 16, colour 10, 20, 30 and alpha 255, the transition SHALL draw "TO BE CONTINUED" at 40, 17 with blend mode 0x203 and one begin and one draw. Any other page SHALL draw nothing but still call the base draw.

#### Scenario: The fade page
- **WHEN** the page is 2
- **THEN** the text is "TO BE CONTINUED" at 40, 17
- **AND** the colour is 10, 20, 30, 255 and the blend mode is 0x203
- **AND** one begin, one draw and one font selection are counted

#### Scenario: Another page
- **WHEN** the page is not 2
- **THEN** nothing is drawn and the base draw is still called

### Requirement: Skill selection settle
The selector SHALL take the low 16 bits of the raw word and clamp an index above 8 to 0. On page 1 a positive index SHALL set bits 0x110 and a zero index SHALL clear them. Any other page SHALL leave the flags unchanged.

#### Scenario: A positive index on page one
- **WHEN** the raw word is 4, the page is 1 and the flags are 0
- **THEN** the index is 4 and the flags become 0x110

#### Scenario: The clamp
- **WHEN** the raw word is 9 or -1
- **THEN** the index is 0 and the flags lose 0x110
- **WHEN** the page is not 1
- **THEN** the flags are unchanged

### Requirement: Inventory close key
A key mask carrying both 0x2000 and 0x2 SHALL close with argument 0 and return 1. It SHALL use the stored callback when one exists and the form close otherwise. Any other mask SHALL return the base handler's result and SHALL NOT close.

#### Scenario: The callback path
- **WHEN** the mask is 0x2002 and a callback exists
- **THEN** the callback runs with argument 0, the form close does not run and the result is 1
- **WHEN** the mask is 0x2002 and no callback exists
- **THEN** the form close runs and the result is 1

#### Scenario: Any other mask
- **WHEN** the mask is 0x2000 and the base handler returns 7
- **THEN** the result is 7 and nothing closes

### Requirement: Skill selector key
The selector SHALL run its base handler when the selection is above zero and the page is above 1, or when the word's low 16 bits are 9 and the mode byte is clear. Otherwise it SHALL close with argument 0 and return 0x40.

#### Scenario: The base paths
- **WHEN** the selection is 3 and the page is 2
- **THEN** the base handler runs and its result returns
- **WHEN** the selection is 0, the word is 9 and the mode byte is 0
- **THEN** the base handler runs and its result returns

#### Scenario: The close path
- **WHEN** the selection is 0 and the word is 4
- **THEN** the form closes and the result is 0x40
- **WHEN** the selection is 0, the word is 9 and the mode byte is 1
- **THEN** the form closes and the result is 0x40

### Requirement: End-session dispatch
Page 2 SHALL continue for action 1 and show the main menu for action 2. Page 4 SHALL reload the stored level for action 1 on form page 4, else end to the main menu. Page 8's action 1 SHALL, on form page 2, hide the kept form, make its second call, prepare the character record and show its form, and otherwise show the next practice fight with the last-round flag. Other pages SHALL call their registered handler. Every path SHALL clear the action and hovered selection, returning 0.

#### Scenario: The built-in pages
- **WHEN** the page is 2 and the action is 1
- **THEN** the continue runs
- **WHEN** the page is 4 and the action is 1 on form page 4
- **THEN** the stored level is reloaded
- **WHEN** the page is 8, the action is 1 and the form page is 2
- **THEN** the three calls and page 8's form are dispatched

#### Scenario: The fallback
- **WHEN** the page is 3 and a handler exists
- **THEN** the handler is called
- **WHEN** the page is 3 and no handler exists
- **THEN** nothing is dispatched
- **AND** every path clears the action and field 0x204 and returns 0

### Requirement: Menu hover
The menu SHALL take its hover value when it equals field 0x204 and zero otherwise. An unchanged selection SHALL return 0. A change SHALL store the selection, play a sound when it is non-zero and the last stamp plus 200 is below the clock, update the stamp only in that case, and return 3.

#### Scenario: An unchanged selection
- **WHEN** the hover, field 0x204 and field 0x70 are all 5
- **THEN** nothing changes and the result is 0

#### Scenario: A changed selection
- **WHEN** the hover and field 0x204 are 5, field 0x70 is 0, the stamp is 0 and the clock is 300
- **THEN** the selection becomes 5, a sound plays, the stamp becomes 300 and the result is 3
- **WHEN** the selection is 0
- **THEN** no sound plays and the result is still 3
- **WHEN** the stamp is 200 and the clock is 300
- **THEN** no sound plays and the stamp stays

### Requirement: Main-menu key
Mask 0x100 SHALL confirm a positive selection with the sound and suppress a zero one. Mask 0x200 SHALL call forward and 0x400 back. Direction 0x8000 SHALL move to max(selection + 1, 1) while below the limit; direction 0x4000 SHALL move to selection − 1 while above 1. The result SHALL be the helper's result or 0.

#### Scenario: The confirm
- **WHEN** the mask is 0x100 and the selection is 5
- **THEN** the confirm runs with the selection, a sound plays and the helper result returns
- **WHEN** the selection is 0
- **THEN** nothing runs and the result is 0

#### Scenario: The directions
- **WHEN** the mask is 0x8001, the selection is 2 and the limit is 4
- **THEN** the selection moves to 3
- **WHEN** the selection is 4
- **THEN** it does not move
- **WHEN** the selection is 0
- **THEN** it moves to 1
- **WHEN** the mask is 0x4001 and the selection is 3
- **THEN** it moves to 2

### Requirement: Inventory detach
An already inactive pane SHALL return true without work. Otherwise the detach SHALL mark its owner, place a visible child, decrement a stack above one or free a stack of one, copy the loot pane's origin, run the base method once and return its result.

#### Scenario: The inactive pane
- **WHEN** the pane is already inactive
- **THEN** the result is true and the base method does not run

#### Scenario: The active pane
- **WHEN** the pane has an owner and a visible child
- **THEN** the owner is marked and the child is placed
- **WHEN** the stack count is 2
- **THEN** it becomes 1 and its top is cleared
- **WHEN** the stack count is 1
- **THEN** it is freed and cleared
- **AND** the base result returns

### Requirement: Inventory page sentinel
The page override SHALL store −3 at field 0x6c when its argument is −1, and SHALL leave the field for any other argument. The base result SHALL return.

#### Scenario: The sentinel
- **WHEN** the argument is −1
- **THEN** field 0x6c becomes −3
- **WHEN** the argument is 4
- **THEN** field 0x6c keeps its value

### Requirement: Main-menu name entry
A key without bit 0x2 SHALL go to the base handler and leave the buffer. Otherwise the ten bytes SHALL shift left by one and the character SHALL be appended, with A–Z lowered by 0x20. A buffer of "barechoons" SHALL run the submit routine.

#### Scenario: The shift
- **WHEN** the buffer is "?barechoon" and the character is "s"
- **THEN** the buffer becomes "barechoons" and the submit routine runs
- **WHEN** the character is "A"
- **THEN** the last byte becomes "a"

#### Scenario: The base path
- **WHEN** the key has no bit 0x2
- **THEN** the base handler runs and the buffer is unchanged

### Requirement: Splash bounds
The splash bounds SHALL write the screen to the first output, −1 to the second and return −1.

#### Scenario: The bounds
- **WHEN** the bounds run for a screen
- **THEN** the first output is the screen, the second is −1 and the result is −1

### Requirement: Splash destroy
Destroy SHALL make one release call on a set free flag, clear the splash global, free each stored pointer present, make one list call for a positive count, always run the base destructor, and make a final call only on a set free flag.

#### Scenario: The freeing path
- **WHEN** the free flag is set, both pointers are present and the count is 2
- **THEN** one release, two frees, one list call, one base call and one final call are counted
- **AND** the global is cleared

#### Scenario: The quiet path
- **WHEN** the free flag is clear and both pointers are absent
- **THEN** no release, no free, no list call and no final call are made
- **AND** the base call still runs

### Requirement: Skill selector draw
At place 10, 180 with a right edge of 110, unit 5 and a measured icon of 20, page 2 SHALL draw a title at 60 across, 90 wide, 100 high. Another page with index 3 SHALL draw the icon at 40, 80 and a label at 90 across, 90 wide, 80 high.

#### Scenario: The title page
- **WHEN** the page is 2
- **THEN** the title starts at 60 across with width 90 and height 100
- **AND** one text draw runs

#### Scenario: A selected skill
- **WHEN** the page is not 2, the raw word is 4 and the icon measures 20
- **THEN** the icon is at 40, 80 and the label starts at 90 across with width 90 and height 80
- **AND** two draws run
- **WHEN** the raw word is 9 or 0
- **THEN** only the begin and bind run

### Requirement: Skill tooltip show
A missing row SHALL show nothing. A found row SHALL set the index to object+0xb8 − 1 and the threshold to row+0x38 − 1, add 0x10 to the flags, place itself at the anchor x and nine units plus the anchor y, use a height of 24 units when global 0x2ad3c0 is 0 and 18 otherwise, set visible, make three calls and stamp three times the base plus the offset.

#### Scenario: A found row
- **WHEN** a row is found, the anchor is 7, 8, the object word is 5, the row word is 4 and the unit is 5
- **THEN** the index is 4 and the threshold is 3
- **AND** the place is 7, 53 and the height is 120
- **AND** the flags gain 0x10, visible is true and three calls run
- **AND** the stamp is three times 10 plus 20, which is 50

#### Scenario: A missing row
- **WHEN** no row is found
- **THEN** nothing shows and the calls do not run

### Requirement: Character loot show
An already inactive loot screen SHALL return true without work. Otherwise it SHALL set bit 0x20000000 in the owner's word, place the child when present and visible, run the base method once and return its result.

#### Scenario: The active loot screen
- **WHEN** the screen is not inactive, has an owner and a visible child
- **THEN** the owner is marked, the child is placed, one base call runs and its result returns
- **WHEN** the screen is already inactive
- **THEN** the result is true and the base method does not run

### Requirement: Journal show
The journal SHALL copy its x and y to the two origin globals, assign the string at the page's table offset and run the base method once.

#### Scenario: A journal page
- **WHEN** the page is 3
- **THEN** the origin copies, the table offset is 3 × 0x18 + 0x10 and one base call runs

### Requirement: Character view destroy
The destructor SHALL make one release call on a set free flag, always run the base destructor once and make one final call only on a set free flag.

#### Scenario: The freeing path
- **WHEN** the free flag is set
- **THEN** one release call, one base call and one final call are counted
- **WHEN** the free flag is clear
- **THEN** only the base call runs

### Requirement: Character view clear
When a view exists the clear SHALL release it once, write zero to the owner's 0x130 word and clear the pointer. The base method SHALL always run and its result SHALL return.

#### Scenario: An existing view
- **WHEN** a view exists
- **THEN** one release call runs, the owner slot is cleared and the pointer is cleared
- **WHEN** no view exists
- **THEN** no release call runs
- **AND** the base result returns in both cases

### Requirement: Skill selector show
The show SHALL clear bits 0x60 on the object when one exists, apply a positive selection, or apply −1 when the word's low 16 bits are nine or ten. It SHALL apply nothing for any other word or a missing object, and the base method SHALL always run.

#### Scenario: The selection
- **WHEN** an object exists and the selection is 3
- **THEN** 3 is applied and bits 0x60 clear
- **WHEN** the selection is 0 and the word is 9 or 10
- **THEN** −1 is applied
- **WHEN** the selection is 0 and the word is 4
- **THEN** nothing is applied
- **WHEN** no object exists
- **THEN** nothing is applied and the base still runs

### Requirement: Class confirm
A positive selection SHALL store itself and one less, return 7, play a sound when the sound global is positive, copy the page global and show the class form with flag 0. A selection of zero or less SHALL return 0x40 and do nothing else.

#### Scenario: A positive selection
- **WHEN** the selection is 3 and the sound global is positive
- **THEN** the selection is stored, the global becomes 2, the sound plays, the page copies, the form shows and the result is 7

#### Scenario: The guard
- **WHEN** the selection is 0 or negative
- **THEN** the result is 0x40 and no form shows

### Requirement: Class navigation
Mask 0x100 SHALL confirm a positive selection with result 7 and the class form, and return 0 for no selection. Masks 0x200 and 0x400 SHALL call the forward helper. Direction 0x8000 SHALL move down to the first available class at or after the selection up to 3; direction 0x4000 SHALL move up from selection − 2, or to 1 when at most 2. The move SHALL run only when the target differs.

#### Scenario: The confirm
- **WHEN** the mask is 0x100 and the selection is 3
- **THEN** the result is 7 and the class form shows
- **WHEN** the selection is 0
- **THEN** the result is 0

#### Scenario: The bit scans
- **WHEN** the available global is 0b1010, so classes 0, 2 and 4 exist
- **THEN** down from 1 moves to 3
- **AND** up from 4 moves to 3
- **AND** up from 2 moves to 1
- **AND** down from 4 does not move

### Requirement: Class card hit test
With screen width 1000, clock 2048, scale 1 and available mask 1, the band SHALL run from 652 to 1164 and the two cards SHALL occupy 212–468 and 500–756. A point at 300, 700 SHALL name card 1 and 501, 700 card 2. Points on an edge or outside the band SHALL name none.

#### Scenario: The two cards
- **WHEN** the point is 300, 700
- **THEN** the named card is 1
- **WHEN** the point is 501, 700
- **THEN** the named card is 2

#### Scenario: The edges
- **WHEN** the point is 468, 700 or 212, 700
- **THEN** no card is named
- **WHEN** the point is 300, 652 or 300, 1164
- **THEN** no card is named
- **WHEN** the point is 300, 653
- **THEN** card 1 is named

### Requirement: Main-menu confirm
A selection of zero or less SHALL copy field 0x70 to 0x204 and return 0x40. A positive selection with direction 0x1000 SHALL store itself at 0x70, copy it, play a sound when ready and return 7; any other direction SHALL return 0x40 without copying.

#### Scenario: The confirmation
- **WHEN** the selection is 3 and the direction is 0x1000
- **THEN** 3 is stored, copied to 0x204, the sound plays and the result is 7
- **WHEN** the selection is 3 and the direction is 0x2000
- **THEN** nothing is copied and the result is 0x40
- **WHEN** the selection is 0
- **THEN** 0x70 copies to 0x204 and the result is 0x40

### Requirement: Main-menu select
The select SHALL store the index and return 3. It SHALL play a sound and update the stamp only when the index is positive, the sound is ready and the stamp plus 200 is below the clock.

#### Scenario: The select
- **WHEN** the index is 3, the stamp is 0 and the clock is 300
- **THEN** the sound plays, the stamp becomes 300 and the result is 3
- **WHEN** the index is 0 or the stamp is 200
- **THEN** no sound plays and the stamp stays

### Requirement: Skill node position
The position SHALL be the layout point at the row in bits 8 through 15 and the column in the low 8 bits of a positive index, rounded to the nearest even integer. A non-positive index SHALL take the base position when the scaled flag is set, otherwise half the width and half the height, truncated toward zero.

#### Scenario: Placed, centred and forwarded
- **WHEN** the index is 0x0205 and the layout returns 12.5 and 34.5
- **THEN** the row is 2, the column 5 and the position 12, 34
- **WHEN** the index is 0, the bounds are 10, 110, 20 and 80 and the flag is clear
- **THEN** the position is 50, 30
- **WHEN** the index is −1 and the flag is set
- **THEN** the base position is returned

### Requirement: Skill screen show
Showing SHALL run the relayout when the open flag is set, set bit 1 of the state and store the clock plus the delay as the deadline.

#### Scenario: The opening
- **WHEN** the open flag is set, the state is 0, the clock 100 and the delay 250
- **THEN** the relayout runs and the state becomes 2
- **AND** the deadline is 350

### Requirement: Skill screen leave
Leaving SHALL call the base when the closing flag is set. Otherwise a non-zero page SHALL do nothing. A set bit 0 with a deadline more than 400 above the clock SHALL do nothing, and a set bit 1 SHALL do nothing; without either, the clear call SHALL run.

#### Scenario: The leave
- **WHEN** the closing flag is set
- **THEN** the base call runs and the clear call does not
- **WHEN** the page is 2
- **THEN** nothing runs
- **WHEN** the state is 1, the deadline 900 and the clock 500
- **THEN** the clear call runs
- **WHEN** the state is 1, the deadline 1000 and the clock 500
- **THEN** nothing runs
- **WHEN** the state is 2
- **THEN** nothing runs

### Requirement: Skill ring position
The angle SHALL be `(1 − turn) + (slot − 3.5) × 2π / count`, summed in that order. The x offset SHALL be the radius times the unit times the engine's sine times the horizontal stretch, the y offset the radius times the unit times the cosine, each rounded to the nearest even integer and multiplied by the turn, and added to the centre. The centre x SHALL lose one unit with the inset-layout flag on a positive page; the centre y SHALL lose three units on page 0.

#### Scenario: A node on the ring
- **WHEN** the count is 8, the radius 10, the stretch 5, the turn 1, the size 200 by 100, the page 0, the unit 4 and the slot 3
- **THEN** the position is 23, 75
- **WHEN** the page is 1 and the inset-layout flag is set
- **THEN** the position is 19, 87

### Requirement: Skill node hit
For each node within 23 units of its centre, five samples SHALL sit at 18.3 units around it, 0.2 of a turn apart, with `x + round(sin) × turn` and `y − round(cos) × turn`. A sample within 5 units SHALL OR its index into the result and add one. The result SHALL start at 0 on page 2 and −1 otherwise.

#### Scenario: A sample and a miss
- **WHEN** page is 2, the unit is 4, the turn is 1, the node is at 100, 100 and the mouse is at 100, 27
- **THEN** the result is 1
- **WHEN** the mouse is at 170, 77
- **THEN** the result is 2
- **WHEN** the page is 0 and the mouse is far
- **THEN** the result is −1

### Requirement: Skill hover
Setting the hover SHALL store a positive index or current and return 3. The candidate SHALL be the index when bits 8–10 and the low 3 bits are positive and its grid byte is set, else 0. An unchanged candidate SHALL leave the stamps; a new positive candidate with a different last positive SHALL stamp the clock, else the stamp SHALL take the elapsed with the under-200 correction. The candidate SHALL become previous and, when positive, last positive. With both non-positive the base runs.

#### Scenario: Hover, miss and stamp
- **WHEN** the index is 0, the current is 0 and the base is 7
- **THEN** the result is 7 and the base ran
- **WHEN** the index is 0x0203, grid byte 20 is set, previous is 0, stamp 900 and clock 1000
- **THEN** the candidate and last positive are 0x0203
- **AND** the stamp stays 900
- **WHEN** the last positive is 0x0101 instead
- **THEN** the stamp becomes 1000
- **WHEN** the candidate equals the previous
- **THEN** the stamps do not change
- **WHEN** the grid byte is clear
- **THEN** the previous stays 0

### Requirement: Skill satellite
The satellite SHALL start from the stored slot's ring point at `(1 − turn) + (stored − 2.5) × 2π / count`, then add the pentagon offset of `sample − 1`: `18.3 × unit` times the sine across and minus the cosine down, each rounded in double precision and multiplied by the turn.

#### Scenario: Two samples
- **WHEN** the count is 8, the radius 10, the stretch 5, the turn 1, the size 200 by 100, the page 0, the unit 4 and the stored slot 3
- **THEN** sample 1 is 177, 2
- **AND** sample 3 is 220, 134
- **WHEN** the page is 1 and the inset-layout flag is set
- **THEN** sample 1 is 173, 14

### Requirement: Match board show
Showing SHALL clear the stored pointer and byte, store the seed and each of the three columns' child count less one. Each of the five panels with a positive count SHALL be cleared, and a present pointer SHALL be cleared. The base result SHALL be returned and a present global SHALL get the minus-one call.

#### Scenario: The opening
- **WHEN** the seed is 7, the columns are 3, 1 and 0, the panels are 1, 0, 2, 0 and 5, the pointer is present, the base is true and the global is absent
- **THEN** the stored columns are 2, 0 and −1
- **AND** the cleared panels are the first, third and fifth
- **AND** the pointer is cleared, the result is true and the global is not called

### Requirement: Match board destroy
Destroying SHALL run the base destructor when the flags are positive, clear the stored pointer, release the list at `0x1a0`, run the base, and free when the object and flags are both present.

#### Scenario: The destroy
- **WHEN** the flags are 1
- **THEN** the base destructor, the list release, the base and the free all run
- **WHEN** the flags are 0
- **THEN** the base destructor and the free do not run

### Requirement: Skill key move
Moving SHALL step the index by the key's low mask. With a high selection it SHALL step back one, or forward while below two. Without one it SHALL scan the grid back from the index less two, or forward from the index less one, for the first zero and select the slot after it. A positive moved index different from the old SHALL be published.

#### Scenario: Simple, backward and forward
- **WHEN** the high selection is positive, the index 3 and the key portion `0x1001`
- **THEN** the index becomes 2 and 2 is published
- **WHEN** the high selection is absent, the index 4 and grid slot 2 is zero
- **THEN** the index becomes 3
- **WHEN** the high selection is absent, the index 4 and grid slot 4 is zero
- **THEN** the index becomes 5

### Requirement: Skill key dispatch
A masked key SHALL run the base call, or the hint with a 200 hide and bit `0x40` cleared. Key `0x100` SHALL store the high or low slot and the scanned index, publish it, hint, hide for 300 and set bit `0x40`, then run the closing, cursor and clear steps. Any other repage SHALL end. The result SHALL be 0.

#### Scenario: Base, key 0x100 and closing
- **WHEN** the key has bit `0x100` and the high selection is 4
- **THEN** the base runs and 4 is published
- **WHEN** the key is `0x100`, the repage 1 and the index 5
- **THEN** the high slot is 5, 1 is published, the hint runs, the hide is 1, the deadline is clock plus 300 and bit `0x40` is set
- **WHEN** the repage is 0
- **THEN** nothing further runs
- **WHEN** the closing word is 1
- **THEN** the handler ends before the cursor and clear steps

### Requirement: Skill pointer mask
An index below −1 SHALL return the base result. Mask bit 1 SHALL leave on a positive page, return on a closing word of 1, finish on a larger one, and otherwise clear when state bit 1 is clear and return 1. An index at or below zero with a selection SHALL hide for 200 and hint. The result SHALL otherwise be 16.

#### Scenario: Base, clearing and hiding
- **WHEN** the index is −2 and the base returns 7
- **THEN** the base runs and the result is 7
- **WHEN** mask bit 1 is set, the page is 0, the closing is 0 and state bit 1 is clear
- **THEN** the clear runs and the result is 1
- **WHEN** mask bit 1 is set and the closing is 1
- **THEN** the result is 16 and nothing clears
- **WHEN** the index is 0, a selection exists and the hide is 0
- **THEN** the hide becomes 2 and the deadline is the clock plus 200

### Requirement: Skill pointer selection
Index `0x10000` with mask bit `0x2000` SHALL show for 500 at state byte 4, return on state bit 1, or gain bit 1 and set the other deadline to the clock plus the delay. A selected index with flag `0x1ea` clear SHALL repage, select on index 1, or write its slot when the used byte is clear, then hint and hide.

#### Scenario: Showing, repaging and writing
- **WHEN** the index is `0x10000`, mask bit `0x2000` is set and the state byte is 4
- **THEN** the state becomes 1 and the show deadline is the clock plus 500
- **WHEN** the index is 5, mask bit `0x2000` is set and both selections are zero with repage 1
- **THEN** the high slot is 5, the hint runs, the hide is 1 and the deadline is the clock plus 300
- **WHEN** the index is 1 and a high selection exists
- **THEN** the selection runs
- **WHEN** the used byte is clear and the selection result is positive
- **THEN** the selection runs and the row and column slot is written

### Requirement: Skill point guards
The hit SHALL return 0 when state bit 1 is set and the page is below 2, and the base result when the form does not own the point or the base is not −1 and the guard refuses. On page 2 a point in the bottom band SHALL return `0x10000`. A state byte of 4 SHALL return the base result.

#### Scenario: Early returns and the band
- **WHEN** state bit 1 is set and the page is 1
- **THEN** the result is 0
- **WHEN** the form does not own the point
- **THEN** the base result is returned
- **WHEN** the page is 2, the size is 200 by 100, the unit 4 and the point 100, 50
- **THEN** the result is `0x10000`
- **WHEN** the state byte is 4
- **THEN** the base result is returned

### Requirement: Skill point nodes
A node awaiting the forget confirmation SHALL be tested first: a point six to ten units below it SHALL give 1 for a rounded horizontal offset between 13 and 4 units to the left and 2 for the same to the right, else −1. A node whose learning row is open SHALL give, for a point in the eight units above it, the slot `round(offset / (10 × unit) + 4.5)` half to even when it is 1 through 8, else −1. Without either the fallback SHALL run.

#### Scenario: High, low and fallback
- **WHEN** the forget node is at 100, 50, the unit 4 and the point 80, 80
- **THEN** the result is 1
- **WHEN** the point is 120, 80
- **THEN** the result is 2
- **WHEN** the learning node is at 100, 40 and the point 120, 20
- **THEN** the slot is 5
- **WHEN** the point is 100, 20
- **THEN** the slot is 4
- **WHEN** no selection exists
- **THEN** the fallback result is returned

### Requirement: Skill link first pass
The search SHALL test the two slots beside a positive hit node, wrapping into 0 through 4. A reachable slot SHALL place its satellite at 18.3 units and 0.2 of a turn, scale the y offset by 4 when the direction's y magnitude is below its x magnitude, normalize the offset and keep the largest dot above 0.1. A kept slot SHALL be written and returned.

#### Scenario: The adjacent slot
- **WHEN** the hit is `0x0202`, the node is at 10, 10, the point 0, 0, the direction 1, 0 and the unit 1
- **THEN** the returned code is `0x0201` and the point 28, 10

### Requirement: Skill link scan
Without a kept slot the search SHALL scan every node's five slots, requiring a squared offset of at least 1 and a dot of at least 0.3, and keep the smallest `length / dot²` from a one-million start. The winner SHALL be written and returned.

#### Scenario: The five-slot scan
- **WHEN** the hit is 0, one node is at 0, 0, the point 0, 0, the direction 0, 1 and the unit 1
- **THEN** the returned code has low slot 5 and the point is 6, 17
- **WHEN** no code is reachable
- **THEN** code 0 and the input point are returned

### Requirement: Skill screen fade
State byte 4 SHALL return alpha and sound 0. Bit 0 clear SHALL return 1. Bit 0 set with the deadline at or below the clock SHALL clear the bit and return 1. Otherwise `t = (deadline − clock) × 0.002`, `x = (1 − t)(1 + t)`, alpha SHALL be the smaller of x and 1, and sound SHALL be `x(2 − x)`.

#### Scenario: Closing, plain and fading
- **WHEN** the state byte is 4
- **THEN** alpha and sound are 0
- **WHEN** bit 0 is clear
- **THEN** alpha and sound are 1
- **WHEN** bit 0 is set, the deadline 500 and the clock 600
- **THEN** the bit clears and alpha and sound are 1
- **WHEN** bit 0 is set, the deadline 1000 and the clock 800
- **THEN** alpha is 0.84 and sound is 0.9744

### Requirement: Skill screen fade-in
With bit 1 clear nothing SHALL change. With the deadline at or below the clock the state SHALL become 4; page 2 SHALL return alpha 0 and any other page SHALL close. Otherwise the sound SHALL become `(deadline − clock) × sound / delay`.

#### Scenario: Clear, closing and fading
- **WHEN** bit 1 is clear
- **THEN** nothing changes
- **WHEN** bit 1 is set, the deadline 500 and the clock 600 on page 2
- **THEN** the state is 4 and alpha is 0
- **WHEN** the page is 1
- **THEN** the close runs
- **WHEN** the deadline is 1000, the clock 800, the sound 0.5 and the delay 250
- **THEN** the sound becomes 0.4

### Requirement: Main menu forward
The main menu's forward action SHALL run action 0 on page 1 and action 1 on page 8, each entering the session, and SHALL sound only when the input is ready. Any other page SHALL do nothing.

#### Scenario: Page one, page eight and another page
- **WHEN** the page is 1 and the input is ready
- **THEN** the action is 0, the sound plays and the session entry follows
- **WHEN** the page is 8 and the input is not ready
- **THEN** the action is 1, no sound plays and the session entry follows
- **WHEN** the page is 2
- **THEN** no action, no sound and no session entry

### Requirement: Character view chain walk
The character view's chain walk SHALL stop at a node whose record is missing, or whose flags carry both 0x12000000 with a negative word at 0x468; any other node SHALL follow its next node at 0x128.

#### Scenario: Settled, missing and open nodes
- **WHEN** the first node has no matching bits and points at a node carrying both 0x12000000 with a negative word
- **THEN** the walk stops at the second node
- **WHEN** the second node carries the bits with a word of 0 and points at a node without a record
- **THEN** the walk stops at the third node
- **WHEN** the only node carries one of the bits and no next node
- **THEN** the walk returns nothing

### Requirement: Settings menu lifecycle
Opening the settings menu SHALL create it when none exists and otherwise re-show it, SHALL lay it out, SHALL select entry 1 behind the resume flag, and SHALL finish with −1. Closing SHALL act only on an existing menu, and clearing its flag SHALL act only on an existing menu.

#### Scenario: Created, shown and absent
- **WHEN** no settings menu exists and the resume flag is clear
- **THEN** the opening creates it, lays it out, selects nothing and finishes with −1
- **WHEN** a settings menu exists and the resume flag is set
- **THEN** the opening re-shows it and selects entry 1
- **WHEN** no settings menu exists
- **THEN** closing and clearing its flag do nothing
- **WHEN** one exists
- **THEN** closing and clearing its flag each act once

### Requirement: Item-panel grid
The system SHALL shape the grid as `cols = round(√(n·W/H))`, `rows = ceil(n/cols)`, fill entries backwards from the last index, and spread the last partial row over the panel's width.

#### Scenario: Three entries
- **WHEN** three entries fill a 400×300 panel with a 160 cell
- **THEN** the grid is 2×2, the last entry is at (10, −15), the middle at (210, −15) and the first at (110, 135)

### Requirement: Item-panel relaxation
The system SHALL place a new entry at a quarter of the panel plus a draw below half of it from the engine's shared generator and push it out over six passes of two rounds, an axis pushing only when its overlap (slack 2) is above the pass number by a signed compare and the entry moving only when both axes push.

#### Scenario: One entry against another
- **WHEN** a new entry is placed with the generator at 0 against another at (100, 100), both half 80, in a 400×400 panel
- **THEN** it lands at (100, 106), moves to (262, 262), the other stays, and the generator has advanced twice

#### Scenario: A far entry
- **WHEN** the new entry lands at (100, 106) and the other entry, half 10, is at (390, 390)
- **THEN** both overlaps are negative and the new entry stays

### Requirement: Item-panel grid relaxation
The system SHALL jitter each ideal grid centre by up to an eighth of a cell from the panel's own seeded generator, give the first slot to the last entry, and relax over five passes dividing the overlap by 4 to 8, each pass ending with every centre kept inside the panel.

#### Scenario: No jitter
- **WHEN** three entries with no extent are laid out in a 400×300 panel with a 4-pixel cell
- **THEN** the last entry's cell is at (98, 73) and the first's at (198, 223)

### Requirement: Item-panel scatter
The system SHALL scatter every entry over the middle half of the panel from the engine's shared generator, x then y for each, and relax over six passes dividing by 1, 1, 2, 2, 3 and 3, with slack 2 when the other entry is further along and 1 otherwise, both entries moving only when both axes overlap, each pass ending with the clamp.

#### Scenario: Five entries
- **WHEN** five entries of half 80 are scattered in a 400×400 panel
- **THEN** the generator advances ten times and every centre lies within 80 to 320

### Requirement: Item-panel hit test and drop
The system SHALL hit-test from the last entry backwards over the cell box, skipping held entries, first by the icon's own pixels and then, when nothing was hit and the GUI's switch is on, by the icon cache's second test. A drop SHALL subtract the grab offset and keep the entry inside by its icon's half extents, right, left, top, then bottom. Removal SHALL shift the tail left.

#### Scenario: Overlapping cells
- **WHEN** two cells are at (0, 0) and a point lies inside both
- **THEN** the hit is the higher index

#### Scenario: Top before bottom
- **WHEN** an entry with half extents 40 is dropped at y −60 in a panel 60 tall with a 160 cell
- **THEN** the top lifts it to −40 and the bottom then sets it to −60

### Requirement: Item-panel drop sources
A drop from the linked panel SHALL show its held entry of the item again and a drop from the panel itself SHALL move the entry still marked dragged with the payload's icon, either being raised to the end; any other drop SHALL append the item, adding its bulk unless it came from the linked panel; a payload without an icon SHALL append nothing.

#### Scenario: From the linked panel
- **WHEN** an item the panel does not hold as held is dropped from the linked panel
- **THEN** it is appended and the used bulk does not change

### Requirement: Item-panel accept and positions
The system SHALL reject a drag on the reject mask, the container's own id, a missing model, an oversized item, a capacity overflow (unless from the linked panel or this one), or a required-mask mismatch; `only_own` SHALL require an entry still marked dragged with the payload's icon. List positions SHALL be sixteenths of a unit, y also divided by the vertical scale, and record positions eightieths.

#### Scenario: Capacity and codec
- **WHEN** the capacity is 10, used 6 and the item bulk 5
- **THEN** the drag is rejected, unless it comes from the linked panel
- **WHEN** a position of (140, 20) px is saved with unit 4 and vertical scale 2
- **THEN** the list value is (560, 40) and the record x value 2800

### Requirement: Item-panel position rounding and records
Position saves SHALL round half to even and keep the low 16 bits, and loads SHALL round half to even. A record load SHALL take the container's id; a record SHALL be saved only for a loaded panel showing a database copy.

#### Scenario: Halves round to even
- **WHEN** list values 2 and 6 and record values 10 and 30 are loaded at unit 4
- **THEN** they give 0, 2, 0 and 2

### Requirement: Item-panel mouse intent
The system SHALL map a mouse event to no action, a drag start with the cell-centre grab, use, open or card; the right-button bit SHALL pass to the base form, nothing SHALL happen without the press bit, and a drag start SHALL require drag permission and a hit.

#### Scenario: Drag, use and card
- **WHEN** the 0x4000 and press bits are set with drag permission and a hit
- **THEN** the intent is a drag start with grab (80, 80) for a 160 cell
- **WHEN** the 0x8000 and press bits are set on a container item
- **THEN** the intent is open
- **WHEN** the 0x2000 and press bits are set with a hit
- **THEN** the intent is card
- **WHEN** the 0x4000 bit is set without the press bit
- **THEN** the intent is no action

### Requirement: Item-panel drag end
The system SHALL map drag end to no change, cancel, snap back, transfer to the linked panel, or removal.

#### Scenario: Four endings
- **WHEN** no entry carries the drag flag
- **THEN** nothing happens
- **WHEN** the drag is cancelled
- **THEN** it is cancelled in place
- **WHEN** drag-out is refused
- **THEN** the entry snaps back
- **WHEN** the destination is the linked panel
- **THEN** the entry transfers
- **WHEN** the destination is elsewhere
- **THEN** the entry is removed

### Requirement: Form skin measures
The system SHALL compute `base = 6u−border`, `close = 4u+1`, `rail = 4u−1`, `tab = u/2+close` and the DPI border `max(2, trunc(dpi/135))`.

#### Scenario: Unit five
- **WHEN** the unit is 5 and the border 2
- **THEN** base is 28, close 21, rail 19 and tab 23

### Requirement: Form defaults and anchoring
The system SHALL default a form to 302×402 with border 2, content 300×800, scroll step 0.025 and page minimum 76×90; unset anchors SHALL resolve to (4, 36), centre or far.

#### Scenario: Anchors
- **WHEN** an unset X anchor is default in an 800-wide parent with a 100-wide child
- **THEN** x is 4
- **WHEN** it is centred
- **THEN** x is 350
- **WHEN** it is far
- **THEN** x is 696

### Requirement: Form client rectangle
The system SHALL build the client rectangle from the border, the parent top/base/extra, the rail and the style word.

#### Scenario: Default and inset
- **WHEN** a 302×402 form has border 2 and no style bits
- **THEN** its client is 2,2–300,400
- **WHEN** the style has bit 0x200
- **THEN** the right edge shrinks by the parent base

### Requirement: Form scrolling
The system SHALL set the scroll step to 10/overflow, the thumb to `max(9, trunc(cw/content·track))` capped at the track, and the thumb x to `border+rail+trunc((track−thumb)·f)`, clamping the fraction and snapping or resetting as the style asks.

#### Scenario: A long form
- **WHEN** the width is 404, content 1200, border 2 and rail 19
- **THEN** the step is 10/796, the thumb 120 and the x at f 0 is 21, at f 1 is 263
- **WHEN** the content fits
- **THEN** the fraction resets to 0

### Requirement: Main-menu pages
The system SHALL provide the title, start, in-game, arena-pause, story-pause and class pages with their exact rows and order; CONTINUE SHALL be disabled without a save, VERSUS SHALL appear only when the versus lobby is available, and the class page's EXPERT row SHALL be disabled until the expert difficulty is unlocked.

#### Scenario: Title and start
- **WHEN** the title page is built without a save
- **THEN** its rows are BEGIN, CONTINUE, ARENA, MANUAL, SETTINGS, EXIT and CONTINUE is disabled
- **WHEN** the start page is built with the versus lobby available
- **THEN** VERSUS appears before BACK and the page has five rows
- **WHEN** the class page is built before the unlock
- **THEN** NOVICE is enabled and EXPERT is disabled

### Requirement: Main-menu row origin
The system SHALL give the title page 0.38, the start page 0.45 or 0.42, the pause pages 0.39 and class select 0.45.

#### Scenario: Start page origins
- **WHEN** the start page has four rows
- **THEN** its origin is 0.45
- **WHEN** it has five
- **THEN** its origin is 0.42

### Requirement: Main-menu in-game row
The system SHALL label the in-game page's first row CONTINUE when a game is in progress and NEW GAME otherwise.

#### Scenario: In game and out
- **WHEN** a game is in progress
- **THEN** the first row is CONTINUE
- **WHEN** it is not
- **THEN** the first row is NEW GAME

### Requirement: Control text spans
The system SHALL compute the three spans `w−2pad−2U`, `w−(4U|1)−2pad−U` and `w−(3U|1)−2pad−2U`, keeping the `|1` bit.

#### Scenario: Unit four and five
- **WHEN** w is 100, pad 2 and unit 4
- **THEN** the spans are 88, 75 and 75
- **WHEN** the unit is 5
- **THEN** the third span is 71

### Requirement: Trackbar geometry
The system SHALL map a cursor to `(cursor−x−pad−span/2)/travel` in double precision, set a value below 0 to 0 and above 1 to 1 while a NaN passes both, quantise by the step count with a 64-bit half-to-even rounding (a NaN rounding to the indefinite integer), place the thumb at `x+pad+rint(travel·value)` and centre the caption in the knob slot.

#### Scenario: Half value
- **WHEN** the cursor is 50 in a 100-wide track with pad 2 and span 20 at x 0
- **THEN** the value is 0.5 and the thumb left is 40
- **WHEN** the value is 0.6447 with four steps
- **THEN** the value is 0.75
- **WHEN** the travel is 0 and the cursor is centred
- **THEN** the value is NaN

### Requirement: Control lists
The system SHALL use a `6U` row, a `3U` separator, a clamped 1-based row hit, non-wrapping ±1 navigation, a 16-row extent with `1/(count−16)`, and the `2·inset + widest + 16U` width rule.

#### Scenario: Row and extent
- **WHEN** the unit is 4 and inset 3, a y of 30 with top 3 gives row 2
- **THEN** a 20-item list has height 390, content 480 and page 0.25
- **WHEN** the list has 16 items
- **THEN** it has no page fraction

### Requirement: Control activation
The system SHALL clear `0x100` on every radio item and set it on the chosen one; a checkable item SHALL report the inverse of its checked bit.

#### Scenario: Radio and check
- **WHEN** a radio item is activated
- **THEN** all radio items lose `0x100` and the chosen one gains it
- **WHEN** a checked checkable item is activated
- **THEN** the result is 0, else 1

### Requirement: Main-menu row advance
The system SHALL advance rows by `round(70·row_h)` in single precision, half to even, and start the first row at `round(height·origin)`.

#### Scenario: Unit row height
- **WHEN** row_h is 1.0
- **THEN** the advance is 70
- **WHEN** row_h is 0.25 or 0.75
- **THEN** the advance is 18 or 52
- **WHEN** the height is 1013 and the origin 0.37
- **THEN** the first row starts at 375

### Requirement: Main-menu row hit
The system SHALL hit row `i` when the point lies in `[top + i·advance, top + i·advance + band)`.

#### Scenario: Hit and miss
- **WHEN** the point is 456 with a 70 band
- **THEN** it hits row 0
- **WHEN** the point is 409 or 900
- **THEN** it hits no row

### Requirement: Settings page content
The system SHALL provide the fifteen Settings items: device and memory info, the audio slider, seven quality choices, and the Vertical sync, Show framerate, Window mode, Show gameplay tips and Early Checkpoints toggles, with the toggles sharing rows 10–12.

#### Scenario: Rows and kinds
- **WHEN** the settings are read
- **THEN** there are fifteen items, the audio row is a slider and the quality rows index the choice list
- **WHEN** the toggles are read
- **THEN** Show framerate is on row 10 column 1 and Early Checkpoints on row 12 column 0

### Requirement: Controls page content
The system SHALL provide the fourteen control bindings from Forward/W to Inventory/I.

#### Scenario: Bindings
- **WHEN** the controls are read
- **THEN** Forward is W, Orbit Camera is Mouse 3 and Interact is Control

### Requirement: Settings layout
The system SHALL lay the pages out from the 630×800 retail form: two tabs, the content rectangle, the settings and controls row tops, the label/control/value columns and the three footer buttons, scaling linearly for other form sizes.

#### Scenario: Retail form
- **WHEN** the form is at (500,130) and 630×800
- **THEN** the tabs are at 520 and 808, the first settings row at 260, the last controls row at 698 and the Defaults button at (560,853)

#### Scenario: Half-size form
- **WHEN** the form is 315×400 at the origin
- **THEN** the first tab is (10,23,140,17) and the first settings row is 65

### Requirement: Main-menu selection
The system SHALL keep a row selection per page and clamp a move to the row range without wrapping.

#### Scenario: Ends
- **WHEN** the title page starts at row 0 and moves up
- **THEN** it stays at row 0
- **WHEN** it moves down past the last row
- **THEN** it stops at the last row

### Requirement: Main-menu activation
The system SHALL turn the selected enabled row into its action and return `None` for a disabled row.

#### Scenario: Continue and novice
- **WHEN** CONTINUE is selected without a save
- **THEN** activation returns `None`
- **WHEN** it is selected with a save
- **THEN** activation returns Continue
- **WHEN** NOVICE is selected
- **THEN** activation returns Novice, NOVICE being always enabled
- **WHEN** EXPERT is disabled
- **THEN** activation returns `None`

### Requirement: Settings audio row
The system SHALL show the Music toggle on the audio slider's row, right column.

#### Scenario: Music
- **WHEN** the settings are read
- **THEN** row 3 is Music, a toggle, on row 2 column 1

### Requirement: Controls bindings
The system SHALL provide 22 control bindings, including the eight lower rows Skills, Journal, Talk, Use Power, Power Bar, Previous Power, Next Power and Power Tree.

#### Scenario: Lower rows
- **WHEN** the controls are read
- **THEN** the list has 22 rows and Skills is K, Journal J and Talk T
- **WHEN** Power Tree is read
- **THEN** its default key is empty

### Requirement: Settings record anchors
The system SHALL resolve a record's y anchor by adding its delta to the previous row's grid y, negated when style bit 0x80000 is set, and its x anchor from the previous row's right edge.

#### Scenario: Anchors
- **WHEN** the y code is 0x8006
- **THEN** the delta is 9 grid units
- **WHEN** the x code is 0x8002
- **THEN** the gap after the previous row is 72 px
- **WHEN** a record's size is zero
- **THEN** the class skin size is used

### Requirement: Settings records
The system SHALL provide the 9 tab/button, 27 settings and 22 binding records and lay them out at unit 4 on a 434×536 form with a 408-px settings list and a 416-px controls view.

#### Scenario: Unit four
- **WHEN** the form is laid out at unit 4
- **THEN** the first tab is (12,20,192,32), the first settings row is at y 64 and the first binding at y 64
- **WHEN** the unit is 6
- **THEN** the form is 651×804 and the first tab is (18,30,288,48)

### Requirement: Settings page switch
The system SHALL show Settings at (0,64,426,408) on page 0 and Controls at y 56 (widescreen) or 96 with height 472−y on page 1, scaling by unit/4.

#### Scenario: The two pages
- **WHEN** page 0 is switched at unit 4
- **THEN** Settings is visible and its child is (0,64,426,408)
- **WHEN** page 1 is switched widescreen at unit 4
- **THEN** Controls is visible at (0,56,426,416)
- **WHEN** page 1 is switched non-widescreen
- **THEN** the child is (0,96,426,376)

### Requirement: Controls scrollbar
The system SHALL size the Controls thumb to 232 px and place it at `17 + 150·f` over the 412-px client and 676-px content at unit 4.

#### Scenario: Ends
- **WHEN** the fraction is 0
- **THEN** the thumb is 232 at y 17
- **WHEN** the fraction is 1
- **THEN** the thumb is at y 167

### Requirement: Main-menu row scale
The system SHALL derive `row_h` as `round(H/135)·0.125` and advance rows by `round(70·row_h)`.

#### Scenario: Live heights
- **WHEN** the client height is 1080
- **THEN** `row_h` is 1.0 and the advance 70
- **WHEN** the height is 453
- **THEN** `row_h` is 0.375 and the advance 26
- **WHEN** the height is 720
- **THEN** `row_h` is 0.625

### Requirement: Main-menu paint
The system SHALL start row baselines at `round(H·origin) + ascent`, advance by `round(70·row_h)`, place a row at `(W − width) >> 1` as an unsigned 64-bit shift, draw disabled rows grey at half alpha, and append `[` to a row with one mark and `[[` to a row with more.

#### Scenario: Title paint
- **WHEN** the title paints at 1600×1080 with origin 0.38, row_h 1 and ascent 49 with BEGIN selected
- **THEN** the first unselected row is at y 529 and BEGIN is drawn last at 459
- **WHEN** CONTINUE is disabled with one mark
- **THEN** its colour is (128,128,128,128) and its text is CONTINUE[
- **WHEN** a row is 102 wide in a 100-wide client
- **THEN** its x lands far past the client

### Requirement: Selected row last
The system SHALL draw the selected row last, in white whatever its state, and nudge it down by unit/3 while it is the pressed row; with no selection every row SHALL be drawn in order.

#### Scenario: Last row
- **WHEN** the last of three rows is selected and pressed with unit 6
- **THEN** it is drawn last at its row y plus 2
- **WHEN** the second of three rows is selected and pressed with unit 6
- **THEN** it is drawn last at its row y plus 2
- **WHEN** it is selected but not pressed
- **THEN** it is drawn at its row y

### Requirement: Control label horizontal alignment
The system SHALL align a label by the style's horizontal field: 1 left of the control, 3 centred in the available width, 4 ending before its right edge, 5 after it; any other value, 2 included, SHALL take the default half-unit inset.

#### Scenario: Four styles
- **WHEN** x is 100, the text width 20, the unit 4 and the style 0x1000
- **THEN** the label x is 76
- **WHEN** the style is 0x2000 with pad 2 and available width 100
- **THEN** the label x is 104
- **WHEN** the style is 0x3000
- **THEN** the label x is 145
- **WHEN** the style is 0x4000
- **THEN** the label x is 184
- **WHEN** the style is 0x5000
- **THEN** the label x is 204

### Requirement: Control label vertical alignment
The system SHALL lift by half a unit (1), centre by the line height with a shift halving (3), base-align (4), place below (5), or use the half-unit default for any other value, 2 included.

#### Scenario: Vertical styles
- **WHEN** y is 100, the unit 4 and the style 0x100
- **THEN** the label y is 98
- **WHEN** the style is 0x200
- **THEN** the label y is 102
- **WHEN** the style is 0x300 with height 20 and line height 16
- **THEN** the label y is 119
- **WHEN** the style is 0x400 with base 14
- **THEN** the label y is 133

### Requirement: GUI fonts
The system SHALL lay interface text out with the original's six GUI fonts (`Rayform06R`, `Rayform08R`, `Rayform10R`, `Rayform12R`, `Rayform16R`, `fontbase24r`), each with its recorded glyph quad, centring height, slant, gap, line height and per-byte advances (the stored widths plus the gap).

#### Scenario: Recorded advances
- **WHEN** `fontbase24r` measures a space, "A", "i" and "m"
- **THEN** they advance 21, 28, 12 and 37, its quad is 36 × 48 and its line 48
- **WHEN** `Rayform06R` measures "lW"
- **THEN** the width is 10

### Requirement: GUI scale levels
A GUI scale level, clamped to 0 through 5, SHALL set the unit (3, 4, 5, 6, 8, 12), the main font (06R, 08R, 10R, 12R, 16R, fontbase24r), the small font (06R at the two lowest levels, else one size down) and the cells of the large and small icon atlases (16, 20, 28, 32, 42, 64 and 12, 16, 20, 24, 32, 48).

#### Scenario: Level 2
- **WHEN** the level is 2
- **THEN** the unit is 5, the fonts are `Rayform10R` and `Rayform08R`, and the cells are 28 and 20

### Requirement: Text line breaks
A line SHALL end after a carriage return, after the last space that fits the width, or, with no space, after the byte that overflows it; a following line feed SHALL be skipped. A text under two bytes SHALL count one line, and a text whose second-to-last byte is a carriage return one more.

#### Scenario: Breaking
- **WHEN** every byte is 10 wide and a space 5, and "ab cd" wraps at 30
- **THEN** the first line ends after the space
- **WHEN** "abcdef" wraps at 30
- **THEN** the first line ends after "d"

### Requirement: Text markup
With markup, a `[` SHALL open a tag that takes no width: `[tcol=RRGGBB]` pushes a colour (at the skin's tag alpha; eight hex digits give their own), `[/tcol]` pops it, `[scrp=i]` slants the glyphs by the main font's slant, and `[algn=2]` moves its line right by half the width left. A tag closed by the text's last byte SHALL lose its last letter, as in the original.

#### Scenario: Colour and centring
- **WHEN** "a[tcol=ff0000]b[/tcol]c" is drawn from x 7 with 10-pixel bytes
- **THEN** "b" is at 17 in red at the tag alpha and "c" at 27 in the text colour
- **WHEN** "[algn=2]ab" is drawn 100 wide
- **THEN** "a" is at 40

### Requirement: Text pictures and spans
`[rimg=NAME]` SHALL draw a picture its size over 6 in units, centred in the frame, moving the text below it down by its height less a pitch. `{%` SHALL start a span slanted by the main font's slant in (180, 200, 240) that ends after `%}`.

#### Scenario: A picture
- **WHEN** a 60 × 60 picture is drawn at unit 5 in a 100-pixel frame
- **THEN** it is 50 × 50 at x 25, and the next line starts 50 pixels down

### Requirement: Key names in text
`[inptN]` (one or two digits) SHALL be replaced by the name of the key bound to control N in title case, underscores turned to spaces.

#### Scenario: A binding
- **WHEN** control 3 is bound to "SPACE"
- **THEN** "Press [inpt3] now" reads "Press Space now"

### Requirement: Caret moves in text
Home and End SHALL go to the wrapped line's start and its end before the break; Up, Down and the page keys SHALL keep the pen offset, picking the nearer side of the byte under it, and scroll the target line in as the first line or the second-to-last; the pointer SHALL place the caret before the first byte whose middle is right of it, its line being y over the font's line height.

#### Scenario: Down keeps the pen
- **WHEN** "abc def ghi" wraps at 40 and the caret is before "e"
- **THEN** Down puts it before "h"

### Requirement: Text box editing
The text box SHALL insert typed bytes at the caret (replacing a selection), insert `\r\n` on Enter, and delete a `\r\n` as one with Backspace or Delete; Left SHALL pass a `\r\n` from after it, and Right from before it SHALL stop between the two; Shift SHALL keep the anchor; a read-only box SHALL take no keys. Enter over a selection SHALL grow the text by the length before the cut, as the original does.

#### Scenario: A line break
- **WHEN** "ab", Enter and "cd" are typed
- **THEN** the text is "ab\r\ncd" in two lines, and Backspace after the break removes both bytes

#### Scenario: Enter over a selection
- **WHEN** "abcdef" has "bc" selected and Enter is pressed
- **THEN** the text is 8 bytes and starts "a\r\ndef"

### Requirement: Text box clipboard and Tab
Ctrl+C, Ctrl+X and Ctrl+V SHALL copy, cut and paste the selection; Ctrl+Home and Ctrl+End SHALL go to the text's ends and scroll there; Tab SHALL reach the owner only when it takes Tab.

#### Scenario: Cut and paste
- **WHEN** "world" of "hello world" is cut and pasted at the start
- **THEN** the text is "worldhello " with the caret after "world"

### Requirement: Text box scroll
The text box SHALL count its lines at its width less a unit and its margin each side (200 units when it sizes itself), pitch them at its spacing in units plus the font's line height, scroll over the lines that do not fit in whole-line steps, snap a set scroll to steps, ease the drawn scroll a fifth of the way per frame (snapping within 0.001), and, while dragging, scroll at most every 180 ms. A box that grows SHALL take the content height plus its top and left insets.

#### Scenario: Five lines in three
- **WHEN** a 60-pixel box with `Rayform10R` holds five lines
- **THEN** the content is 100 pixels and a step is half the scroll

### Requirement: Icon atlas
An icon atlas SHALL put an icon in its first free cell; a full atlas SHALL double its shorter side, its height first, a wider atlas repacking its cells in order; the first icon SHALL open a square the next power of two of the cell; a handle SHALL be the cell's index plus one.

#### Scenario: Growth
- **WHEN** a 16-pixel atlas takes three icons
- **THEN** it is 16 × 16, then 16 × 32, then 32 × 32 with the second icon moved to the first row

### Requirement: Icon draws
Icons SHALL draw as one cell-sized quad, plain, turned by the recorded turn table, mirrored, outlined (the icon in the face colour, then four outline copies a pixel left, right, below and above) or glowing (four copies two pixels out at half alpha, then the icon in white).

#### Scenario: An outline
- **WHEN** an icon is outlined at (5, 6) at alpha 0.5
- **THEN** five quads draw: the icon, then copies at x 4 and 6 and at y 5 and 7, their alphas halved

### Requirement: Built-in glyphs
The interface SHALL draw twelve glyphs into its small atlas at the GUI unit, in order: a cross (or the pack's symbol), right, left, up and down chevrons, three window frames, minus and plus boxes, a drop-down chevron over a bar and a tick; strokes are bright with dimmer fringes, a fringe over a fringe halving the first and adding its own.

#### Scenario: The minus box at unit 5
- **WHEN** the minus glyph is drawn into a 20-pixel cell at unit 5
- **THEN** its bright bar is pixels 4 through 10 of row 12 from the bottom

### Requirement: Icon hits
A colour icon SHALL be hit by its alpha: at a point, its alpha over 255, the row read being the cell height less y up from the cell's bottom; near a point, any alpha within a unit of it, the square cut to the cell at its low edges and at its high edges only when the low edge is inside.

#### Scenario: One opaque pixel
- **WHEN** a 16-pixel icon has one opaque pixel at column 4, row 10 from the bottom
- **THEN** the alpha at (4, 6) is 1 and a press within 2 pixels of (6, 8) hits it

### Requirement: Table view rows
A table SHALL draw rows 6 units tall under a 6-unit header, scroll over all its rows but one, and not hit the header or the rows behind it. Down SHALL pass greyed rows and Up SHALL not. Rows SHALL keep the order they were added in.

#### Scenario: Rows and hits
- **WHEN** a unit-5 table 200 pixels tall has five rows
- **THEN** the content is 320 pixels, y 29 hits nothing and y 30 hits row 1

#### Scenario: Down over a greyed row
- **WHEN** row 2 is greyed and row 1 is selected
- **THEN** Down selects row 3 and Up then selects row 2

### Requirement: Table view presses
A press SHALL select a row that is not greyed; a second press SHALL edit an editable cell (all of it selected) or open the column's choice list; a double press SHALL activate the row; a press past the last row of a growing table SHALL add a row and edit it; committing an emptied first cell of the last row SHALL delete that row.

#### Scenario: Select then edit
- **WHEN** row 2 is pressed twice on its editable first cell
- **THEN** it is selected, then its text is edited with all of it selected

### Requirement: Item view layout
An item view SHALL lay its entries out as a grid (pitch the icon plus 4 units, rows of 6 units more and a line, names cut under the icon to one byte fewer than fit at the gap, with an ellipsis), a list (one per row of the icon plus a unit) or columns (the icon plus 2 units apart) that scroll sideways, and SHALL render one missing visible icon per request.

#### Scenario: A grid
- **WHEN** a unit-5 grid 200 pixels wide holds seven entries
- **THEN** icons are 20, the pitch 40, rows 70 and four entries share a line

### Requirement: Item view input
An item view SHALL rename on a second press, activate on a double press, toggle marks in check mode (on the selected entry setting every other flag bit, as the original), and jump to the next entry whose name starts, case-blind, with the letters typed within 600 ms.

#### Scenario: Type-ahead
- **WHEN** "S" is typed and then "h" within 600 ms
- **THEN** the selection goes to "Sword" and then to "shield"

### Requirement: Basic controls
The interface SHALL know the seven control classes (button 80 × 24, check box 16 × 16, image button 28 × 28, trackbar 16 tall, label 20 tall, icon button and picture 28 × 28) and answer their messages: hover lights a control that is not on, a release over it toggles it (a radio group keeps one on; a three-state box cycles off, on, partly on), and a trackbar takes its value from the pointer past its margin and half its knob over its travel, quantised to its steps, or steps by keys and the stick.

#### Scenario: A trackbar
- **WHEN** a 100-pixel trackbar at x 10 has its 22-pixel knob and 2-pixel margin
- **THEN** the pointer at 60 gives 37 / 74

### Requirement: GUI level in the app
The app SHALL keep the GUI level with the GUI scale; a change SHALL empty both icon atlases, give them the level's cells and draw the glyphs again, and the glyph atlas SHALL be kept as an upright texture.

#### Scenario: A larger window
- **WHEN** the scale goes from 5 to 8
- **THEN** the atlases' cells become 42 and 32 and the glyphs are drawn again at unit 8

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

### Requirement: Match board selected match
The arena match board SHALL build its lineup for one selected match record
copied by the open tab: a standard match on the standard tab, the selected
daily event on the events tab and the selected tournament event on the
tournament tab. The record's lowest rank SHALL be the rank floor, its side
size SHALL cap the lineup, its kind SHALL select the Captain rule, flag 1
SHALL lift the one-leader and marked-fighter rules and flag 4 SHALL mark a
weapon event.

#### Scenario: A daily Captain event drives the lineup
- **WHEN** the events tab selects a Captain event with floor 3 and side
  size 2, and the company's ranks are 2, 1, 0 and 4 with the last a leader
  and the player at rank 3
- **THEN** the lineup holds only the rank-4 leader, because the first
  fighter needs the full floor and the player would be a second leader

### Requirement: Match board standard matches
The standard tab SHALL list Duel, Doubles, Skirmish, Fray, Reserve and
Pugilism with side sizes 1, 2, 3, 1, 3 and 1. It SHALL multiply the
match's fee and prize by the rank plus one through the original's coin
packing, keep the tier at or below the rank, move the tier to the rank's
row on a rank change, grey the tier rows above the rank, and store no tier
for Pugilism.

#### Scenario: A Duel at Novice
- **WHEN** the Duel is selected at rank Novice with tier Master
- **THEN** the tier becomes Novice, the rows Adept, Expert and Master are
  greyed, the floor is Novice, the fee is 9 copper and the prize 3 silver

#### Scenario: Pugilism at Expert
- **WHEN** Pugilism is selected and the rank changes to Expert
- **THEN** no tier is stored, the fee is 5 copper and the prize 2 silver
  and 6 copper

### Requirement: Match board lineup rebuild
The lineup SHALL be rebuilt by walking the lineup order and adding each
combatant whose rank passes the floor until the side size, with at most one
leader, the player counting as one, unless flag 1 is set.

#### Scenario: The player is the second leader
- **WHEN** a six-a-side match with floor Inept rebuilds over ranks 2, 1
  (leader), 0 and 4 (leader), with the player last in the order
- **THEN** the lineup is the first three and the player stays in the
  company table

### Requirement: Match board fighter moves
A double click SHALL add a fighter when there is room and otherwise bench
the last teammate to the end of the company table; a drop on a row SHALL
swap with it; a marked fighter SHALL swap with the first marked teammate. The
fighter's place in the lineup order SHALL become the row it joined, or the
first place after a marked swap or a bench. A fighter moved back SHALL be
inserted in pool-slot order.

#### Scenario: A bench puts the newcomer first in the order
- **WHEN** a two-a-side lineup of slots 1 and 2 is full and the player is
  double-clicked
- **THEN** slot 2 is benched to the end of the company table, the lineup is
  1 and the player, and the next rebuild puts the player first

### Requirement: Match board Captain moves
On a Captain event with more than two teammates of whom fewer than two
reach the floor, a fighter under the floor SHALL NOT replace one who does,
and after a move a qualified teammate SHALL be moved first.

#### Scenario: An unranked Captain newcomer
- **WHEN** a Captain lineup of ranks 1, 2 and 4 with floor 3 receives a
  rank-0 fighter dropped on the rank-4 row
- **THEN** the drop is refused, and a drop on the rank-1 row is taken and
  moves the rank-4 teammate first

### Requirement: Match board tournament status
The tournament tab SHALL show the six events' winners, reading a winner of
1 as the player's company, below 1 as a dash, and from 2 through the
tournament's company table. Its status SHALL say "The tournament has
ended." when every entered event is settled and the last event has a
winner, and "No more events entered." when it has none. On a tournament day
with entries the board SHALL switch to this tab and lock fighter assignment.

#### Scenario: The last event decides the end
- **WHEN** the only entered event has a winner and the last event has none
- **THEN** the status reads "No more events entered.", and once the last
  event has a winner it reads "The tournament has ended."

### Requirement: Match board weapon event
A weapon event SHALL show the weapon row's name on the reward panel,
shorten the lineup to 120 pixels and mark the panel when the first entrant
does not carry one of the row's weapons.

#### Scenario: The weapon check
- **WHEN** the selected tournament event names the row "Mace" with weapons
  7 and 9 and its first entrant carries 9, then 8
- **THEN** the panel reads "Mace" and is unmarked, then marked

### Requirement: Match board day controls
The standard and events refreshes SHALL write the no-more-matches message
once three matches were entered that day and clear it otherwise, and the
tournament refresh SHALL clear it. After each rebuild or move the primary
button SHALL be disabled once three matches were entered or while the
lineup is short, and hidden on the season tab. A zero coin cell SHALL be
drawn as a dash four pixels right.

#### Scenario: The day's matches are spent
- **WHEN** three matches were entered and the standard tab refreshes
- **THEN** the message box shows the no-more-matches text and the primary
  button is disabled

### Requirement: Skin scheme shades
A scheme colour SHALL expand into twelve shades: the base at 4; the base times the scale, its square and its cube at 5 to 7; at 8 to 11 the base toned by 1.75, 2, 2.25 and 2.5 times the tone (at most 1) and scaled by the fourth to seventh powers; at 3 down to 0 the base over the scale, divided by the scale again, then twice more with a 10/11 step each; and last entries 4 to 7 toned by 0.5, 1, 1.25 and 1.5 times the tone.

#### Scenario: A gentle scale
- **WHEN** 60, 90, 120 expands with scale 1.25 and tone 0.4
- **THEN** shades 0 to 3 are 20, 30, 41; 28, 42, 56; 38, 58, 77 and 48, 72, 96
- **AND** shades 4 to 8 are 65, 89, 113; 87, 109, 132; 113, 137, 160; 146, 169, 193 and 188, 210, 232

### Requirement: Skin tone and scale rounding
Toning SHALL pull each channel towards the luminance (green × 0.587 plus red × 0.299, then blue × 0.114) and keep the low byte of the rounded result; scaling SHALL round to nearest even and saturate; scaled shades SHALL keep the destination's alpha.

#### Scenario: A tone past one wraps
- **WHEN** 200, 0, 100 is toned by 2
- **THEN** it becomes 198, 142, 42

### Requirement: Skin accent shades
The scheme's accent SHALL give a dim shade (half) and a bright one (1.9, then toned by 0.8), both with the second alpha byte times the scheme's float as their alpha, and four mixes at 0.4, 0.6, 0.5 and 0.9 towards the bright one; the last two SHALL be blended from the form group's fourth shade by that product over 255.

#### Scenario: The accent alpha wraps
- **WHEN** the accent is 200, 100, 50 with scale 1.5 and second alpha 200
- **THEN** the dim shade is 100, 50, 25, 44 and the bright one 210, 197, 178, 44
- **AND** the last two mixes have alpha 38

### Requirement: Skin alpha ramps
The first group SHALL be copied to the hot group. The alpha ramps SHALL give the form group and the last two mixes the first byte times its scale, and all forty-eight shades of the control, hot, label and highlight groups the second byte times its scale.

#### Scenario: The last highlight shade ramps too
- **WHEN** the second alpha 160 is scaled by 0.25
- **THEN** every highlight shade, the twelfth included, has alpha 40

### Requirement: Skin batch
Selecting the mode already selected SHALL bind nothing; mode 0x10 SHALL ask the subclass and keep its answer as the selected mode; an unnamed mode SHALL be stored and bind nothing. A run's end SHALL submit its vertices as quads, lines or triangles with the state then current, into one vertex list that a cleared batch reuses.

#### Scenario: Selecting twice
- **WHEN** mode 2 is selected with the tint and again without it
- **THEN** the texture tint stays 1

#### Scenario: Reuse
- **WHEN** a batch is cleared and refilled
- **THEN** its vertex list keeps its capacity

### Requirement: Skin selection band
A selection band SHALL draw an outline whose middle is half the width less one in, then glow fans reaching into the row from its top and bottom edges, wider when selected.

#### Scenario: The lower glow
- **WHEN** a band spans 10, 20 to 110, 100 at unit 4, unselected
- **THEN** the outline's middle is at x 59.5
- **AND** the lower fan reaches from y 99 up to 94 at the middle and 95 at the sides

### Requirement: Skin line boxes
Double frames SHALL start each top edge a pixel short of the right; bevel and gradient line boxes SHALL meet their left mixes half a pixel below the right ones, the bevel's right side ending on its top edge; two-ink bands SHALL be two lines; four-ink frames SHALL fade their sides from the edge inks.

#### Scenario: A bevel box
- **WHEN** a bevel box spans 10, 20 to 30, 40
- **THEN** the left mixes meet at 30.5, the right ones at 30, and the right side ends at 29.5, 20.5

### Requirement: Skin gradients and shadows
A clamped gradient frame SHALL clamp its bands on the rectangle before its lines move in; alpha-scaled gradients SHALL keep the low byte of the scaled alphas; a shadow SHALL halve its height as an unsigned word.

#### Scenario: A gradient frame
- **WHEN** a gradient frame spans 10, 20 to 110, 100 at unit 4
- **THEN** its left bands meet at 10.5, 28 and 10.5, 92

### Requirement: Skin scroll arrows
A scroll arrow SHALL draw six line pairs in a look's inks: look −1 the label group's second shade, look 0 its fifth, look 1 the highlight group's ninth under the additive blend; the copies after the fourth vertex SHALL take the shade before at two thirds of its alpha. Other looks SHALL draw nothing.

#### Scenario: A horizontal arrow
- **WHEN** an arrow of width 9 is drawn at 10, 20 with look 0
- **THEN** its first four vertices are 10, 25; 14, 21; 19, 25 and 14.5, 20
- **AND** a named shade of alpha 200 gives the copies alpha 132

### Requirement: Skin control faces
A face SHALL draw its two-ink rectangle, bordered box and outline in the pressed, hot or idle shades; the gradient face's hot bevel SHALL run from the hot group's fifth to its sixth shade; the pressed panel's border SHALL use control shades 5 and 8.

#### Scenario: An idle face
- **WHEN** an idle face is drawn
- **THEN** its rectangle runs from control shade 5 above to 3 below and its border from 8 outside to 6 inside

### Requirement: Skin sliders and scroll bars
A slider's track SHALL fade the hot group's fifth and first shades at 0.6 alpha and its filled bar SHALL be one unit wide; a scroll bar SHALL be drawn a pixel lower and two shorter, its striped arm at style 0x1000_0000.

#### Scenario: A scroll bar
- **WHEN** a scroll bar spans 0, 0 to 100, 24 at unit 4
- **THEN** its trough is 1, 2 to 99, 22 and its thumb 83, 3 to 98, 21

#### Scenario: A filled slider
- **WHEN** a filled slider of width 100, inset 4 and unit 6 is at one half
- **THEN** the bar's shift is 43

### Requirement: Form marks
A form mark pressed under the held button SHALL draw a bevel just outside its rectangle, the outline and a face a pixel in fading from the form group's fourth shade up to its third; the mark under the mouse SHALL draw that bevel, a second one a pixel inside and the outline.

#### Scenario: A hovered mark
- **WHEN** a mark is under the mouse and not pressed
- **THEN** it draws two bevels and an outline and no face

### Requirement: Form mark icons
A mark's icon SHALL sit half a unit in from the left and half a unit and one below the top, at scale 1 when the mark is under the mouse or pressed with the button up, otherwise in the label group's seventh shade.

#### Scenario: A close mark
- **WHEN** a 200-wide titled form with a close mark is drawn at unit 4
- **THEN** the mark spans 179, 3 to 196, 23 and its icon is at 181, 6

### Requirement: Form chrome
The chrome SHALL draw the gutter strips (none above a titled form), the client fill when the form keeps no lines, the first style field's double frame or (style bit 0x40) an outline a pixel in, and the second field's double frame two pixels outside the client; a flat form SHALL draw only its frame.

#### Scenario: The client frame
- **WHEN** the client spans 20, 30 to 180, 80 and the second field is set
- **THEN** a double frame runs from 18, 28 to 182, 82

### Requirement: Form background scrollbars
The default background's vertical scrollbar SHALL draw its frame, shadows, arrows −9 and −10 as frame slots pressed when held and one gradient bar for the thumb with the texture shifted by its place; the horizontal one SHALL draw arrows −13 and −14 and a frame-slot thumb with the texture shifted.

#### Scenario: The vertical scrollbar
- **WHEN** the thumb is at 30 with length 20 on a 100-high form with client top 24
- **THEN** the arrow frames are 180, 24 to 195, 39 and 180, 80 to 195, 95 and the thumb 180, 30 to 195, 50
- **AND** one shadow is 180, 50 of size 15 by 11

### Requirement: Form background title
The default background SHALL draw the rounded shell, the title gradient, the optional two-line band and the marks before the scrollbars, then the arrow icons, the mark icons and the title's two blocks at y `(margin + line height + 25) / 2`.

#### Scenario: The title
- **WHEN** a 200-wide form with margin 4, inset 5, a close mark and line height 13 is drawn at unit 4
- **THEN** the title blocks are 165 wide at 8, 21 in the form's second shade and 9, 22 in the label's seventh

### Requirement: Form page order
A page SHALL draw its background, stop when its state hides it, clip to its client, translate its contents to the inset and top less the scroll (zero when locked), then draw the line rectangles, the controls, the visible children and last the drop shadow, which the lock does not remove.

#### Scenario: The origin
- **WHEN** a page with inset 5, top 24, right 200 and bottom 130 has no stored scroll
- **THEN** the scroll is −195, −106 and the origin 200, 130
- **AND** locked, the origin is 5, 24

### Requirement: Skin label inks
A label SHALL draw in the label group's third shade when focused and its seventh otherwise, the slot's idle label in its ninth; the slot's pressed label SHALL draw control shade 2 one pixel left and label shade 7 one below; the block form SHALL keep the bytes that fit, add the offset and right-align for style 0x4000.

#### Scenario: A block label
- **WHEN** a 100-wide block label with inset 2 and step 7 is drawn
- **THEN** it keeps 13 bytes

### Requirement: Skin glowing text
A coloured label SHALL draw its text in the label group's eighth shade at the measured place, then four copies a pixel left, right, below and above in the highlight group's first shade under the additive blend.

#### Scenario: The glow
- **WHEN** coloured text is drawn at 10, 20
- **THEN** the copies are at 9, 20; 11, 20; 10, 21 and 10, 19

### Requirement: Skin label fade
A fade SHALL take a zero start as now and scale the label group's seventh shade's alpha by the elapsed third of a second, keeping the low byte, and clear the fade bit once it passes 1.

#### Scenario: A fade
- **WHEN** a label started at 1000 is drawn at 1150
- **THEN** its alpha is half of the shade's
- **AND** at 1400 the fade bit is cleared

### Requirement: Technique pick
The technique selector SHALL put a picked technique into its skill slot of the character record's six skills: an empty slot first forgets the technique in training (the first skill whose slot past its level holds one); with a positive pool a slot nothing was picked into before gains the unspent experience held to 0…5 000, or under the weighted rule `pool · 4 / (4 + Σ levels)` capped at 5 000 while the pool loses `gain · divisor / 4`.

#### Scenario: A first pick
- **WHEN** skill 1 holds 15 000, skill 2 10 000, the pool is 40 000 and a technique is picked into skill 1's empty third slot
- **THEN** skill 1 holds 20 000 and the pool stays 40 000

#### Scenario: The weighted rule
- **WHEN** the same record picks under the weighted rule into skill 2's first slot
- **THEN** skill 2 holds 15 000 and the pool is 28 750

### Requirement: Technique forget
Forgetting the technique in a slot followed by filled slots SHALL close the slots up, forget every technique among the first four that names a forgotten one, and take 5 000 from the skill for each; forgetting the last filled slot SHALL clear it and set the skill to one level less than the techniques it held. Outside the weighted rule a positive pool then refunds into the technique in training, at most to its level.

#### Scenario: A dependent chain
- **WHEN** skill 3 holds 20 000 with techniques 1, 2, 3, 4 and its first slot is forgotten
- **THEN** it holds technique 4 alone and 5 000

### Requirement: Technique selector keys
The technique selector SHALL pass a key to the base form only while a technique is picked in the confirmation mode or while row code 9 is hovered with the lock byte clear, and close with 0x40 otherwise.

#### Scenario: A plain key
- **WHEN** technique 3 is picked in the list mode and a key arrives
- **THEN** the form closes with 0x40

### Requirement: Compact skill hit
The compact skill screen SHALL hit-test a point against each row whose line plus six units is not above it, take its first cell strictly within five units, let a later row override, and return `row << 8 | column` with both counted from 1.

#### Scenario: The first cell
- **WHEN** the margin is 4 units, the spacing 24, the unit 4 and the point is (24, 80)
- **THEN** the hit is 0x101

### Requirement: Compact skill wheel
The compact skill screen's wheel SHALL take, for a mostly horizontal wheel, the row whose line is nearest the pointer and move one column that way, and for a vertical one the row below the pointer (wheel down) or above it (wheel up), skipping rows without slots.

#### Scenario: Wheel down
- **WHEN** the pointer is on row 1's line, row 2 has no slots and the wheel moves down
- **THEN** row 3 is taken

### Requirement: Loadout cost and equip
The loadout editor SHALL cost a weapon at its points less what it displaces from the hands, and equip an item only when a copy is free of the hands, its rank is at most the form's rank (5 when unset) and the points used plus its cost are within the rank's limit; a champion takes any item.

#### Scenario: Beside a sword
- **WHEN** the first hand holds a one-handed weapon of 4 points, the second a shield of 3, and an off-hand-capable one-handed weapon of 5 is dropped
- **THEN** it costs 2

### Requirement: Loadout layout
The loadout editor SHALL lay the arsenal 228 pixels wide at (4, 180), `492 · opacity` tall rounded half to even (the opacity 0.83739835 on a wide interface), and the equipment 236 pixels wide at y 88 four pixels from its parent's right edge, 628 tall or 548 on a wide interface.

#### Scenario: A wide interface
- **WHEN** the interface is wide
- **THEN** the arsenal is 412 pixels tall

### Requirement: Divination reading
The divination reading SHALL darken the screen to alpha 128 over a second, deal its three cards over 600 ms each rising by the ease `(3 − 2t)·t²`, turn them over 900 ms each from 1.8 s by half a turn eased the same way, and hold them face up from 4.5 s, sounding `PprMov` at each deal step and at the first three turns.

#### Scenario: Half way through a deal
- **WHEN** 300 ms have passed at a unit of 6
- **THEN** the first card is at y 50

### Requirement: Destination ring
The destination screen SHALL place row `i > 0` at the angle `(1 − turn) + (2.5 − i)·2π/(count − 1)`, `unit · sin · 40` across and `unit · cos · 40` down from the centre (each rounded half to even, then scaled by the turn), reveal over 500 ms with the alpha eased once and the turn twice by `x(2 − x)`, and hit only the three orbit nodes within 14 units.

#### Scenario: The first orbit node
- **WHEN** four rows sit on a 200 × 100 screen at unit 4, fully revealed
- **THEN** row 1 is at (100, −110)

### Requirement: Skill learning row requirements
The learning row's mask SHALL mark the nodes of the five written techniques chosen, the slots the tree has no node for absent (over a chosen mark), then every still-clear slot missing when either of its two prerequisites is not among the techniques the points fund, read through the record's byte layout.

#### Scenario: Close Combat's mask
- **WHEN** Close Combat has 35 000 points and Remise, Feint, Fend and Riposte
- **THEN** the mask is chosen for the first four, absent for the fifth to seventh and clear for Dual Wield
- **WHEN** Riposte is not chosen
- **THEN** Dual Wield is missing
- **WHEN** Ranged Combat is asked
- **THEN** every slot is absent

### Requirement: Skill node press state
A ring node SHALL be learnt when the points fund its level; the next level SHALL be open while a level is ready and its technique byte is clear, and, with a technique chosen there ahead of the points, learnt once the skill is more than ten points past its funded levels, else open; any other level SHALL be locked.

#### Scenario: One funded level
- **WHEN** Close Combat has 5000 points and Remise
- **THEN** level 1 is learnt, level 2 is open with a ready level and locked without, and level 3 is locked
- **WHEN** Feint is chosen ahead at 5010 points
- **THEN** level 2 is open, and learnt at 5011 points

### Requirement: Skill unlearning
Forgetting a level SHALL move the points by where the level sits, in 16-bit arithmetic: at or below the committed levels both the committed and the record's points lose 5000; one past them the committed points round down to their levels and the record's lose 5000 while it holds more levels, else take the committed points; further out the record's lose 5000 when they fund the level, else round down.

#### Scenario: A wrapped count
- **WHEN** the committed points are 5000, the record's 0 and level 1 is forgotten
- **THEN** the record holds 60 536 points

### Requirement: Skill unlearning cascade
The forgotten level's technique SHALL leave, the later written slots move down, and from the last funded level down every technique listing the removed one as a prerequisite SHALL be unlearned in turn.

#### Scenario: Armour's chain
- **WHEN** Armour holds I, II and III at 15 000 committed points and I is forgotten
- **THEN** II and III go with it and both point counts are 0

### Requirement: Skill learning commit
A press on a ring node with nothing pending SHALL open the forget dialog for a learnt node or the learning row (with the group's mask) for an open one, starting a 300 ms popup opening. A second press SHALL, in the forget dialog, unlearn on Yes; in the learning row, on a slot with a clear mask byte, unlearn the record's surplus choice and write that slot's technique into the pending level. Either SHALL refresh the ready flag and start the popup's 200 ms close unless it is fading.

#### Scenario: Riposte at level two
- **WHEN** Close Combat has 5000 points and Remise, level 2 is pressed and then slot 4
- **THEN** the techniques are Remise and Riposte
- **WHEN** slot 8 is pressed instead
- **THEN** nothing is written, Dual Wield's prerequisites being missing

### Requirement: Skill experience pass
With global experience the screen SHALL spend its worth: the last group with a partial committed level first, at the missing points times a weight of four plus the committed levels, quartered toward zero, or the budget's share `budget × 4 / weight` when short; then each level between a group's committed and record levels at `weight × 5000 / 4`, the weight rising after each; and the rest, compared unsigned, on the record's surplus choice, a full level or the share on top of the level below.

#### Scenario: A surplus choice takes the rest
- **WHEN** Close Combat's committed and record points are 2500 with Remise and Feint chosen and the worth is 5000
- **THEN** the record holds 7000 points and nothing is left

### Requirement: Skill ready flag
A locked screen SHALL never be ready. While experience is left over a level SHALL be ready. Otherwise the scan SHALL stop at the first skill whose next technique byte is set, which must be no more than ten points past its levels, and the levels counted to there SHALL stay below fifteen.

#### Scenario: The scan stops early
- **WHEN** every skill has 15 000 points and two techniques
- **THEN** no level is ready
- **WHEN** Close Combat instead has 10 000 points with a third technique chosen
- **THEN** a level is ready

### Requirement: Skill group icons and orbits
A group SHALL show one icon per funded level, plus the next while a technique is chosen ahead (at half alpha) or a level is ready with fewer than five funded, on the pentagon about a point four units left of and above the group; the first empty level SHALL draw the choose icon and end the row. Its orbit bars SHALL span 20.3 to 26.3 units out and 14 either side, the last filling by the points past the funded levels when below 0.99 in double precision.

#### Scenario: Two levels
- **WHEN** Close Combat has 10 000 points with Remise and Feint at 100, 100 and the unit 4
- **THEN** the icons stand at 84, 11 and 154, 61

### Requirement: Skill popups and tooltips
The learning row and the forget dialog SHALL open over 300 ms (alpha `t(2 − t)`, width eased again) and close over 200 ms (alpha `(remaining × 0.005)²`), clearing the pending choice after. The learning row SHALL be 82 by 28 units four units under its node with eight icons ten units apart; the forget dialog 82 by 20 units two under it with Yes 12 units left and No 6 right; a ring node's tip 82 by 14 units 13 under it, clamped into the window, naming the technique chosen at that level.

#### Scenario: Half way open
- **WHEN** an opening popup has 150 ms left
- **THEN** the alpha is 0.75 and the width scale 0.9375

### Requirement: Extended control classes
The unit SHALL register the text field, combo box, colour swatch, spinner, progress bar, separator, image, texture view, trackbar, caption and scroll bar with their default sizes, hooks for the text field, combo box, spinner, trackbar and scroll bar, and a unit hook caching the spinner button width `3·unit | 1`. The trackbar's hook SHALL default the knob width to 20 and the pad to 2.

#### Scenario: Defaults
- **WHEN** the unit is 4
- **THEN** the spinner button is 13 wide and a trackbar with no knob width gets 20

### Requirement: Spinner rules
A spinner commit SHALL set a value below the minimum to it and above the maximum to it, letting a NaN pass, and change the value only when it moves by more than 1e-8 in double precision. A press left of the button column SHALL place the caret, on the buttons step up above half the height and down below. Typed bytes SHALL pass as digits, a dot (stepping over a dot at the anchor) or a minus while the anchor or caret is 0.

#### Scenario: A NaN commit
- **WHEN** a spinner at 50 commits a NaN
- **THEN** the value stays 50 and the text is marked dirty

### Requirement: Extended control geometry
The image SHALL pick `(x − pad)/(w − pad)` and `1 − (y − pad)/(h − pad)` from the message's coordinates, the scroll bar map `(x − left − pad − width/2)/travel` with a `4·unit + 1` thumb, both clamped with a NaN passing; the progress bar SHALL fill `w × value` half to even; a separator SHALL leave its text between two `(w − gap)/2 − 1` lines, or keep only the right part when right-aligned; a drop-down list that does not fit SHALL open above or be cut to the rows below by its row counts.

#### Scenario: Zero travel
- **WHEN** a scroll bar 21 wide with pad 2 and unit 4 maps x 10
- **THEN** the value is NaN

### Requirement: Main-menu hit band
The main menu SHALL hit a row when the point lies strictly inside its band: the menu font's ascent less its second origin field tall from the band's top, and the row's width from `round((W − width)·0.5)`. The first band SHALL start at `round(H·origin)` and each next one at `round(70·row_h + top)` in single precision; a disabled row SHALL take no point but advance.

#### Scenario: Title band
- **WHEN** the title page at 1600×1080 has a 100-wide first row, ascent 49 and inset 16
- **THEN** y 420 at x 800 hits row 1, y 410 and y 443 miss, and x 751 hits while x 750 misses
- **WHEN** row_h is 0.25 and the top 101
- **THEN** the next bands start at 118 and 136

### Requirement: Main-menu miss
While a manual opened from the menu covers it, the main menu SHALL keep every point; otherwise a point on no row SHALL be kept when the menu blocks and let through when it does not.

#### Scenario: Blocking
- **WHEN** a point misses every row of a blocking menu
- **THEN** the menu keeps it

### Requirement: Main-menu commands
The main menu's title and arena pages SHALL open, row by row: a story game at the class select, the story saves, the arena page, the manual, the settings and the quit; an arena game at the emblem generator, the arena saves, the practice page, the versus lobby (or the title without it) and the title. The practice page SHALL open a practice game at the novice or expert difficulty, or go back.

#### Scenario: Arena fourth row
- **WHEN** the arena page's fourth row is clicked without the versus lobby
- **THEN** the title page is shown

### Requirement: Main-menu in-game commands
The in-game CONTINUE SHALL resume outside practice and start a new practice character in practice; its EXIT SHALL save and leave an arena, quit a story, and return to the title otherwise, never quitting the application. An end page's third row SHALL finish the match, as a forfeit unless the session state has bit 4.

#### Scenario: Exits
- **WHEN** the in-game EXIT is clicked in an arena, a story and a practice game
- **THEN** the commands are the arena's exit, the story's quit and the exit to the title

### Requirement: Main-menu press and release
The running main menu SHALL select the row under the pointer, mark a pressed row, run a row's command on the release over the pressed row (and not when released elsewhere), and run the selected row at once on Enter. Escape SHALL go back from the arena and practice pages and close the menu elsewhere.

#### Scenario: Release elsewhere
- **WHEN** a row is pressed and the pointer is released over another row
- **THEN** no command runs and the press mark clears

### Requirement: Window focus
The window manager SHALL let only the modal form (or no form) take the focus while one is set, clearing the focus otherwise. A new form SHALL deselect the old one and start with no control; a control the form has not already selected SHALL be selected and stored, and a control it has SHALL leave the stored control as it was.

#### Scenario: Reselecting
- **WHEN** a form already on control 2 is focused with control 2 while the stored control is 1
- **THEN** nothing is selected and the stored control stays 1

### Requirement: Tooltip request
A select answer with bit 0x100 SHALL request a tooltip for the focused form and control, stamped now, or 400 earlier when a tooltip shows and the answer lacks 0x200. The tick SHALL show it once 400 clock units have passed if the focus is still on it, and clear the request either way; a press or the tooltip's own time SHALL hide a showing tooltip with a 256-unit fade.

#### Scenario: Delay
- **WHEN** a hinted control is focused at 1000
- **THEN** the tick at 1400 shows nothing and the tick at 1401 shows its tooltip

### Requirement: Modal form
The modal form SHALL take every hit, in its own coordinates; a press whose target is another form (through a capture) SHALL close it and clear it.

#### Scenario: Outside point
- **WHEN** a modal form is set and the pointer is outside it
- **THEN** the hit is still the modal form, with no control

### Requirement: Pointer drag
With a button held, the pointer SHALL start a drag once it moves more than `(200 − elapsed)/20` times the drag distance from the press point during the first 191 clock units, and the distance itself after; the distance SHALL be one pixel per 540 of screen height, rounded half to even, at least 2.

#### Scenario: Threshold
- **WHEN** a press at x 100 with distance 2 is followed at once by moves to 120 and 121
- **THEN** 120 does not drag and 121 does

### Requirement: Double click
The same button pressed again within 450 clock units and one pixel of the press point SHALL send the double-click press, then the plain press when that is answered 0 or 0x10, and otherwise swallow the button's release.

#### Scenario: Window
- **WHEN** a second press comes 449 units after the first at the same point and the form answers 0
- **THEN** the form receives the double-click press and then the plain press

### Requirement: Press and release dispatch
A form answering a press with anything but 0x40 SHALL hold it, come to the front, and take the capture when the answer has bits 1 and 4. A release of the held buttons SHALL click (0x2000) a press nobody held that did not drag, send a held press's release to its form, and let a capture keep itself on 0x40 and drop on 0x80; a release of other buttons SHALL do nothing.

#### Scenario: Unheld click
- **WHEN** a form answers its press with 0 and the button is released
- **THEN** it receives the click word with 0x2000

### Requirement: GUI level lengths
The level switch SHALL derive the title as six units less two, a title-bar control as four units and one, the controls' step as half a unit (shifted) and a control, and a grip as four units less one, from level + 3, or 8 at level 4; level 5 SHALL then raise the unit to 12 with the other lengths kept.

#### Scenario: Level 5
- **WHEN** the level is 5
- **THEN** the unit is 12, the title 46, a control 33 and a grip 31

### Requirement: Form clamp and toggle switch
A clamped form SHALL keep x at 0 or more, move flush with the screen's right edge when it overflows, keep y at least a unit, and keep its size. Switching a toggle on SHALL clear the on and lit bits of every on control of its group, itself included, reporting each unless quiet, then set it on with its lit bit following.

#### Scenario: Overflow
- **WHEN** a 100-wide form at x 750 is clamped on an 800-wide screen
- **THEN** it is at x 700

### Requirement: Text truncation
Shortening a text with the ellipsis SHALL count the ellipsis' advance and then the bytes before the last; at the k-th byte that passes the width the first k − 1 bytes and the ellipsis SHALL remain. A line cut to `round(width)/gap` glyphs SHALL replace its last with the ellipsis and still advance by the replaced byte. A fitted tail SHALL be measured with the GUI level's main font.

#### Scenario: Ellipsis
- **WHEN** every byte and the ellipsis advance 10 and "abcdef" is fitted to 45
- **THEN** the result is "abc" and the ellipsis

### Requirement: Text width fit
The width fit SHALL count, for each line, every byte that fits (the head of a wrapped word included), narrow the width to the average, widen it by the GUI unit until the text takes no more lines than before (a final carriage return counting one more), keep the widest line of that pass and return that pass's count. The line count of an empty text SHALL be zero.

#### Scenario: Carriage return
- **WHEN** "abc def" and a carriage return is fitted from 30 with every letter 10, a space 5 and unit 4
- **THEN** two lines are returned and the width is 30

### Requirement: Wrapped text orientation
A wrapped block SHALL draw each line's glyphs hanging down from the line's top and stop when the top passes the block's height less a line; the lines it skips SHALL be drawn at the block's top.

#### Scenario: Three lines
- **WHEN** "ab cd ef" wraps at 30 from y 0 with a 16-pixel line
- **THEN** the glyph tops are 0, 16 and 32

### Requirement: Menu font record
The menu font record SHALL name `titlefont.rfi`, hold 112-pixel cells, ascent 98, a 36-pixel space, line height 128 and a trim of 6, and rescale every metric and the advances of `A` through `a` by half the row scale, rounded half to even.

#### Scenario: Quarter scale
- **WHEN** the record is rescaled by 0.25
- **THEN** the cell is 28 and C, 70 at design, advances 18

### Requirement: Single-line edit keys
The single-line editor SHALL leave the window's end to its scroll; a byte refused at the maximum, a key below Backspace and an unknown key SHALL change nothing and not scroll; a copy or cut with nothing selected SHALL take the whole text; a paste SHALL scroll even when the clipboard had no text. The caret from a pointer SHALL walk up to the window's last byte.

#### Scenario: Copy all
- **WHEN** "hello" has no selection and Ctrl+C is pressed
- **THEN** the copied text is "hello"

### Requirement: Single-line edit draw
The editor's whole-run draw SHALL read the window's end as a count from the first visible byte; without a selection shown it SHALL draw the window in the bound colour; with one it SHALL draw the bytes before it in the state's ink, a rectangle one line tall ending a pixel below the baseline in the selection colour at half alpha, the selected bytes in the selected ink and the rest in the state's ink. A `[tcol]` tag with its own alpha SHALL keep it, zero included.

#### Scenario: Selection
- **WHEN** "abcd" with 10-pixel bytes and a 16-pixel line has b and c selected at baseline 20
- **THEN** the rectangle is 10, 5, 20 wide and 16 tall at alpha 50 of 100

### Requirement: Viewport samples
A supersampled frame SHALL render every sample of its level: sample i uses the level's point i, the share `(i + 1)/count` in double precision, `0.5 − (i + 0.5)/count` and, after the first, the blend `1 − 1/(i + 1)`; a point's jitter SHALL be `2·scale·point/size` per axis with the view's projection scales. The camera scale SHALL be `(2·fov)·0.5` over the distance held to 100–3000, a not-a-number distance staying one.

#### Scenario: Level 2
- **WHEN** level 2 is rendered
- **THEN** it takes points 3, 4 and 5 with shares one third, two thirds and one

### Requirement: Viewport frame-rate readout
The frame-rate readout SHALL refresh once more than 250 ms have passed since the last, with the frames over the elapsed seconds, two decimals and " FPS", and sit with its right end a unit inside the client, its width the main font's gap times its length.

#### Scenario: Place
- **WHEN** "57.25 FPS" is placed with an inner right of 800, border 8, unit 5, gap 6 and line height 16
- **THEN** it is at 733, 21

### Requirement: Viewport cursor actions
A view press with no control selected SHALL pick the cursor action whose mask the press word holds with the most bits in the mask's low 16, the last among equals; a running action SHALL be ended by the next press (taking it on bit 0), keep running on a release answered 0x40, and in a view drag get the moves until it answers 0x80.

#### Scenario: Low bits
- **WHEN** the masks are 0x1 and 0x10001 and the word is 0x10001
- **THEN** the later mask is picked

### Requirement: Control layout pass
The control layout SHALL place only records not yet placed, mark them in its second pass, resolve a row start by scanning back from the record above, fill a known class's size, style and default margin (1 for the icon button and picture), initialise a trackbar, snap widths and x to four pixels half to even, and right-align by the client's inner right less the border. Appending records SHALL keep those up to the last in use and drop the rest.

#### Scenario: Snap
- **WHEN** a trackbar record 50 wide is laid out at unit 4
- **THEN** it is 48 wide and 16 tall with an 18-pixel knob and margin 3

### Requirement: Item card request
A card request with an item under the pointer and no request pending SHALL ask for a delayed card when the item's use type is 0 or 1, run the game's use first for type 2, and build the card at once otherwise. A delayed card SHALL come due 200 ms of real time after the request, a new request SHALL replace a pending one, and a close request SHALL cancel a pending timer before closing an open card.

#### Scenario: A delayed card
- **WHEN** a card is requested at 1000 ms for an item of use type 0
- **THEN** it is not due at 1199 ms, is built once at 1200 ms, and a cancelled later request builds nothing

### Requirement: Character view camera
The character view SHALL place its camera 410 cm from the figure at (0, 100, 0) towards (−200, 60, 360) with a 30° field of view, and choose its first super-sampling level as 1 for a setting at most 0, the setting plus 2 for 1 to 3 and 4 above. Its task SHALL ease the figure's race share up to the race's value by 0.003 per frame millisecond, never past it.

#### Scenario: The camera
- **WHEN** the view is built with a super-sampling setting of 3
- **THEN** the camera is 410 cm from the figure on the (−200, 60, 360) side and the level is 5

### Requirement: Character view background
The background SHALL be the skin scheme's bytes over 240, decoded from sRGB when sRGB targets are on and brightened by 1.4 when they are off, washed to red 0.38 and green 0.4 of the green and blue 0.45 of the blue, as bytes of 255 at the top and 280 at the bottom rounded half to even.

#### Scenario: The wash
- **WHEN** a scheme of green 240 and blue 200 is washed without sRGB targets
- **THEN** the top colour is (136, 143, 134)

### Requirement: Figure drag acceptance
The figure SHALL accept apparel only for a race that wears it in a view that allows changes, a withheld weapon or shield only with the body's matching permission, and an item with neither section only as the AL crystal on a race whose core word takes it while nothing is shown. A drop SHALL wear apparel, take a weapon, take a shield or take the body's core crystal, in that order.

#### Scenario: The AL crystal
- **WHEN** item 500 is dragged over a body whose race core word takes it, first with nothing shown and then with an item shown
- **THEN** the first is accepted and the second refused

### Requirement: Figure drag start and release
A frozen view SHALL start drags only from a held item, and an item in a locked view flagged 0x80000000 SHALL be dragged as a payload no panel takes. A release SHALL take a held item out of its hand, take the shown item off when the dragged icon is the shown one, and otherwise take the dragged item off, charging the pack its bulk when the pack did not receive it.

#### Scenario: Releasing over the figure
- **WHEN** drags end over hand 2, over the shown item, and elsewhere with the pack open
- **THEN** the first takes hand 1 (0-based) out, the second takes the shown item off, and the third takes the dragged item off and charges the pack
