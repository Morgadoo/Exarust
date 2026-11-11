## ADDED Requirements

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
