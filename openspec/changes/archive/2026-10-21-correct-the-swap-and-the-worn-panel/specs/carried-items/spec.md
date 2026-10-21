## MODIFIED Requirements

### Requirement: Weapon set swap
A character's possessions SHALL hold an out and a stowed weapon set of a main and an off hand, and a carried list in which an item in a hand stays listed with its held bit 0x100. Swapping the sets SHALL toggle the record's second-set flag 0x0100_0000, unmark each going item's held entries once per hand it fills less the incoming hands that hold it, and mark the incoming items held the same way, a two-handed item counting once. The sets SHALL trade places only when a hand's item changes.

#### Scenario: A sword for a two-handed axe
- **WHEN** a sword is out and a two-handed axe fills both stowed hands, and the sets are swapped and swapped back
- **THEN** after the first swap the sword's entry is unmarked, both axe entries are held and the flag is set; after the second the marks and the flag are as before

#### Scenario: An item in both sets
- **WHEN** the same item is in the out main hand and the stowed main hand
- **THEN** a swap keeps its held mark and the main hand does not change

#### Scenario: The same items in both sets
- **WHEN** both sets hold the same sword in the main hand, the stowed one marked lit, and the sets are swapped
- **THEN** the flag is set, no hand plan is made and the out sword stays unlit
