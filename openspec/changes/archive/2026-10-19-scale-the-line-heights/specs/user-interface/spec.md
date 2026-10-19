## ADDED Requirements

### Requirement: Line heights
The scale method SHALL store three heights from 80, 104 and 128 times the vertical scale, rounded to nearest even and keeping the low byte. It SHALL count one call to the parent's scale.

#### Scenario: Unit scale leaves the heights
- **WHEN** the vertical scale is 1
- **THEN** the heights are 80, 104 and 128

#### Scenario: A half scale halves the heights
- **WHEN** the vertical scale is one half
- **THEN** the heights are 40, 52 and 64

#### Scenario: A double scale wraps the low byte
- **WHEN** the vertical scale is 2
- **THEN** the heights are 160, 208 and 0
