## ADDED Requirements

### Requirement: Line chip
At unit 5 and place 10, 20 the chip SHALL draw (10, 40), (10, 20), (30, 20), (30, 40). The first sample SHALL be 0.75, 0.8, 0.75, 1, then 0.85, 1, 0.85, 0.8. Flag 0x40 SHALL bind the second sample, 0.85, 0.8, 0.85, 1, then 0.95, 1, 0.95, 0.8, and draw the same corners again. Flag 0x10 SHALL select the first ink. Mode 0x10000 SHALL select the second ink with alpha 255. Any other flag word SHALL select the third ink.

#### Scenario: A plain chip draws one sample
- **WHEN** the chip is drawn at unit 5, place 10, 20, with a clear flag word and three distinct inks
- **THEN** the corners are those above, the first sample binds once, the second is absent, one draw is counted, and the third ink is used

#### Scenario: Bit 0x40 draws the second pass
- **WHEN** the same chip is drawn with flag 0x40
- **THEN** the second sample binds after the first, and the same corners are drawn twice

#### Scenario: The inks follow the flag word
- **WHEN** the chip is drawn with flag 0x10
- **THEN** the first ink is used
- **WHEN** the chip is drawn with flag 0x10000
- **THEN** the second ink is used with alpha 255
