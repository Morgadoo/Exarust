## ADDED Requirements

### Requirement: Parent skin rectangle
At place 10, 20 of size 100 by 80 with inset 5 the method SHALL draw one rectangle from 15, 25 to 105, 95 through the virtual, passing flag bits 0x10 and 0x40 as two booleans. Flag bit 0x40 SHALL add one cornered overlay call at the same rectangle.

#### Scenario: The rectangle always draws
- **WHEN** the flags are 0
- **THEN** one rectangle call is made and the overlay does not run

#### Scenario: The flags pass through
- **WHEN** flag bit 0x10 is set
- **THEN** the first boolean is true and the second is false
- **WHEN** flag bit 0x40 is set
- **THEN** the second boolean is true and one overlay call runs at 15, 25, 105, 95
