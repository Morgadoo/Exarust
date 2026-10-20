## ADDED Requirements

### Requirement: Mind mask
The mind mask SHALL start removal on event 2. In state 2 it SHALL clear body bit 0x10000 and be inactive; otherwise it SHALL set the bit and be active.

#### Scenario: On and off
- **WHEN** the mask is not being removed
- **THEN** it sets bit 0x10000 and reports active
- **WHEN** event 2 starts its removal with the flag set
- **THEN** it clears the bit and reports inactive

### Requirement: Mind sense
The mind sense SHALL start removal on event 2. In state 2 it SHALL be active and trigger only while its stored value is above zero; otherwise it SHALL be inactive and not trigger.

#### Scenario: Positive and empty
- **WHEN** the sense is not being removed
- **THEN** it is inactive and does not trigger
- **WHEN** it is being removed with a value above zero
- **THEN** it triggers
- **WHEN** it is being removed with a value of zero
- **THEN** it is active but does not trigger
