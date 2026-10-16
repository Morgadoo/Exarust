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
