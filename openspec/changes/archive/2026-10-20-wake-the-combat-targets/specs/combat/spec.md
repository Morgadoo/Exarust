## ADDED Requirements

### Requirement: Combat wake
With the wake argument set the method SHALL clear bits 0x6 from the body's 0xdf8 word, wake the body once and call the 0x148 virtual once for each listed target. Without the argument it SHALL do nothing.

#### Scenario: The wake
- **WHEN** the wake argument is set, the body word is 0x7 and the list holds 3 targets
- **THEN** the word becomes 0x1
- **AND** one body wake and three target calls run

#### Scenario: No wake
- **WHEN** the wake argument is clear
- **THEN** nothing runs and the body word is unchanged
