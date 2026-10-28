## ADDED Requirements

### Requirement: Skill draw background and gate
On page 2 the skill draw SHALL first emit the background quad twelve by eight units, centred across and twelve units above the bottom, inked in the highlight under pointer code `0x10000` and otherwise in the skin's grey at `60 × scale + 120`. When the opening scale is below 0.001 in double precision the draw SHALL stop there (a NaN continues); otherwise it SHALL record the skin's full-screen fade with the scale.

#### Scenario: A hidden page draws only its background
- **WHEN** the screen's state is 4 on page 2 with the unit 4, the size 200 by 100 and the skin grey 0xc0
- **THEN** the draw holds one textured batch whose first corner is 76, 68 inked 0xc0, 0xc0, 0xc0, 120 and records no fade
- **WHEN** the same screen is on page 0
- **THEN** the draw is empty

### Requirement: Skill draw ring passes
Past the gate the draw SHALL dim the ring page in black at `77 × scale`, draw each node's plate (30 units times the content alpha, corners from the top left down, across and up with texture coordinates 0,0 0,1 1,1 1,0) with the ring program at `0, 0, 0, 0.6 × scale`, the five spokes with the skin's textured program over the background in the skin's ink using the engine's sine and cosine, the icons, and the orbit bars with the bar program over the background texture.

#### Scenario: The first plate and the dim
- **WHEN** the ring page is fully open at unit 4 on a 200 by 100 screen
- **THEN** the dim's corners are 0,0 then 0,100 then 200,100 then 200,0
- **AND** the first plate's first corner is the node less 120 on both axes with texture 0, 0

### Requirement: Skill tooltip inks
The ring tooltip's text SHALL take the plain ink 255, 248, 240 at `120 × scale × slide` and its glow `scale × (slide × 0.6)` in double, the slide stepping by `elapsed × 0.005` in double. The learning row's alpha SHALL be the popup's times the scale: its glow `scale × (row × 0.6)`, its text and icons 120 times it, a slot with a requirement byte set half that, and the hovered clear slot 255, 204, 153 at 255 times it.

#### Scenario: A full row
- **WHEN** the popup and the scale are 1
- **THEN** the row's glow is 0.6 and its text alpha 120

### Requirement: Forget dialog inks
The forget dialog's glow SHALL be `scale × (popup × 0.6)` in double and its text 120 times the scale and the popup's alpha. Once the popup's own alpha passes 0.5, Yes and No SHALL take the highlight under pointer codes 1 and 2 and the plain ink otherwise, at `(popup − 0.5) × (base × scale) × 2`.

#### Scenario: No under the pointer
- **WHEN** the popup and the scale are both 1 and the pointer code is 2
- **THEN** the glow is 0.6, the title's alpha 120, Yes 255, 248, 240, 120 and No 255, 204, 153, 255

### Requirement: Skill programs
The screen's setup SHALL build the ring and bar programs from their sources, upload to both the fixed `MPrj` matrix with columns 2,0,0,0 then 0,2,0,0 then 0,0,−0.001,0 then −1,−1,−0.001,1, look up `Colour` in both, set the bar program's `Tex` to unit 0 and look up its `Prog`, and load the `SkillShow_01` sound. Opening SHALL build a ring and, for a positive page, override only its count (4), radius (72), group tables and page.

#### Scenario: The projection's columns
- **WHEN** the programs are set up
- **THEN** the projection's first column is 2, 0, 0, 0 and its last −1, −1, −0.001, 1

### Requirement: Spinner set word
The spinner's set message SHALL take one 32-bit word: an integer spinner SHALL store it when its bits differ; a float spinner SHALL read it as a float when flagged, else convert it from an integer, clamp below the minimum, else above the maximum, with ordered compares, and land only a move over 1e-8.

#### Scenario: An unflagged word
- **WHEN** a float spinner of 0 to 100 receives the unflagged word 3
- **THEN** its value becomes 3.0

### Requirement: Spinner drag capture and halves
A capture SHALL take its bounds from the current value, integer differences signed 64-bit, the left bound unlimited above a maximum of 100000 and the right above a value of 100000. In the button column (the text inset wide) the point SHALL light the upper half when its offset from the top is below half the height and the state enables `0x4000000`, else the lower under `0x8000000`; outside the column both SHALL clear.

#### Scenario: An integer capture
- **WHEN** an integer spinner of 0 to 100 at step 1 holds 12 with the unit 4, the press at y 0 and the cursor at y 10
- **THEN** the capture's bounds are −346 and 62
