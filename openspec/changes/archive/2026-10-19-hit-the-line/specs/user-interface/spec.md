## ADDED Requirements

### Requirement: Line hit test
The hit test SHALL use a span of four units, or six when the wide switch is set. The trim test SHALL run when flags masked with 0x30 equal 0x10 and y is at or above the span. The strip test SHALL run otherwise when control bit 0x200 is set and x is past the left limit. The returned code SHALL be the original's unchanged.

#### Scenario: The trim test returns its codes
- **WHEN** the flags are 0x10, the control word has bit 2, the right limit is 100, the span is 20 and the point is 90, 10
- **THEN** the code is -4
- **WHEN** the point is 50, 10 with control bit 2 and no other bit
- **THEN** the code is 0
- **WHEN** the point is 10, 10 and control bit 1 is set
- **THEN** the code is -3

#### Scenario: The strip test returns its codes
- **WHEN** the control word has bit 0x200, x is past the left limit, and y is between the top and bottom limits
- **THEN** the code is -8
- **WHEN** y is at or past the bottom limit but below it plus the span
- **THEN** the code is -10
- **WHEN** y is at or below the top limit and above it minus the span
- **THEN** the code is -9

#### Scenario: Every other point is 0
- **WHEN** the flags and control word select no test
- **THEN** the code is 0
