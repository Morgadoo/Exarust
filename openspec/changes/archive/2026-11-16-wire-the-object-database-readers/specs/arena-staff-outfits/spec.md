## MODIFIED Requirements

### Requirement: Loadout equip gate
An arsenal box entry taken onto the preview SHALL be refused when its flags share 0x400, when the item's rank is above the tab's (5 for no tab), or when the loadout's cost plus the item's is above twice the rank's point limit. A weapon SHALL cost its weapon points less what it displaces from the hands; a shield and an apparel their own five-bit points. A weapon's points SHALL be its word's five bits, one less (not below 0) when its flag bits `0x30` are `0x10`.

#### Scenario: A rank 1 weapon at rank 0
- **WHEN** a rank 1 weapon is taken at tab 0
- **THEN** it is refused and the entry keeps its flags

#### Scenario: A one-handed weapon's cost
- **WHEN** a one-handed weapon of word 6 is taken with a one-handed weapon of word 4 in the first hand
- **THEN** it costs 2, and a two-handed weapon of word 9 costs 6
