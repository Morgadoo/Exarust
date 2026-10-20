## ADDED Requirements

### Requirement: Barrier update fraction
The barrier update SHALL compute its fraction as (target frame − base frame) + extra and its position as a + (b − a)·t, with a the current pair and b the previous pair.

#### Scenario: One tick
- **WHEN** the current is (0, 0, 0), the previous (10, 0, 0), the frames 5 and 3, and the extra 0.1
- **THEN** the fraction is 2.1 and the position (21, 0, 0)

### Requirement: Barrier padding
The barrier's stored padding SHALL be 0.2.

#### Scenario: The constant
- **WHEN** the padding is read
- **THEN** it is 0.2
