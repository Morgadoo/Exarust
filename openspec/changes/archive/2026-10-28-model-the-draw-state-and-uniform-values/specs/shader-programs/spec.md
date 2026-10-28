# Spec Delta

## ADDED Requirements

### Requirement: Per-draw plan
A draw SHALL gather lights only when it has a light list and its material word has the diffuse or specular bit, and SHALL otherwise clear the six slots. It SHALL rewrite the material block's diffuse alpha to `1 − fade` only under the frame's fade bit when the two differ by more than the engine epsilon (an unordered compare does not rewrite). A half-pass draw under sun shadow mode 3 SHALL halve the last sub-frame index, truncating toward zero.

#### Scenario: Unlit material
- **WHEN** a draw with a light list has neither the diffuse nor the specular bit
- **THEN** its light slots are cleared

#### Scenario: Half passes
- **WHEN** four sub-frames are drawn for a half-pass object under sun mode 3
- **THEN** two sub-frames are drawn

### Requirement: Motion-blur shutter time
A node with the motion-blur flag SHALL get, for sub-frame `i`, the time `((i + first + 0.5)·d / factor + clock) − d·0.5` in single precision in that order.

#### Scenario: Four samples
- **WHEN** four samples cover a 0.1 s shutter at clock 2
- **THEN** the first sub-frame's time is 1.9625 and the last 2.0375

### Requirement: Fog position
`FogPos` SHALL be the eye plus half the difference to the sun camera's eye, with `w` the scene's integer height limit and `y` lowered to that limit when above it.

#### Scenario: Above the limit
- **WHEN** the midpoint's `y` is 350 and the limit is 340
- **THEN** `FogPos` is `(x, 340, z, 340)`

### Requirement: Refraction background
A liquid-mode record SHALL bind the scene refraction texture's high field above an opacity of 0.85 and its low field otherwise when the scene's refraction switch is on (nothing without the texture), and the built-in `BkgRfr02` or `BkgRfr01` by the same test when it is off; the opacity SHALL be widened to double before the compare. Any other program with local reflections SHALL bind the scene texture's low field when there is one.

#### Scenario: Stored 0.85
- **WHEN** a liquid record with opacity 0.85 is drawn with the switch off
- **THEN** `BkgRfr02` is bound

### Requirement: Light colour precision
A light's colour SHALL be scaled by `2/π` over the tap count computed in double and narrowed, then by `(1 − dimmer)·exposure`; a sun-like light's filter radius SHALL be `softness·0.5` in single precision times the double `0.005`, narrowed, whatever `FastLgt` says.

#### Scenario: Seven taps
- **WHEN** a white light of intensity 1 uses seven taps
- **THEN** its colour is 0.09094568
