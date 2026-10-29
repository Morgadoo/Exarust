## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Live and draft controls
The volume, texture-detail and texture-quality sliders and the vertical-sync, frame-rate, tips and checkpoints checkboxes SHALL change the draft and their own live option at once; the other sliders and the window-mode checkbox SHALL change the draft only. The Music checkbox SHALL only mute the running session.

#### Scenario: A texture slider
- **WHEN** the texture-quality slider moves to 0 and the super-sampling slider to 1
- **THEN** the live texture quality is 2 and the live super-sampling is unchanged

