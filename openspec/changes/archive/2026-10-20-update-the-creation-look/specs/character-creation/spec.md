## ADDED Requirements

### Requirement: Creation look
Updating the look SHALL set its four values to 16 and the default caption when female, and otherwise set them to 0. A variant below 0 SHALL keep the default caption; otherwise the caption SHALL be the byte `variant + 0x41`.

#### Scenario: Female, lettered and default
- **WHEN** the look is female with variant 0
- **THEN** the four values are 16 and the caption is the default
- **WHEN** the look is male with variant 2
- **THEN** the four values are 0 and the caption is C
- **WHEN** the look is male with variant −1
- **THEN** the caption is the default
