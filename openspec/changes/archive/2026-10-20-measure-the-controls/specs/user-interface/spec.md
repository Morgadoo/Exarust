## ADDED Requirements

### Requirement: Control text spans
The system SHALL compute the three spans `w−2pad−2U`, `w−(4U|1)−2pad−U` and `w−(3U|1)−2pad−2U`, keeping the `|1` bit.

#### Scenario: Unit four and five
- **WHEN** w is 100, pad 2 and unit 4
- **THEN** the spans are 88, 75 and 75
- **WHEN** the unit is 5
- **THEN** the third span is 71

### Requirement: Trackbar geometry
The system SHALL map a cursor to `(cursor−x−pad−span/2)/travel`, clamp to 0..1, quantise by the step count, place the thumb at `x+pad+rint(travel·value)` and centre the caption in the knob slot.

#### Scenario: Half value
- **WHEN** the cursor is 50 in a 100-wide track with pad 2 and span 20 at x 0
- **THEN** the value is 0.5 and the thumb left is 40
- **WHEN** the value is 0.6447 with four steps
- **THEN** the value is 0.75

### Requirement: Control lists
The system SHALL use a `6U` row, a `3U` separator, a clamped 1-based row hit, non-wrapping ±1 navigation, a 16-row extent with `1/(count−16)`, and the `2·inset + widest + 16U` width rule.

#### Scenario: Row and extent
- **WHEN** the unit is 4 and inset 3, a y of 30 with top 3 gives row 2
- **THEN** a 20-item list has height 390, content 480 and page 0.25
- **WHEN** the list has 16 items
- **THEN** it has no page fraction

### Requirement: Control activation
The system SHALL clear `0x100` on every radio item and set it on the chosen one; a checkable item SHALL report the inverse of its checked bit.

#### Scenario: Radio and check
- **WHEN** a radio item is activated
- **THEN** all radio items lose `0x100` and the chosen one gains it
- **WHEN** a checked checkable item is activated
- **THEN** the result is 0, else 1
