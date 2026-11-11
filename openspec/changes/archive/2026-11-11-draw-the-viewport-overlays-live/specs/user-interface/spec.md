## MODIFIED Requirements

### Requirement: Frame-rate readout
The system SHALL show the frame rate in the window's top-right corner as the original's viewport does ("57.25 FPS"): counted only while drawn, read once more than 250 ms have passed since the last reading, in the engine's number formatting with two decimals, in the GUI level's main font in physical pixels. A command-line option hides it.

#### Scenario: Running a level
- **WHEN** a level is shown
- **THEN** the top-right corner reads the current frame rate followed by "FPS"

### Requirement: Viewport frame-rate readout
The frame-rate readout SHALL refresh once more than 250 ms have passed since the last, with the frames over the elapsed seconds, two decimals in the engine's number formatting and " FPS", and sit with its right end a unit inside the client, its width the main font's gap times its length.

#### Scenario: Place
- **WHEN** "57.25 FPS" is placed with an inner right of 800, border 8, unit 5, gap 6 and line height 16
- **THEN** it is at 733, 21

## ADDED Requirements

### Requirement: Viewport overlay inks
The readout and the version line SHALL each be drawn over a black shadow at half alpha a pixel up and left; the readout in the skin's seventh label ink at alpha 0xa0, the version line in that ink as it is. The readout's right end SHALL sit at the window's width less twice the border (8) and a unit, and the version line a unit in from the left with its baseline the border and a unit up from the bottom.

#### Scenario: A 2560 by 1440 view
- **WHEN** "60.00 FPS" is placed at unit 8 with the 16-point font
- **THEN** its left is 2560 less 24 less 153 and its baseline 44

### Requirement: Title overlays
The version line "V 0.9.5.2g" SHALL show, and the readout SHALL not, from the return to the title or the splash's hand-over until a session starts or continues.

#### Scenario: The title after the splash
- **WHEN** `play exanima01 --splash` reaches the main menu
- **THEN** the bottom-left corner reads "V 0.9.5.2g" and no frame rate is shown

### Requirement: Frame-rate flag
The readout's flag SHALL take the ShowFPS setting at every session start and whenever the setting changes, and the return to the title SHALL clear it.

#### Scenario: Turning ShowFPS off in a session
- **WHEN** the setting is turned off while playing
- **THEN** the readout goes at once
