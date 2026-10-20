## ADDED Requirements

### Requirement: Arena instance roll
The system SHALL copy a template into a match instance, roll its rank from the MT unit draw with nearest-even rounding, set the Challenger window and draw a weapon row for flag-4 templates on the LCG.

#### Scenario: Elimination and Challenger
- **WHEN** Elimination rolls at slot 3 with r 0.5
- **THEN** its rank is 3, its window 3–3 and its cash is the slot-scaled packed value
- **WHEN** Challenger rolls at slot 0
- **THEN** its window is 0–5
- **WHEN** Brawl rolls at slot 4
- **THEN** its rank is −1

### Requirement: Arena daily board
The system SHALL generate six daily rows from the seven ranked templates with the cap, minimum-slot and reject filters.

#### Scenario: Seed 12345
- **WHEN** the daily board is generated from seed 12345
- **THEN** the template ids are 8, 13, 8, 11, 13, 12

### Requirement: Arena tournament board
The system SHALL draw the weekly prize around the eight-index history, generate six tournament rows and shuffle the 19 companies into 16 slots on the same LCG.

#### Scenario: Shuffle
- **WHEN** the companies are shuffled from seed 0
- **THEN** the order is 1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15
