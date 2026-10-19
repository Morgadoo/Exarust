## ADDED Requirements

### Requirement: Parent skin detail
At place 10, 20 of size 100 by 80 with inset 5 and unit 5 the method SHALL prepare once and draw the inset rectangle 15, 25, 90, 70 once. Style bit 0x20000000 with flag bit 0x40 clear SHALL add the horizontal helper at 10, 20, 100, 80 with offset −2 and mode 0x50. Style bit 0x40000000 with flag bit 0x40 set, or flag bit 0x80, SHALL add the vertical helper at the inset rectangle.

#### Scenario: The inset rectangle always draws
- **WHEN** the style is 0 and the flags are 0
- **THEN** one prepare and one rectangle call are made, and no detail helper runs

#### Scenario: The horizontal detail
- **WHEN** style bit 0x20000000 is set and flag bit 0x40 is clear
- **THEN** the horizontal helper runs at 10, 20, 100, 80 with offset −2 and mode 0x50
- **WHEN** flag bit 0x40 is set
- **THEN** it does not run

#### Scenario: The vertical detail
- **WHEN** style bit 0x40000000 and flag bit 0x40 are set
- **THEN** the vertical helper runs at 15, 25, 90, 70
- **WHEN** flag bit 0x80 is set
- **THEN** the vertical helper runs at 15, 25, 90, 70
