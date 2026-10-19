## ADDED Requirements

### Requirement: Skin label placement
The label SHALL align from style bits 0x7000 and 0x700 exactly as the line label. Bit 0x10 SHALL select the first ink. Mode 0x10000 SHALL select the coloured renderer with a one-pixel nudge on bit 0x40. Otherwise bit 0x40 SHALL draw a shadow one pixel left in the second ink and a second call one pixel down in the third ink. Any other word SHALL draw once in the fourth ink.

#### Scenario: The shadow arm draws twice
- **WHEN** a label of width 40, line height 24 and base 18 is placed at 10, 20 of size 100 by 80 at unit 5 with state 0x40
- **THEN** the first call is one pixel left at 12, 23 in the second ink
- **AND** the second call is one pixel down at 13, 24 in the third ink

#### Scenario: The single arms
- **WHEN** the state has bit 0x10
- **THEN** one call is made at 13, 23 in the first ink
- **WHEN** the state is 0
- **THEN** one call is made at 13, 23 in the fourth ink
- **WHEN** the state has mode 0x10000 and bit 0x40
- **THEN** the coloured renderer is used at 13, 24
