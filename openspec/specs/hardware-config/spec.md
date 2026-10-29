# hardware-config Specification

## Purpose
The settings screen behind the original's hardware configuration: its graphics, audio and input options with their steps and labels, the live, draft and saved copies, Apply, Cancel and Defaults, the GPU benchmark and its quality tiers, the startup handlers and the Controls page's rows.

## Requirements

### Requirement: Slider steps
The settings page SHALL turn a slider fraction `f` into its option with the processor's to-nearest, ties-to-even conversion: volume `1.5·f`, super-sampling `round(3·f)` plus one when above 1, shading `round(3·f) + 3`, texture detail `round(f) + 2`, texture quality `round(3·f) + 2`, shadow detail `round(f) + 4`, shadow quality `round(3·f) + 1`, hardware anti-aliasing `1 << round(3·f)` when positive, aim speed `f` shown as `round(99·f) + 1`.

#### Scenario: Ties
- **WHEN** the texture-detail slider is at 0.5 and the anti-aliasing slider at 0.5
- **THEN** texture detail is 2 and the samples are 4

### Requirement: Slider labels
Labels SHALL keep the original's leading space. The first branch of the Off…Ultra scale SHALL take only 0 and 1 and that of the Coarse… scale only 0 to 2, so a negative value shows no text.

#### Scenario: A negative value
- **WHEN** the Off…Ultra label is asked for −5 and for 1
- **THEN** the first has no text and the second is " Off"

### Requirement: Opening the page
Opening the settings form SHALL take the saved set from the live options, read the settings file again over the live options into the page's draft (each line through its startup rule), and SHALL then restore the live quality levels, super-sampling, hardware anti-aliasing, vertical sync and window mode from the saved set. Opening SHALL reset nothing and SHALL run no benchmark.

#### Scenario: A file read again
- **WHEN** the live super-sampling is 0, the frame rate is hidden and the file holds `Supersample = 4` and `ShowFPS = 1`
- **THEN** the draft's super-sampling is 4, the live super-sampling stays 0 and the live frame rate is shown

### Requirement: What the rows show
The volume row SHALL show the live volume over 1.5; the music, super-sampling and window rows the live options; the other rows the draft. The anti-aliasing row SHALL be disabled while the live super-sampling is above 0.

#### Scenario: Rows
- **WHEN** the live volume is 0.75 and super-sampling is 3
- **THEN** the volume slider opens at 0.5 and the anti-aliasing row is disabled

### Requirement: Live and draft controls
The volume, texture-detail and texture-quality sliders and the vertical-sync, frame-rate, tips and checkpoints checkboxes SHALL change the draft and their own live option at once; the other sliders and the window-mode checkbox SHALL change the draft only. The Music checkbox SHALL only mute the running session.

#### Scenario: A texture slider
- **WHEN** the texture-quality slider moves to 0 and the super-sampling slider to 1
- **THEN** the live texture quality is 2 and the live super-sampling is unchanged


### Requirement: Apply's file
Apply SHALL do nothing, keeping the form open, when the settings file cannot be read. Otherwise it SHALL write `PerfIndex`, `Volume` (percent), `Supersample`, `ShadingQuality`, `TextureDetail`, `TextureQuality`, `ShadowDetail`, `ShadowQuality`, `HWMultisample`, `InputMode` (0 or 51), `AimSpeed`, `Fullscreen`, `Music`, `ShowFPS`, `ShowGameTips`, `Checkpoints`, `VerticalSync` and `ShowFPS` in that order.

#### Scenario: Missing file
- **WHEN** the settings file is missing and Apply is pressed
- **THEN** no file is created, nothing changes and the form stays open

### Requirement: Apply's sources and pushes
Apply SHALL take the quality levels, both sampling options, full screen, vertical sync and the second `ShowFPS` from the draft and the other lines from the live options. It SHALL commit the Controls page's copy of the bindings when its Forward key is set, take the draft's window mode, and take each of super-sampling, shading, shadow quality and shadow detail that differs from the saved set live. Hardware anti-aliasing SHALL only be stored.

#### Scenario: Line sources
- **WHEN** the controller switch is on and the draft's anti-aliasing is 4
- **THEN** the lines hold `InputMode = 51` and `HWMultisample = 4` and the live anti-aliasing is unchanged

### Requirement: Cancel
Cancel SHALL push the saved set into the live options (all but the shadow detail) and close the form, dropping the Controls page's edits.

#### Scenario: Cancel after edits
- **WHEN** the super-sampling and volume sliders moved and Cancel is pressed
- **THEN** the live options are those the page opened with

### Requirement: Defaults on the Settings page
Defaults on the Settings page SHALL run the GPU benchmark (the Deck takes its fixed preset: super-sampling 3, shading 5, texture detail 2, texture quality 5, shadow detail 5, shadow quality 3) and make the result the saved set, every other option copied from the live ones except the window mode, which stays the saved set's. It SHALL push that set into the live options (keeping their shadow detail) and the draft (keeping its anti-aliasing and window mode).

#### Scenario: Defaults after a benchmark
- **WHEN** the benchmark gives a score of 4 on a 1600 × 900 display and the live anti-aliasing is 4
- **THEN** the live super-sampling is 3, the live shadow detail is unchanged, the draft's shadow detail is 5 and the anti-aliasing stays 4

### Requirement: Defaults on the Controls page
Defaults on the Controls page SHALL restore the shipped keys into the page's copy of the bindings, or, in controller mode, the saved aim speed into the live one.

#### Scenario: Shipped keys
- **WHEN** Forward was rebound to L and Defaults is pressed on the Controls page
- **THEN** the page's copy binds Forward to W again

