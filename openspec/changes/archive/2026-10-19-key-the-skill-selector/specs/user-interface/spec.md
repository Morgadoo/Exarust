## ADDED Requirements

### Requirement: Skill selector key
The selector SHALL run its base handler when the selection is above zero and the page is above 1, or when the word's low 16 bits are 9 and the mode byte is clear. Otherwise it SHALL close with argument 0 and return 0x40.

#### Scenario: The base paths
- **WHEN** the selection is 3 and the page is 2
- **THEN** the base handler runs and its result returns
- **WHEN** the selection is 0, the word is 9 and the mode byte is 0
- **THEN** the base handler runs and its result returns

#### Scenario: The close path
- **WHEN** the selection is 0 and the word is 4
- **THEN** the form closes and the result is 0x40
- **WHEN** the selection is 0, the word is 9 and the mode byte is 1
- **THEN** the form closes and the result is 0x40
