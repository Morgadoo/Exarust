## ADDED Requirements

### Requirement: Creation continue button
The button SHALL be labelled "Continue from save" and set bit `0x40000000` when the mode is 2, and "Continue from checkpoint" with the bit clear otherwise. Bit `0x10` SHALL be set when the mode is not below the threshold.

#### Scenario: Save, checkpoint and threshold
- **WHEN** the mode is 2 and the threshold 3
- **THEN** the label is "Continue from save", the bit is set and the button is disabled
- **WHEN** the mode is 5
- **THEN** the label is "Continue from checkpoint", the bit is clear and the button is enabled
- **WHEN** the mode equals the threshold
- **THEN** the button is enabled
