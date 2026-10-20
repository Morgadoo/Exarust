## ADDED Requirements

### Requirement: Rank roll rounding
The system SHALL roll a template's rank with round-to-nearest-even, not truncation.

#### Scenario: Elimination at slot three
- **WHEN** Elimination is rolled at slot 3 with r 0.5
- **THEN** its rank is 3 and its window is 3–3

### Requirement: Company profile order
The system SHALL store each company's three build bytes in memory order `[+8,+9,+10]`.

#### Scenario: Fitten Gang
- **WHEN** the second company is read
- **THEN** its bytes are 0x42, 0x8C, 0x01
