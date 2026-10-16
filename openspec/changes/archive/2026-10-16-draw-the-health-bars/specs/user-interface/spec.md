## ADDED Requirements

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

### Requirement: Low-health heartbeat
In story levels the system SHALL start the heartbeat when a blow leaves the player's yellow health at or above 0.01 and below 0.05, at most once a minute: for 7 s a dark red (80, 0, 0) border from the window's edges to a fifth of its height in, pulsing to alpha 96 every 800 ms, with the `heartbeat` sound looping at gain 0.3. Arena levels SHALL have no heartbeat.

#### Scenario: Low blow in a story level
- **WHEN** a blow leaves the player's yellow health at 0.03 in a story level
- **THEN** a pulsing dark red border and a heartbeat play for 7 s

#### Scenario: Arena
- **WHEN** the same blow lands in the arena
- **THEN** no border shows

### Requirement: Interface compositing
The system SHALL lay the interface over the game's view so that a translucent interface pixel blends once by its opacity: the interface camera's texture, drawn over transparent black, SHALL be composited premultiplied.

#### Scenario: Half-transparent white over the floor
- **WHEN** an interface pixel of white at opacity 0.28 lies over the arena floor (about 42 of 255)
- **THEN** the window shows it at about 147 of 255, not 87
