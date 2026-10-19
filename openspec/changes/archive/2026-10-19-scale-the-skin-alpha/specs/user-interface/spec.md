## ADDED Requirements

### Requirement: Skin alpha scale
The method SHALL scale the first stored alpha by the horizontal scale into 14 entries and the second by the vertical scale into 48 entries. Each product SHALL round to nearest even and keep the low byte.

#### Scenario: Unit scales leave the alpha
- **WHEN** both stored alpha bytes are 255 and both scales are 1
- **THEN** all 14 and all 48 entries are 255

#### Scenario: A half scale halves the alpha
- **WHEN** the first alpha is 100, the second is 200, and both scales are one half
- **THEN** the 14 entries are 50 and the 48 entries are 100

#### Scenario: A double scale wraps the low byte
- **WHEN** the second alpha is 200 and its scale is 2
- **THEN** the 48 entries are 144
