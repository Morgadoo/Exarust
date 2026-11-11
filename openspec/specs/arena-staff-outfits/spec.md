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

### Requirement: Loadout form drags
A left press over an entry of the loadout form's kit list or arsenal box SHALL start its drag, its icon following the pointer. Released over the preview, an arsenal entry SHALL be taken as the equip rules say and a kit-list entry SHALL dress the preview; released over the arsenal box, a kit-list entry SHALL be returned; released over its own view, the entry SHALL only move there; released anywhere else, it SHALL stay where it was.

#### Scenario: Equip by drag
- **WHEN** a rank 0 weapon is dragged from the arsenal box onto the preview at tab 0
- **THEN** the entry is lent and the kit list holds the weapon

#### Scenario: A drag let go elsewhere
- **WHEN** an arsenal entry is released over the form's records
- **THEN** nothing is taken and the entry keeps its place and flags

### Requirement: Loadout dress from the kit list
A kit-list item released over the preview SHALL be refused when the kit list holds no more copies of it than the shown record's two out hands hold, and otherwise by the rank and points gate; a passing one SHALL be worn (after the equip conflict pass, whose displaced pieces stay in the kit list) or taken into the hands in the tab's loadout, and the kit list SHALL keep its entry.

#### Scenario: One copy already held
- **WHEN** the kit list holds one sword and the tab's loadout holds it in the main hand
- **THEN** dragging the sword onto the preview is refused

### Requirement: Loadout preview take-off
A left press over the preview's figure SHALL pick the item under the pointer and drag it. Released over the kit list, the item SHALL come off the tab's loadout (a worn piece with the underwear rule after it, a held item by the scheduled take-out) and the kit list SHALL keep its copies. Released anywhere but the kit list and the arsenal box, nothing SHALL change.

#### Scenario: A shirt off into the kit list
- **WHEN** the worn shirt is dragged off the preview onto the kit list
- **THEN** the tab's loadout no longer wears it and the kit list still holds it

### Requirement: Loadout preview return
A drag off the preview released over the arsenal box SHALL have the arsenal take the item back as for a return, the kit list lose its first copy, and the item come off the tab's loadout; then every loadout SHALL keep at most the kit list's remaining copies less one in each hand pair (none when the list held none).

#### Scenario: Back to the arsenal
- **WHEN** a held sword with two kit-list copies is dragged off the preview onto its shelf's box that lent it
- **THEN** the lent mark clears, the kit list holds one copy and no loadout keeps the sword in a hand pair that held it once

### Requirement: Arena trade sales
A confirmed trade at the arena's shop or the company merchant SHALL give up each sold item, the sell basket first. A unique item (either top id bit set) SHALL leave every arsenal box and every holder's list, own items and loadouts. Any other item SHALL lose one copy on its shelf (scanning each box from its end, the last box first: the first copy flagged 0x200, else the first found), and each holder in turn keeps at most the copies left.

#### Scenario: Sold world copy
- **WHEN** the manager sells a world copy that an employee also holds in his kit and loadouts
- **THEN** no arsenal box, kit list or loadout names the item any more

### Requirement: Arena trade purchases
A confirmed trade at the arena's shop or the company merchant SHALL make each bought item the world's own copy and stow it with no flags on its shelf's first box. A bought item SHALL NOT join the manager's pack, and an item no shelf takes SHALL be dropped.

#### Scenario: Bought helmet
- **WHEN** a level copy of a helmet is bought at the shop
- **THEN** the arsenal's armour shelf holds its world copy and the pack is unchanged

### Requirement: Roster drag
Manage Company's roster SHALL let an employee's row be dragged: let go over the roster, the employee moves to the row under the pointer (the last row when none, never the manager's), the rows between close up, and the log's saved records, the tournament entrants and the order list take the moved ids.

#### Scenario: Dragged to the last row
- **WHEN** the first of three employees is let go on the third's row
- **THEN** the roster reads the second, the third, then the first

### Requirement: Manage table double press
A double press on a roster row SHALL open the loadout form on that row. A double press on a report line SHALL run the line's action: the tournament line opens the board on its tournament tab, a hire-list line the hire page on the first candidate of its profession, a training or rank line the roster page on its first row.

#### Scenario: Roster row
- **WHEN** the third roster row is pressed twice within the double-press time
- **THEN** the loadout form opens on that employee

### Requirement: Apparel table pass
The outfit tables' apparel rows SHALL take, when their item has a body section, the class its armour type gives when theirs is zero (types 1 and 2 class 1, 3 and 4 class 4, 5 class 8, 6 class 0x40), the rank and points of its rank word, and twenty bonus floats: a non-zero coverage word u gives (padding + slash) × (√(u × 0.01) × 0.05 × 0.5), the bracket in double precision. A zero weight SHALL become ten.

#### Scenario: Coverage floats
- **WHEN** a coat of armour type 5 with padding 1, slash 2 and a first coverage word of 25 fills a row of class 0
- **THEN** the row's class is 8 and its first float is 3 × (√0.25 × 0.025)

### Requirement: Weapon and shield table pass
The outfit tables' weapon and shield rows SHALL take a weight of ten when theirs is zero. When their item has the table's section (a weapon's or a shield's), a zero rank SHALL take the top three bits of the section's rank byte and the cost always its low five bits; a row whose item lacks the section keeps its own rank and cost.

#### Scenario: Shield rank kept
- **WHEN** a shield row of rank 1 names a shield whose rank word is rank 4, five points
- **THEN** the row keeps rank 1 and costs 5
