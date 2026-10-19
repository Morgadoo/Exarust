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
