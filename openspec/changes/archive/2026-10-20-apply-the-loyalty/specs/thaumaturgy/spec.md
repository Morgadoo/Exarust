## ADDED Requirements

### Requirement: Loyalty control
Event 2 on an applying loyalty SHALL return 2; otherwise it SHALL move the state to 2.

#### Scenario: Event two while applying
- **WHEN** an applying loyalty receives event 2
- **THEN** the result is 2 and the state stays applying

### Requirement: Loyalty apply and remove
An applying loyalty SHALL refuse a missing or marked target into state 2; a valid target SHALL apply with marker 0x10010a, set state 0 and return 0. A removing loyalty SHALL remove the target and return 2.

#### Scenario: Refused, applied and removed
- **WHEN** the target is missing or marked
- **THEN** the state becomes 2 and nothing applies
- **WHEN** the target is valid
- **THEN** the effect applies with marker 0x10010a and the state returns to 0
- **WHEN** the loyalty is removing
- **THEN** the target is removed and the result is 2
