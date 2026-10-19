## ADDED Requirements

### Requirement: Span view
Each span-bar corner SHALL be placed as x plus the shift, times the scale, and as the shift minus y, times the other scale. All twelve corners SHALL store the colour word. With shifts 2 and 100, scales one half and 2, and colour 17, the first corner SHALL place at 8.5, 160. An unset view SHALL place that corner at positive zero and negative zero, and store colour 0. The raw quads SHALL stay. The mark written when bit 4 is set is not in.

#### Scenario: A shifted view places the first corner
- **WHEN** unit 5 draws a span from 10, 20 with span 100, stored word 4, shifts 2 and 100, scales one half and 2, and colour 17
- **THEN** the first raw corner stays 15, 20, the placed corner is 8.5, 160, and colour 17 is stored twelve times

#### Scenario: An unset view stores zeros
- **WHEN** the same span is drawn with shifts 0 and 0, scales 0 and 0, and colour 0
- **THEN** the first placed corner is positive zero and negative zero, and colour 0 is stored twelve times
