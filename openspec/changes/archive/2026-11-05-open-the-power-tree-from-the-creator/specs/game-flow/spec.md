## ADDED Requirements

### Requirement: Story start power writes
The story start's power writes SHALL put the power word into the record's power word and the value into the potential kept beside the power slots, never the word into the potential.

#### Scenario: The unknown background
- **WHEN** a new story starts for the unknown background on a record that knows powers
- **THEN** the record's power word is 0x510000 and its potential 4.0
