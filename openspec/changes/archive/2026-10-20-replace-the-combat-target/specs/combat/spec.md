## ADDED Requirements

### Requirement: Combat replace
The replace SHALL set the first entry equal to the old target to the new one and leave the list alone when absent. It SHALL replace slots 0x778 and 0x770 when equal, the body's hit pointer with the new target's 0x10 word and its contact pointer with the new target, each when equal to the old.

#### Scenario: The replacement
- **WHEN** the entries are 1, 2, 3, the old target is 2 and the new is 9
- **THEN** the entries become 1, 9, 3
- **WHEN** the old target is absent
- **THEN** the list is unchanged
- **AND** matching slots, hit and contact pointers take the new target
