## ADDED Requirements

### Requirement: Combatant reset
The reset SHALL zero the count, the 0x770/0x788/0x798 qwords, the 0x200/0x158 words, the 0x87e byte and the body's 0xe38/0xe50 pointers, and set 0x7a0 to −1. It SHALL clear a marker's two words and decrement a reference's count before clearing it.

#### Scenario: The reset
- **WHEN** a marker and a reference of count 3 are present
- **THEN** the listed fields are zero, 0x7a0 is −1, the marker clears and the reference becomes 2 with its pointer cleared
- **WHEN** neither is present
- **THEN** the fields still reset and neither path runs

### Requirement: Combatant measures
The measure pair SHALL store the given reach at 0x7b0 and the body value times 1.0 at 0x7b8.

#### Scenario: The measures
- **WHEN** the reach is 12.5 and the body value is 0.75
- **THEN** 0x7b0 is 12.5 and 0x7b8 is 0.75
