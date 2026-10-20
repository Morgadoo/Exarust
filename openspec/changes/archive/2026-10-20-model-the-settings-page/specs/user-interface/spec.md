## ADDED Requirements

### Requirement: Settings page content
The system SHALL provide the fifteen Settings items: device and memory info, the audio slider, seven quality choices, and the Vertical sync, Show framerate, Window mode, Show gameplay tips and Early Checkpoints toggles, with the toggles sharing rows 10–12.

#### Scenario: Rows and kinds
- **WHEN** the settings are read
- **THEN** there are fifteen items, the audio row is a slider and the quality rows index the choice list
- **WHEN** the toggles are read
- **THEN** Show framerate is on row 10 column 1 and Early Checkpoints on row 12 column 0

### Requirement: Controls page content
The system SHALL provide the fourteen control bindings from Forward/W to Inventory/I.

#### Scenario: Bindings
- **WHEN** the controls are read
- **THEN** Forward is W, Orbit Camera is Mouse 3 and Interact is Control

### Requirement: Settings layout
The system SHALL lay the pages out from the 630×800 retail form: two tabs, the content rectangle, the settings and controls row tops, the label/control/value columns and the three footer buttons, scaling linearly for other form sizes.

#### Scenario: Retail form
- **WHEN** the form is at (500,130) and 630×800
- **THEN** the tabs are at 520 and 808, the first settings row at 260, the last controls row at 698 and the Defaults button at (560,853)

#### Scenario: Half-size form
- **WHEN** the form is 315×400 at the origin
- **THEN** the first tab is (10,23,140,17) and the first settings row is 65
