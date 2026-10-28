## ADDED Requirements

### Requirement: Loadout control table
The arena loadout form SHALL build its 27 controls from the shipped control records in the constructor's order, with their classes, positions, sizes, captions, style and state words.

#### Scenario: Shipped records
- **WHEN** the control table is read
- **THEN** record 0 is "ARSENAL" at x 200, record 21 is "CLOSE" with style 0x20483300 and record 22 "HIRE" has state 0x110

### Requirement: Divination cards drawn through their shader
The divination reading SHALL draw each card's shadow, back and face through the card shader, a turning card under the engine's axis-angle matrix with its shadow turned and its light `par0.z` taken from the turn, and a held card's face with the half width negated.

#### Scenario: Turn matrix and light
- **WHEN** a card turns a quarter about z
- **THEN** the matrix's second term is −1, its fourth +1, and the light's z is the matrix's last diagonal term

#### Scenario: Shader gain
- **WHEN** a white vertex colour draws a sheet texel (200, 100, 50) with green 100 at the mask
- **THEN** an upright card shows (200, 100, 50, 100) and a card edge-on to the light (40, 20, 10, 100)

### Requirement: Destination symbol colour
The destination screen SHALL colour each row's symbol as the first skin colour plus the second scaled by the row's mix, with the alpha replaced by the reveal alpha times 0.6.

#### Scenario: Full mix
- **WHEN** the base is (0.25, 0.25, 0.25) and the added colour (0.5, 0.25, 0) at mix 1
- **THEN** the symbol is drawn with colour (0.75, 0.5, 0.25)
