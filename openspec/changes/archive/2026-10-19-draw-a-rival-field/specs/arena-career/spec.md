## ADDED Requirements

### Requirement: Rival field
A tournament field SHALL take 16 companies from the 19, numbered 1 through 19, and SHALL not repeat one. Each draw SHALL step the roll by multiplying by 0x08088405 and adding 1. The high half of how many remain times that word SHALL choose the bag slot. The id SHALL be one more than the value there. A slot that is not last SHALL close by copying each later value down one.

#### Scenario: Roll 0 and roll 1 leave three companies out
- **WHEN** a field is drawn from roll 0 and another from roll 1
- **THEN** roll 0 yields 1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15 and ends at 0x4b157bf0, and roll 1 yields 1, 17, 5, 7, 14, 8, 4, 10, 11, 2, 13, 3, 18, 6, 12, 19 and ends at 0xead32bb1
