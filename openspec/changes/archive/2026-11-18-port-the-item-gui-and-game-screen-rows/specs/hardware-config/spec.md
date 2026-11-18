## ADDED Requirements

### Requirement: First-run detection
When the settings file still has its `FirstRun` set (or has none), the game's start SHALL run the auto-detection (the GPU benchmark, or the Deck's preset) and, once it has its levels, SHALL write the settings lines into the settings file with no form open: the page values are the saved set pushed into them (the window mode, hardware anti-aliasing and performance index kept), the live options give the rest. Grid quadrants and screenshot runs skip it.

#### Scenario: A fresh user folder
- **WHEN** the game starts with a settings file holding only its new `FirstRun = 0`
- **THEN** the benchmark runs and, when it ends, the file holds the detected Supersample, ShadingQuality, ShadowDetail and ShadowQuality lines

### Requirement: Benchmark message box
While the GPU benchmark runs, the Settings form SHALL be hidden and take no input, and the "Configuration" message box (the text "Analysing system" with the code page's ellipsis, flags 0x60: no buttons, no close box, framed) SHALL show; when the benchmark ends the box goes and the form shows again.

#### Scenario: Defaults on the Settings page
- **WHEN** the player presses Defaults on the Settings page
- **THEN** the form hides, the box shows until the three runs end, and the form returns with the detected levels
