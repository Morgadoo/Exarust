## ADDED Requirements

### Requirement: Held items' contact mobility
Every tick the arm setter is called, the system SHALL put each held item's contact mobility at the item's base times the setter's held factor: 1 in an attack's or a parry's branch, 2 in every other branch that sets the arms. A shock or a kick SHALL leave the items as they are. The shield SHALL take it whatever its holder holds; the other held items only while the holder is armed (its first held item's class has a hands bit).

#### Scenario: A guard and a swing
- **WHEN** an armed character stands in its guard, then swings
- **THEN** its weapon's contact mobility is twice its base while it stands and its base during the swing

#### Scenario: A torch alone
- **WHEN** a character holds only a torch
- **THEN** the torch keeps its contact mobility

#### Scenario: A shield
- **WHEN** an unarmed character carries a shield and walks
- **THEN** the shield's contact mobility is twice its base
