## MODIFIED Requirements

### Requirement: Damage and reactions
The system SHALL reduce the target's health by an amount that grows with the square of the blade tip's speed. It SHALL shove the struck part with a share of the blade's velocity, daze the body in proportion to the damage, and play a stagger reaction. When health runs out, the body SHALL go limp, keeping the blow's momentum.

#### Scenario: Knock-down
- **WHEN** a fighter's health drops to zero or below
- **THEN** its body goes limp and falls under the blow, it stays down, and it is no longer attacked
