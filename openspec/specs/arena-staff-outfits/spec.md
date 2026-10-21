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
