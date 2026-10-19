## ADDED Requirements

### Requirement: Second skin slider
At place 10, 20 of size 100 by 80 with inset 5, thumb 10, fraction one half and unit 5, style 0 SHALL draw the track at 11, 22 of size 98 by 76 and the thumb at 55, 26 of size 10 by 68, translating by −40 then back. Style 0x10000000 SHALL fill 16, 26 of size 88 by 68 and draw a bar at 57, 20 of size 5 by 80. Any other style SHALL draw nothing.

#### Scenario: The plain arm
- **WHEN** the style is 0
- **THEN** the track is 11, 22, 98, 76
- **AND** the thumb is 55, 26, 10, 68
- **AND** the translation is 40 with flag bit 0x10 as the first boolean

#### Scenario: The filled arm
- **WHEN** the style is 0x10000000
- **THEN** the fill is 16, 26, 88, 68
- **AND** the bar is 57, 20, 5, 80
- **AND** one begin, one bind and one end are counted

#### Scenario: Another style
- **WHEN** the style is 0x20000000
- **THEN** nothing is drawn
