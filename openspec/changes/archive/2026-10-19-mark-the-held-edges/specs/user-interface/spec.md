## ADDED Requirements

### Requirement: Held edges
With the hold bit, style bits 0xc0 equal to 0x80, and no wide bit, a y below the unit SHALL set mask bit 1 and an x below the unit SHALL set bit 2. A right sum past the room SHALL set bit 4, and a bottom sum past the room SHALL set bit 8. An equal sum SHALL set neither. A clear hold bit, a wide bit, and style 0x10 without those bits SHALL set none. The quads after that mask are not in.

#### Scenario: A y of 4 sets bit 1
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, x 4 and y 4, and a right room of 100
- **THEN** the mask is 7, and an equal sum leaves bits 4 and 8 clear
