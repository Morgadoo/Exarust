## ADDED Requirements

### Requirement: First-hit scaling
The hit SHALL return unchanged for a blocked or helpless combatant. Otherwise a first hit SHALL scale by `(1 + stat) × 0.025 + 0.95` in double precision. A target lacking bit 0x100000 SHALL take the threshold when its hit is lower and gain the bit. A body with that bit SHALL make one special call.

#### Scenario: The scaling
- **WHEN** the hit is first, the current value is 10 and the stat is 0
- **THEN** the scaled value is 9.75
- **WHEN** the stat is 1
- **THEN** the scaled value is 10
- **WHEN** the hit is not first
- **THEN** the current value is unchanged

#### Scenario: The target mark
- **WHEN** the target flag lacks 0x100000, its hit is 3 and the threshold is 5
- **THEN** its hit becomes 5 and the flag gains 0x100000
- **WHEN** the target already has the bit
- **THEN** its hit is unchanged
- **WHEN** the combatant is blocked or helpless
- **THEN** nothing changes
