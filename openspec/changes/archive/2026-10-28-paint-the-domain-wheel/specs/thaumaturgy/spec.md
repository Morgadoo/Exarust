## ADDED Requirements

### Requirement: Domain wheel paint order
The domain wheel SHALL paint, in order: its backdrop, six sector bands, the chosen domain's glow, the six icons, the ring frame and the last highlighted icon's label. The backdrop, bands and glow SHALL draw with blend `0x103`.

#### Scenario: A hidden wheel
- **WHEN** the wheel's show is 0
- **THEN** the backdrop's reach is 1000 and its alpha 0

### Requirement: Domain wheel bands
With `s = (2 − shown)·shown` and `R = 140s`, sector `i` SHALL be a band at `(i + 2)·2π/6` from `round(0.64R)` to `round(1.2R)` with `par1 = (0.738i, 4(1 − shown)² + 4.5(1 − share)²·shown, 1 − share)`. The backdrop's `par1` SHALL be the reciprocal of `(2 − f)·f·shown`, or 1000 when that is not above 0.01.

#### Scenario: The first band
- **WHEN** the wheel is fully shown with no shares
- **THEN** the Mind band runs from 90 to 168 pixels above the centre with `par1 = (0.738, 4.5, 1)`

### Requirement: Domain wheel icons
The icon under the pointer SHALL get four glow copies 1.5 pixels off each diagonal at `round(96·shown)`, then be drawn at `round(192·shown)`. The chosen icon SHALL wear its domain colour at `round(255s)`.

#### Scenario: A hot and a chosen icon
- **WHEN** the wheel at unit 4 is fully shown, has Force chosen and Mind under the pointer
- **THEN** Mind's first glow copy sits at (122.5, 78.5) in client pixels at alpha 96, and Force's icon is (168, 135, 255) at alpha 255

### Requirement: Domain wheel texture
The tree's load SHALL resample `domainwheel` to `unit · 32` pixels a side through the engine's resample-to-size path, in its own format word and flags. It SHALL resample again when the GUI unit changes.

#### Scenario: The shipped wheel
- **WHEN** the 384-pixel sheet is loaded at unit 4, then the unit becomes 5
- **THEN** the texture is 128 pixels a side, then 160

### Requirement: Tree edit drags
A tree press SHALL clear the drag state and SHALL enter only the pan (middle button). No press SHALL enter the node, link-turn or link drags.

#### Scenario: Presses on a node and a link
- **WHEN** a node, then a link, is pressed with the left button, a left drag starts and a right press comes
- **THEN** the drag state stays idle, and only a middle press starts the pan

## MODIFIED Requirements

### Requirement: Tree domain wheel child
The tree SHALL carry the domain wheel at `(10, height − 290)`, 280 pixels square, with the opened domain as its choice, the bar's fade as its show and the focus share as its own focus value (48 with no record). An icon click SHALL go through the wheel's confirm; a confirmed domain SHALL start the domain switch and play `ThaumShow02` at 0.25. A pick or drop SHALL start the wheel's 300 ms ease.

#### Scenario: Known and unknown domains
- **WHEN** a tree opened in Mind with a 700-pixel height is clicked at the Energy icon (150, 626) with no known domains, then at the Mind icon with all known, then at the Energy icon again
- **THEN** the first two clicks start no switch and the third chooses Energy, starts the 256 ms fade-out and plays `ThaumShow02`

#### Scenario: No record
- **WHEN** a tree opens with no record
- **THEN** the wheel's focus value is 48
