## ADDED Requirements

### Requirement: Creation appearance scales
Scaling three values SHALL multiply each by 255 and round to the nearest even integer, keeping the low byte. Scaling two values SHALL multiply each by 65535 and round the same way, keeping the low 16 bits.

#### Scenario: Bytes and words
- **WHEN** the three values are 0, 0.5 and 1
- **THEN** the bytes are 0, 128 and 255
- **WHEN** the two values are 0 and 0.5
- **THEN** the words are 0 and 32768
- **WHEN** the value is 1
- **THEN** the word is 65535
