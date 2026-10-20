## ADDED Requirements

### Requirement: Save number format
Formatting SHALL give `"000"` for zero or a negative value. Otherwise it SHALL write the decimal digits and pad on the left with zeros to three characters, leaving longer values whole.

#### Scenario: Zero, short and long
- **WHEN** the value is 0
- **THEN** the text is 000
- **WHEN** the value is 5
- **THEN** the text is 005
- **WHEN** the value is 12
- **THEN** the text is 012
- **WHEN** the value is 1234
- **THEN** the text is 1234
- **WHEN** the value is −7
- **THEN** the text is 000
