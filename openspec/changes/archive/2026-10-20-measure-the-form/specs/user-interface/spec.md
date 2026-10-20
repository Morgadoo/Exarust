## ADDED Requirements

### Requirement: Form skin measures
The system SHALL compute `base = 6u−border`, `close = 4u+1`, `rail = 4u−1`, `tab = u/2+close` and the DPI border `max(2, trunc(dpi/135))`.

#### Scenario: Unit five
- **WHEN** the unit is 5 and the border 2
- **THEN** base is 28, close 21, rail 19 and tab 23

### Requirement: Form defaults and anchoring
The system SHALL default a form to 302×402 with border 2, content 300×800, scroll step 0.025 and page minimum 76×90; unset anchors SHALL resolve to (4, 36), centre or far.

#### Scenario: Anchors
- **WHEN** an unset X anchor is default in an 800-wide parent with a 100-wide child
- **THEN** x is 4
- **WHEN** it is centred
- **THEN** x is 350
- **WHEN** it is far
- **THEN** x is 696

### Requirement: Form client rectangle
The system SHALL build the client rectangle from the border, the parent top/base/extra, the rail and the style word.

#### Scenario: Default and inset
- **WHEN** a 302×402 form has border 2 and no style bits
- **THEN** its client is 2,2–300,400
- **WHEN** the style has bit 0x200
- **THEN** the right edge shrinks by the parent base

### Requirement: Form scrolling
The system SHALL set the scroll step to 10/overflow, the thumb to `max(9, trunc(cw/content·track))` capped at the track, and the thumb x to `border+rail+trunc((track−thumb)·f)`, clamping the fraction and snapping or resetting as the style asks.

#### Scenario: A long form
- **WHEN** the width is 404, content 1200, border 2 and rail 19
- **THEN** the step is 10/796, the thumb 120 and the x at f 0 is 21, at f 1 is 263
- **WHEN** the content fits
- **THEN** the fraction resets to 0
