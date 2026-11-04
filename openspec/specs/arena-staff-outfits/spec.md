# arena-staff-outfits Specification

## Purpose
The arena career's staff: fighter rank, saved item ids, the hire fee, the roster and loadout form rules, the icon atlas, and the saved special match and weapon-row checks.

## Requirements

### Requirement: Fighter rank
The fighter rank SHALL be the sum of the six skill experience words, each divided by 5000 with the remainder dropped, halved with the remainder dropped, and capped at five.

#### Scenario: Caps and floors
- **WHEN** the six words are all 0
- **THEN** the rank is 0
- **WHEN** the words are 5000, 5000, 0, 0, 0, 0
- **THEN** the rank is 1
- **WHEN** the six words are all 65000
- **THEN** the rank is 5

### Requirement: Save id remaps
Before a save every arena item SHALL be given its saved id in the order manager, stash, arsenal, then each employee, and within a record hands, worn items, loadouts, carried items. After a load an arsenal entry flagged for an employee SHALL remap the stash or the first employee whose own list holds the id, with each first match ending its search, and one flagged for a character SHALL remap hands and carried items only.

#### Scenario: Stash match first
- **WHEN** an employee-flagged entry's id is held in both the stash and an employee's list
- **THEN** the stash item is replaced, the manager's loadouts are remapped and the staff is not walked

### Requirement: Hire fee
The hire fee SHALL be the rank plus one, squared, times 12, plus four fifths of the kit's copper value rounded to the nearest copper.

#### Scenario: A rank 2 candidate
- **WHEN** a candidate of rank 2 carries a kit worth 100 copper
- **THEN** the fee is 188 copper

### Requirement: Roster health fill
A roster row's health fill SHALL be the health word multiplied by one over 65535 in double precision and narrowed once to single precision.

#### Scenario: A word a single-precision product misses
- **WHEN** the health word is 257
- **THEN** the fill's bit pattern is 0x3b808081

### Requirement: Loadout form binding
Binding a record to the loadout form SHALL repair a fighter whose loadout count is not six, copy the record only on the hire page, apply the kept rank's loadout only when the form's tab is below the repaired count, and give the record the event's saved hands only on the event page.

#### Scenario: A short fighter record
- **WHEN** a fighter with four loadouts is bound with tab 5 and kept rank 5
- **THEN** it is repaired and the kept rank's loadout is applied

### Requirement: Loadout form opening
Opening the form SHALL clamp the selection to the list. The roster and event pages SHALL show the manager at selection 0 and the employee one before the selection otherwise; the hire page SHALL show the candidate at the selection; the event page SHALL also fix the event's rank. The second pane SHALL be reloaded from the stash at selection 0 and from the selected employee's list otherwise, and the hire page SHALL keep its containers.

#### Scenario: An event page
- **WHEN** the event page opens at selection 2 for rank 5
- **THEN** the record is the first employee, the rank is 5 and the second pane lists that employee's items

### Requirement: Icon atlas
The arsenal icon atlas SHALL start as one cell and, each time it is full, double its height when it is as wide as it is tall and its width otherwise, laying cells out again in row order.

#### Scenario: Seven icons
- **WHEN** seven 32-pixel icons are added
- **THEN** the atlas is two cells wide and four tall, 64 by 128 pixels

### Requirement: Saved special match
Restoring a saved special match SHALL take the saved rank only when the rolled rank is positive and the template lacks flag 2, set the rank window from the roll for every template but the Challenger, and always take the saved flags.

#### Scenario: A Challenger
- **WHEN** a Challenger match with rolled rank 4 is restored with saved rank 2 and flags 0x10
- **THEN** the rank stays 4, the window is unchanged and the flags are 0x10

### Requirement: Weapon row check
A weapon row SHALL hold an item only when the item appears before the row's first empty entry.

#### Scenario: Entry after a gap
- **WHEN** the row is 3, 0, 7 and the item is 7
- **THEN** the row does not hold it

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
