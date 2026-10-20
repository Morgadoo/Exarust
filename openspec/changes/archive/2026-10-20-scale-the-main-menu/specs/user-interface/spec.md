## ADDED Requirements

### Requirement: Main-menu row scale
The system SHALL derive `row_h` as `0.5·round(H/135)/4` and advance rows by `trunc(70·row_h)`.

#### Scenario: Live heights
- **WHEN** the client height is 1080
- **THEN** `row_h` is 1.0 and the advance 70
- **WHEN** the height is 453
- **THEN** `row_h` is 0.375 and the advance 26
- **WHEN** the height is 720
- **THEN** `row_h` is 0.625
