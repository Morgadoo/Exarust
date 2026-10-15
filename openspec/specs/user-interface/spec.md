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
