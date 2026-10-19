## ADDED Requirements

### Requirement: Bevel box
The bevel SHALL draw two gradient fill quads, six border lines and four outline lines. Bit 0x40 of the state SHALL select the raised arm, else bit 0x10 the pressed arm, else the idle arm. The insets SHALL be literal pixels and SHALL NOT follow the unit. Three begins, twelve binds and three ends SHALL be counted.

#### Scenario: The idle bevel draws both quads, six lines and four lines
- **WHEN** a bevel is drawn at 10, 20 of size 100 by 80 with a clear state and distinct idle inks
- **THEN** the fill runs 12, 22 to 108, 98 as two quads with the seam at 60
- **AND** the fill stops are 0.6 and 0.4
- **AND** the border runs 11.5, 21.5 to 108.5, 98.5 as six lines with stops 0.65 and 0.35, the left seam at 60.5 and the right seam at 60
- **AND** the outline runs 10.5, 20.5 to 109.5, 99.5 as four lines
- **AND** the counts are 3, 12 and 3

#### Scenario: The state selects the arm
- **WHEN** the bevel is drawn with bit 0x40 set
- **THEN** the raised inks are used
- **WHEN** it is drawn with bit 0x10 and bit 0x40 clear
- **THEN** the pressed inks are used

#### Scenario: The shipped inks shade only the pressed arm
- **WHEN** the bevel is drawn with the shipped inks
- **THEN** the raised and idle arms are fully transparent, and the pressed arm is black with alpha 0xE0
