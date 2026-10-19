## ADDED Requirements

### Requirement: Line rail
At unit 5 and place 10, 20 of size 100 by 80 the rail SHALL draw a left cap from 0, 85 to 30, 105, a right cap from 100, 85 to 90, 105, a middle from 30, 95 to 90, 105, and a thumb from 105, 30 to 95, 40. The cap sample SHALL bind twice, the middle once and the thumb once. Sixteen colour writes SHALL be counted. Bit 0x10 SHALL select the first ink, otherwise the second. Mode 0x10000 SHALL rebind the thumb in the third ink with alpha 255.

#### Scenario: The track and the thumb draw
- **WHEN** the rail is drawn at unit 5, place 10, 20, size 100 by 80, with a clear state and distinct inks
- **THEN** the four quads are those above, the four samples bind in order, and sixteen colour writes are counted

#### Scenario: The thumb ink follows the mode
- **WHEN** the state has mode 0x10000
- **THEN** the thumb uses the third ink with alpha 255
- **WHEN** the state has no mode
- **THEN** the thumb uses the track ink

#### Scenario: Bit 0x10 selects the first ink
- **WHEN** the state has bit 0x10
- **THEN** the track uses the first ink
- **WHEN** the state is 0
- **THEN** the track uses the second ink
