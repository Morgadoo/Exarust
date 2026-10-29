## ADDED Requirements

### Requirement: Held strikers gate the executor
The attack executor SHALL start nothing, not even a released swing's overhead, for a fighter that holds no striker. The held count SHALL count each held weapon and each fist or claw that the empty hands hold: 2 unarmed, 1 with one weapon, 2 with two.

#### Scenario: Unarmed and armed fighters attack
- **WHEN** an unarmed fighter (two fists) and a fighter with one sword request an attack once their cooldowns have passed
- **THEN** both start a side attack

#### Scenario: No striker held
- **WHEN** a fighter with no hand strikers requests an attack, free or with a released swing
- **THEN** nothing starts
