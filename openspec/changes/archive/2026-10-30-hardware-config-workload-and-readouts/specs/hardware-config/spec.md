## ADDED Requirements

### Requirement: Benchmark workload
The GPU benchmark SHALL draw the original's fragment program into a 1024² target from a 512² texture whose every byte is 0x3f, sampled with repeat addressing and linear filtering. It SHALL draw four full-screen quads in each warm-up frame and sixteen in each measured frame, and every drawn quad SHALL be shaded (none rejected by the depth test).

#### Scenario: Warm-up and runs
- **WHEN** the benchmark is warming up, and then running
- **THEN** four quads are drawn, and then sixteen

### Requirement: Readouts and Deck detection
The Settings page SHALL show "Valve Steam Deck" as the device on a Deck and the renderer's name otherwise. It SHALL show the video memory as `<n> MB`, or "Unknown" without a figure. A Deck SHALL be recognised by the Steam runtime's `SteamDeck=1` or the DMI product name `Jupiter` or `Galileo`.

#### Scenario: No figure on a Deck
- **WHEN** the form opens on a Deck whose driver gives no memory figure
- **THEN** the readouts are "Valve Steam Deck" and "Unknown"

### Requirement: Controls page help and view tabs
The Controls page SHALL hold the power-tree help text in a read-only box at 2, 48 units, its client's width less 4 units wide and 50 units high. The Keyboard and Controller tabs SHALL show only with Steam Input; a tab turning on SHALL set the controller mode and view and place the Controls page 21 units down. Without Steam Input the page SHALL be placed 14 units down in the keyboard view.

#### Scenario: Help box at unit 4
- **WHEN** the form opens at unit 4
- **THEN** the help box is at 8, 192, 381 wide and 200 high

### Requirement: Page switch
A page switch SHALL only show the page. It SHALL run no benchmark and SHALL leave the page's key copy as it is.

#### Scenario: Back to Settings
- **WHEN** the Controls tab and then the Settings tab are pressed after Forward was rebound to L
- **THEN** no benchmark runs and the page's copy still binds Forward to L

### Requirement: Older slider steps
The older record table's sliders SHALL step shading, texture detail and texture quality as `round(3·f) + 2`, shadow detail as `round(4·f) + 1`, shadow quality as `round(4·f) + 2`, and anti-aliasing as `1 << round(3·f)` when positive. Each rounding SHALL go to the nearest value, ties to even.

#### Scenario: A tie
- **WHEN** the older shadow-detail slider is at 0.125
- **THEN** the detail is 1

## MODIFIED Requirements

### Requirement: Defaults on the Controls page
Defaults on the Controls page SHALL restore the shipped keys into the page's copy of the bindings, or, in the controller view (the Controller tab lit, which needs Steam Input), the saved aim speed into the live one.

#### Scenario: Shipped keys
- **WHEN** Forward was rebound to L and Defaults is pressed on the Controls page
- **THEN** the page's copy binds Forward to W again

#### Scenario: Controller mode without Steam Input
- **WHEN** the controller mode is on without Steam Input and Defaults is pressed on the Controls page
- **THEN** the shipped keys come back and the aim speed is unchanged
