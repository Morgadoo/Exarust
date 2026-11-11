## MODIFIED Requirements

### Requirement: Tree domain wheel child
The tree SHALL carry the domain wheel built from the wheel's constructor form at `(10, height − 290)`, 280 pixels square, with the opened domain as its choice, the bar's fade as its show and the focus share as its own focus value (48 with no record). An icon click SHALL go through the wheel's confirm at the live interface unit; a confirmed domain SHALL start the domain switch and play `ThaumShow02` at 0.25. A pick or drop SHALL start the wheel's 300 ms ease.

#### Scenario: Known and unknown domains
- **WHEN** a tree opened in Mind with a 700-pixel height at unit 4 is clicked at the Energy icon (150, 626) with no known domains, then at the Mind icon with all known, then at the Energy icon again
- **THEN** the first two clicks start no switch and the third chooses Energy, starts the 256 ms fade-out and plays `ThaumShow02`

#### Scenario: No record
- **WHEN** a tree opens with no record
- **THEN** the wheel's focus value is 48

## ADDED Requirements

### Requirement: Domain wheel paint scale
The wheel's paint SHALL multiply its ring, outer ring, frame and label ring by the GUI scale, the interface unit in double times 0.25 stored in single precision.

#### Scenario: Unit 5
- **WHEN** the wheel is painted fully shown at unit 5
- **THEN** its icons sit on a 95-pixel ring and its outer ring is 175 pixels
