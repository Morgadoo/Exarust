## ADDED Requirements

### Requirement: Transition fade
On page 2 at 200, 50 with a glyph of 8 by 16, colour 10, 20, 30 and alpha 255, the transition SHALL draw "TO BE CONTINUED" at 40, 17 with blend mode 0x203 and one begin and one draw. Any other page SHALL draw nothing but still call the base draw.

#### Scenario: The fade page
- **WHEN** the page is 2
- **THEN** the text is "TO BE CONTINUED" at 40, 17
- **AND** the colour is 10, 20, 30, 255 and the blend mode is 0x203
- **AND** one begin, one draw and one font selection are counted

#### Scenario: Another page
- **WHEN** the page is not 2
- **THEN** nothing is drawn and the base draw is still called
