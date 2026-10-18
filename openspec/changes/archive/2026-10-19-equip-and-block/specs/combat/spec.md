# Spec Delta

## ADDED Requirements

### Requirement: Guarded blow
A parry SHALL stop the blow: no wound and no health. A shield that is not parrying SHALL deal no lasting wound and yellow 0.05·(1 − stamina)·(0.4 + 0.4·impact)·leak, with leak = 1 − block/2. An open guard SHALL keep the flesh wound.

#### Scenario: Parry
- **WHEN** the same cut that wounds bare flesh is parried
- **THEN** the wound and the yellow damage are zero, and the fighter's health is unchanged

#### Scenario: Tired shield
- **WHEN** that cut meets a shield of block 0.58 with an empty shield arm
- **THEN** the lasting wound is zero and the yellow damage is the leak, less than the flesh wound
