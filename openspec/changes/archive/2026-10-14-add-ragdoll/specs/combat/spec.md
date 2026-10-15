## MODIFIED Requirements

### Requirement: Damage and reactions
The system SHALL reduce the target's health by an amount that grows with the square of the blade tip's speed, play a stagger reaction, and make the body go limp when health runs out.

#### Scenario: Knock-down
- **WHEN** a fighter's health drops to zero or below
- **THEN** its body goes limp and falls, it stays down, and it is no longer attacked
