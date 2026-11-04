# Spec Delta

## ADDED Requirements

### Requirement: Loadout form item views
The hub loadout form on the roster page SHALL show, at GUI unit 4 on a 1024 by 768 form, the shown person's kit list at (4, 180) 228 by 492, the arsenal box of the chosen shelf and box 236 by 628 at y 88 four pixels from the form's right edge, and the character preview from x 236, 544 wide; every place SHALL scale by the unit over 4.

#### Scenario: Unit 3
- **WHEN** the form is 768 by 576 at unit 3
- **THEN** the arsenal box is at (588, 66), 177 by 471

### Requirement: Loadout kit list
The loadout form's kit list SHALL be the hub stash for the manager and the employee's own list otherwise, and an entry the shown rank wears or holds in its first two hands SHALL carry the equipped mark 0x200.

#### Scenario: Marks
- **WHEN** the rank holds item 5 in its second hand, item 6 in its third and wears item 7
- **THEN** the list's items 5 and 7 carry 0x200 and items 6 and 8 do not

### Requirement: Loadout equip gate
An arsenal box entry taken onto the preview SHALL be refused when its flags share 0x400, when the item's rank is above the tab's (5 for no tab), or when the loadout's cost plus the item's is above twice the rank's point limit. A weapon SHALL cost its points less what it displaces from the hands; a shield and an apparel their own five-bit points.

#### Scenario: A rank 1 weapon at rank 0
- **WHEN** a rank 1 weapon is taken at tab 0
- **THEN** it is refused and the entry keeps its flags

### Requirement: Loadout equip from the arsenal
An accepted arsenal entry SHALL be lent (0x400), the item SHALL join the kit list, and the tab's loadout SHALL take it into its hands or, for an apparel, onto its worn list after the conflict pass.

#### Scenario: Within the limit
- **WHEN** a rank 0 weapon of 4 half points is taken at tab 0 with an empty loadout
- **THEN** the entry carries 0x400, the kit list holds the item and the main hand holds it, for a cost of 4 of 10

### Requirement: Loadout return to the arsenal
A kit-list entry dropped on the arsenal box SHALL clear the lend from the shown box's first entry lent with the item when the item's shelf is the one shown; otherwise the arsenal's lend query SHALL be asked. An item neither took back SHALL join the shown box with no flags when its shelf is shown, else go onto its own shelf's first box with no flags.

#### Scenario: Not lent, shelf shown
- **WHEN** nothing holds the item lent and its shelf is shown
- **THEN** the shown box gains a flagless entry of it

### Requirement: Loadout trim on return
A returned item SHALL leave the kit list, and each of the six loadouts SHALL lose its first worn copy of the item, or keep at most as many hand copies as the kit list still holds.

#### Scenario: Lent in another box
- **WHEN** box 3 of the weapon shelf holds the item lent and box 0 is shown, and the kit list holds two copies
- **THEN** box 3's entry loses 0x400 and a loadout holding it in both hands keeps the first

### Requirement: Saved per-rank kits
The arena save SHALL write the manager's record with his six per-rank loadouts into the world character database, and the arena load SHALL give the career's manager those six loadouts back.

#### Scenario: Round trip
- **WHEN** rank 2's loadout holds a second-hand item and a worn item and the hub is saved and loaded
- **THEN** the loaded manager's six loadouts equal the saved ones
