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
The system SHALL pick the cursor's shape as the original does. In combat mode: chevrons while walking to the cursor (two beyond 300 cm), else the arrowhead while Ctrl is held, else the ring, also down or collapsing; the arrowhead once dead. Outside it: the cross while down, collapsing or dead, chevrons while walking to the cursor, else the arrowhead over a point within 3 m and the cross beyond. Ring and chevrons SHALL turn away from the player.

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
At place 10, 20 of size 100 by 80, style bits 0x30 SHALL select one frame: 0x10 from 10, 20 to 110, 100 with texture 0x9c then 0xb8, and 0x20 the same rectangle with 0xb4 then 0x9c. Style bits 0xc0 SHALL independently select a second frame inset by one on every side, with 0x40 using 0x9c then 0xb8 and 0x80 using 0xb4 then 0x9c. Any other value in a group SHALL select nothing.

#### Scenario: Each group selects its frame
- **WHEN** the style is 0x10
- **THEN** one frame is 10, 20 to 110, 100 with textures 0x9c and 0xb8
- **WHEN** the style is 0x20
- **THEN** one frame is the same rectangle with textures 0xb4 and 0x9c
- **WHEN** the style is 0x40
- **THEN** one frame is 11, 21 to 109, 99 with textures 0x9c and 0xb8
- **WHEN** the style is 0x80
- **THEN** one frame is the inset rectangle with textures 0xb4 and 0x9c

#### Scenario: Both groups run
- **WHEN** the style is 0x50
- **THEN** the first frame is 10, 20 to 110, 100 and the second is 11, 21 to 109, 99
- **WHEN** the style is 0, 0x30 or 0xc0
- **THEN** no frame is selected

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
With the guard clear the method SHALL return three programs in order: kind 5 with `MPrj` set to 2, kind 0x15 with `MPrj` set to 2 and a `Scale` lookup, and kind 5 with `MPrj` set to 2, `Tex` set to 0 and a `TexCol` lookup. It SHALL count six uniform operations. With the guard set it SHALL return none.

#### Scenario: The three programs
- **WHEN** the guard is clear
- **THEN** three programs are returned with kinds 5, 0x15 and 5
- **AND** the first sets `MPrj` to 2 and nothing else
- **AND** the second sets `MPrj` to 2 and looks up `Scale`
- **AND** the third sets `MPrj` to 2, sets `Tex` to 0 and looks up `TexCol`
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
Page 2 SHALL call 0x10018bf80 for action 1 and show form 0x1001243c0 for action 2. Page 4 SHALL call 0x100124650 for action 1 on form page 4, otherwise 0x100124820, and call 0x100124820 for action 2. Page 8 SHALL, for action 1 on form page 2, call 0x100132250, 0x10017b480 and 0x10012c2c0 and show 0x100124800; otherwise it SHALL show 0x100123d30 with the last-round flag. Any other page SHALL call its registered handler when present. Every path SHALL clear the action and field 0x204 and return 0.

#### Scenario: The built-in pages
- **WHEN** the page is 2 and the action is 1
- **THEN** 0x10018bf80 is called
- **WHEN** the page is 4 and the action is 1 on form page 4
- **THEN** 0x100124650 is called
- **WHEN** the page is 8, the action is 1 and the form page is 2
- **THEN** the three calls and form 0x100124800 are dispatched

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
A positive selection SHALL store itself and one less, return 7, play a sound when the sound global is positive, copy the page global and show form 0x10012a870 with flag 0. A selection of zero or less SHALL return 0x40 and do nothing else.

#### Scenario: A positive selection
- **WHEN** the selection is 3 and the sound global is positive
- **THEN** the selection is stored, the global becomes 2, the sound plays, the page copies, the form shows and the result is 7

#### Scenario: The guard
- **WHEN** the selection is 0 or negative
- **THEN** the result is 0x40 and no form shows

### Requirement: Class navigation
Mask 0x100 SHALL confirm a positive selection with result 7 and form 0x10012a870, and return 0 for no selection. Masks 0x200 and 0x400 SHALL call 0x10012ac60. Direction 0x8000 SHALL move down to the first available class at or after the selection up to 3; direction 0x4000 SHALL move up from selection − 2, or to 1 when at most 2. The move SHALL run only when the target differs.

#### Scenario: The confirm
- **WHEN** the mask is 0x100 and the selection is 3
- **THEN** the result is 7 and form 0x10012a870 shows
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
The angle SHALL be `(slot − 3.5) × 2π / count + 1 − turn`. Each axis SHALL be the centre plus the product of the two offsets, the unit and the cosine (x) or sine (y), rounded to the nearest even integer and multiplied by the turn. The centre x SHALL lose one unit when scrolled with a positive page; the centre y SHALL lose three units on page 0.

#### Scenario: A node on the ring
- **WHEN** the count is 8, the offsets are 10 and 5, the turn is 1, the size is 200 by 100, the page is 0 and the slot is 3
- **THEN** the position is 285, −39
- **WHEN** the page is 1 and the screen is scrolled
- **THEN** the position is 281, −27

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
The satellite SHALL start from the stored slot's ring point at `(stored − 2.5) × 2π / count + 1 − turn`, with x radius both offsets and y radius only the first, then add the rounded `unit × 18.3 × cos` of `(sample − 1) × 0.2` of a turn to x and subtract the sine from y, each times the turn.

#### Scenario: Two samples
- **WHEN** the count is 8, the offsets are 10 and 5, the turn is 1, the size is 200 by 100, the page is 0, the unit is 4 and the stored code is 3
- **THEN** sample 1 is 358, 53
- **AND** sample 3 is 226, 10
- **WHEN** the page is 1 and the screen is scrolled
- **THEN** sample 1 is 354, 65

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
A high selection SHALL give 1 for a horizontal offset between 13 and 4 units to the left and 2 for the same to the right, within 4 units vertically, else −1. A low selection SHALL give the rounded slot from 4.5 plus the offset over ten units when it is 1 through 8, else −1. Without selections the fallback SHALL run.

#### Scenario: High, low and fallback
- **WHEN** the high node is at 100, 50, the unit 4 and the point 80, 50
- **THEN** the result is 1
- **WHEN** the point is 120, 50
- **THEN** the result is 2
- **WHEN** the low node is at 100, 40 and the point 120, 40
- **THEN** the slot is 5
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
