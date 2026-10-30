# Spec Delta

## ADDED Requirements

### Requirement: Collapse drops the held items from the records
When a character's collapse starts, every hand that does not hold a shield SHALL let go of its item in the character's hands, and the character's possessions record SHALL lose those items from its out set together with their held entries in the carried list; a shield SHALL stay.

#### Scenario: Armed collapse
- **WHEN** a character holding a weapon in the main hand and a shield on the off arm starts to collapse
- **THEN** its main hand is empty, the shield stays, and on the next tick its fists are set up again

#### Scenario: Record drop
- **WHEN** a character whose record holds a sword in the main hand starts to collapse
- **THEN** the record's main hand is empty and the sword's held entry is gone from the carried list

### Requirement: Unarmed set-up on a rise
A rising character SHALL be set up unarmed exactly when both out-set hands of its possessions record are empty; a character without a record SHALL be read from its hands, where a shield or an off-hand light fills the off hand. A rise SHALL end the collapse, so a later collapse drops again.

#### Scenario: Dropped weapon
- **WHEN** a character that collapsed with a sword rises
- **THEN** it is set up unarmed

#### Scenario: Kept shield
- **WHEN** a character that collapsed with a sword and a shield rises
- **THEN** it is not set up unarmed
