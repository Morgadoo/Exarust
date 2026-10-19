## ADDED Requirements

### Requirement: Control-code rectangle
The skin SHALL derive four metrics from the unit U and trim T: close 4U + 1, rail 4U − 1, base 6U − T, and tab U / 2 + close. Code −4 SHALL return the close band; −6 and −5 SHALL shift it left by the tab metric selected by flag bits 1 and 3; −9, −10, −13 and −14 SHALL return the rail squares; any other code SHALL return nothing.

#### Scenario: The close band
- **WHEN** unit 5 and trim 2 give close 21, rail 19, base 28 and tab 23, and the place is x 100, y 50, size 20, field a 7 and field b 90
- **THEN** code −4 returns 74, 4, 95, 26
- **AND** code −6 with flag bit 1 returns 51, 4, 72, 26
- **AND** code −5 with flag bits 1 and 3 returns 28, 4, 49, 26

#### Scenario: The rail squares
- **WHEN** the same place and metrics are used
- **THEN** code −9 returns 61, 7, 80, 26
- **AND** code −10 returns 61, 11, 80, 30
- **AND** code −13 returns 20, 11, 39, 30
- **AND** code −14 returns 71, 11, 90, 30
- **AND** any other code returns nothing
