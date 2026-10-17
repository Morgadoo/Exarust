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
