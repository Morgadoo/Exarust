## ADDED Requirements

### Requirement: Parent skin slider
At place 10, 20 of size 100 by 80 with inset 5, thumb 10 and fraction one half, style 0 SHALL draw the track at 12, 22 of size 96 by 76 and the thumb at 55, 25 of size 10 by 70, translating by −40 then back. Style 0x10000000 SHALL fill 16, 26 of size 88 by 68 and draw a bar at 57, 20 of size 5 by 80. Any other style SHALL draw nothing.

#### Scenario: The plain track
- **WHEN** the style is 0 and unit is 5
- **THEN** the track is 12, 22, 96, 76
- **AND** the thumb is 55, 25, 10, 70
- **AND** the translation is −40
- **AND** flag bit 0x10 selects the first thumb boolean

#### Scenario: The filled track
- **WHEN** the style is 0x10000000 and unit is 5
- **THEN** the fill is 16, 26, 88, 68
- **AND** the bar is 57, 20, 5, 80
- **AND** one begin, one bind and one end are counted

#### Scenario: Another style
- **WHEN** the style's two bits are neither 0 nor 0x10000000
- **THEN** nothing is drawn
