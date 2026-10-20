## ADDED Requirements

### Requirement: Command-dead control
Event 2 on an applying command-dead SHALL return 2; otherwise it SHALL move the state to 2.

#### Scenario: Event two while applying
- **WHEN** an applying command-dead receives event 2
- **THEN** the result is 2

### Requirement: Command-dead apply and remove
An applying command-dead SHALL move a missing target to state 2; a valid target SHALL apply marker 0x10010d, raise the target's flag and return to state 0. A removing command-dead SHALL remove the marked effect, clear the target's flag and return 2.

#### Scenario: Missing, valid and removed
- **WHEN** the target is missing
- **THEN** the state becomes 2 and nothing applies
- **WHEN** the target is valid
- **THEN** marker 0x10010d applies and the state returns to 0
- **WHEN** the power is removing
- **THEN** the marked effect is removed and the result is 2