### Requirement: GPU benchmark
The benchmark SHALL warm up for half a second and then take three half-second runs, each run's rate being its frame count over its whole half-seconds rounded to even, and SHALL score the best rate divided by 60.

#### Scenario: Best of three
- **WHEN** the runs count 5, 10 and 5 frames in one half-second each
- **THEN** the score is 10 / 60

### Requirement: Quality tiers
A score below 1 SHALL give super-sampling 0, shading 3, texture detail 2, texture quality 2, shadow detail 4 and shadow quality 1 below 0.6 or 2 from it; from 1 to 1.5 texture quality 3 and shadow quality 2, the rest as below 1; from 1.5 shading 5, texture detail 3, texture quality 5, shadow detail 5, shadow quality 4 and super-sampling from `√(2304000 / (width·height)) × score`: 0 below 3, 1 below 5, 3 below 10, else 4. A NaN score SHALL take every "not below" branch.

#### Scenario: Tiers
- **WHEN** the score is 2.38, 3.96 or 7.91 on a 1600 × 900 display
- **THEN** super-sampling is 1, 3 or 4

### Requirement: Startup handlers
Reading the settings file SHALL set the volume to the `Volume` value × 0.01 in single precision, the aim speed to `(AimSpeed − 1) / 99` in double precision then single without a limit, and the controller mode when `InputMode` is greater than 0.

#### Scenario: Handlers
- **WHEN** the file holds `Volume = 150`, `AimSpeed = 199` and `InputMode = -51`
- **THEN** the volume is 1.5, the aim speed 2 and the keyboard is used

### Requirement: Controls rows
The Controls page SHALL have 22 rows, each with a key cell: Forward, Back, Left, Right, Run/Sprint, Crouch, Orbit Camera, Cursor Look, Toggle Combat, Alternate Attack, Switch Equipped, Flip weapon, Interact, Inventory, Skills, Journal, Talk, Use Power, Power Bar, Previous Power, Next Power and Power Tree, each bound to its own slot of the player's table.

#### Scenario: Power rows
- **WHEN** the shipped bindings are shown
- **THEN** Use Power shows Q, Power Bar shows C and Power Tree shows P

### Requirement: Key names
A key name SHALL be "Mouse 3", "Mouse 4" or "Mouse 5" for the mouse flag's fourth to sixth codes and empty for its others, `<code>` for a code with bit `0x4000`, and otherwise the key's name in title case.

#### Scenario: Shapes
- **WHEN** the codes are the mouse flag with 4, bit `0x4000` with 4, and the left shift key
- **THEN** the names are "Mouse 3", "<4>" and "Left Shift"

### Requirement: Settings form layout
The Settings form SHALL be 434 × 536 at unit 4, centred, with its three record arrays laid out by the control layout pass at unit 4 and rescaled with the form and its pages by whole units of their clients. The Settings page SHALL be a child at 0, 64 of the form's width less 8 by 408; the Controls page a child of the width less 16, 416 high, over 676 of content, placed by the page switch at 14 units down and 118 units less that high.

#### Scenario: Unit 4
- **WHEN** the form opens at unit 4 on a 1280 × 720 window
- **THEN** it sits at 423, 92, its tabs at 12, 12, Defaults, Apply and Cancel at 40, 156 and 276 on y 484, and the first key cell at 232, 6 of the Controls page

#### Scenario: Unit 6
- **WHEN** the form opens at unit 6 on a 1920 × 1080 window
- **THEN** it is 646 × 802 at 637, 139 and the Settings page is 634 × 610 at 0, 96

### Requirement: Settings form hit test
The form SHALL hit the page in view before its own records, and a record on its rectangle (edges included) and then within two units. Rules, labels without a callback, hidden and disabled records SHALL take no hit.

#### Scenario: A row label
- **WHEN** the pointer is on a Controls row's label
- **THEN** nothing is hit, and on the row's key cell the cell is

### Requirement: Settings form events
A press, a held move and a release SHALL go to the record as its control class's messages, and its report SHALL become the callback's event: a tab turning on switches the page, a slider gives its fraction, a checkbox its lit bit, a key cell opens the key-read screen for its row, and Defaults, Apply and Cancel act. The wheel SHALL scroll the Controls page by whole notches of 0.25.

#### Scenario: The CONTROLS tab
- **WHEN** CONTROLS is pressed and released at 1280 × 720, unit 4
- **THEN** the form shows the Controls page at y 56 and hides the rules

#### Scenario: The wheel
- **WHEN** the wheel turns one notch down over the Controls page
- **THEN** its scroll is 0.25 and the records move up 66 pixels

### Requirement: The form in the running game
The menu's SETTINGS rows SHALL open the form over the menu, which takes no input while it is open; hiding the menu SHALL close the form without Apply. Apply SHALL commit the bindings to the running game and save them to the store at once. The key-read and warm-up screens SHALL draw their text over a black backdrop at their fade.

#### Scenario: Hiding the menu
- **WHEN** the menu closes while the form is open
- **THEN** the form and the configuration's page close and the live options keep their changes

### Requirement: Anti-aliasing under super-sampling
The anti-aliasing slider's step SHALL write the draft whatever the super-sampling. The super-sampling slider at a level of 1 or more SHALL set the anti-aliasing slider to 0 through a reported set and disable it, and at 0 enable it; the page SHALL open the same way while the live super-sampling is on.

#### Scenario: Super-sampling turned on
- **WHEN** the super-sampling slider moves to its top step
- **THEN** the anti-aliasing slider is at 0, disabled, and the draft's samples are 0
