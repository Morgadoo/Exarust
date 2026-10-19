## ADDED Requirements

### Requirement: Flat slider
A flat slider whose high mode bits are clear SHALL draw four quads. Both caps SHALL use 0.2, 0.8, 0.2, 1. The track SHALL use 0.25, 0.8, 0.25, 1. The thumb SHALL use 0.4, 0.8, 0.4, 1 and sit at the unit plus the spare width times the value, ties rounded even. State bit 0x10 selects ink 0, bit 0x200000 selects ink 1, and any other state selects ink 2. State bit 0x10000 counts a mark. Any other mode selects the ink and draws no flat quads. The scroll bar is not in.

#### Scenario: Unit 5 places the thumb
- **WHEN** unit 5 draws a slider at 10, 20 of width 100 with value 0, then with value one half, then with state bits 0x10 and 0x10000, and a scroll mode is drawn
- **THEN** the first thumb is 5 with pad 10 and ink 2, the half value sets the thumb to 40, the marked state selects ink 0, and the scroll mode draws no flat quads
