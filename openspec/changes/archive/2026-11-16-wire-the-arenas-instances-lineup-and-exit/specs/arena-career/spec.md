## ADDED Requirements

### Requirement: Season rewards as instances
A swept season's four rewards SHALL each be made a fresh item instance before the arsenal stows it with flags 0: the record instanced through the object database's dispatcher and, when the instance has an item body, its quality byte 255, damage, dirt and the three packed spreads 0, then its description refreshed from its template.

#### Scenario: A swept season
- **WHEN** the fourth week of a season the company swept ends through Manage Company's End Day with the season's records in the live database
- **THEN** the arsenal holds four local instances of those records with the condition bytes 255, 0, 0, 0 and the base records are unchanged

### Requirement: Week prize as a graded copy
A won week's prize SHALL reach the arsenal as the prize graded pristine (a factory weapon copied into the local tier at the grade of -1, any other prize its own id). Without game data the drawn ids SHALL be stowed as they are.

#### Scenario: A won week without game data
- **WHEN** a week the company won ends and no base tier is loaded
- **THEN** the prize's own id is stowed in the arsenal with flags 0

### Requirement: Arena load id pass
After an arena slot's chunks are read and before the continue fixups, every arsenal entry's id SHALL go through the instancer, an untagged base record with the instance bit becoming a local instance. An entry whose id changed SHALL remap its holders: with 0x400 the stash then the manager's loadouts, else the first employee whose kit holds it and his loadouts; with 0x200 the manager's hands and pack, then each employee's.

#### Scenario: A flagged base item
- **WHEN** an arena slot is loaded whose arsenal holds a 0x400 entry and a stash item of a base record with the instance bit
- **THEN** the entry and the stash item hold the same new local instance naming that record

### Requirement: Weapon restriction pane
The tournament tab of a weapon event SHALL show the weapon restriction pane's three records: the "Weapon Restriction" caption, the weapon row's name and a Choose button, moved up in the compact form. The button SHALL be disabled on the tournament's own day.

#### Scenario: Choose clicked
- **WHEN** the tournament tab shows a weapon event on a day before the tournament and its Choose button is clicked
- **THEN** the board asks for the selected event's lineup, and on the tournament's day the same click asks for nothing

### Requirement: Event lineup from the board
Choose SHALL open the loadout form on its event page at the selected event's first entrant's row and the event's rank, the view's two out hands holding the event's two weapon words. The form's close SHALL write the out hands back to the event's first entrant, the second word's item id cleared, give the loadout its own hands back and refresh the board's weapon mark.

#### Scenario: A chosen weapon
- **WHEN** Choose opens the lineup of an event whose first entrant is the second employee and the form closes with item 0x99 in the main hand
- **THEN** the event's first entrant holds 0x99 with its flags in the first word, the second word's item is 0, and the employee's loadout has its own hands

### Requirement: Lineup reorder
A lineup row dropped on the lineup SHALL move within it: a marked fighter swaps with the first marked teammate, a drop on a row swaps the two, a drop past the rows moves the fighter to the end. The Captain rules and the event's entrants or the lineup order SHALL follow as for a company fighter's drop.

#### Scenario: Two fighters swapped
- **WHEN** the first lineup row of a Doubles is dragged onto the second
- **THEN** the two fighters change places and the staff's order list follows

### Requirement: Winner names
The tournament winner cell SHALL name the field's company c by the company names' entry c - 1, and the season table SHALL name a week slot s by entry s - 2, a rival's slot being the field's word plus two.

#### Scenario: A rival's week
- **WHEN** the team at field place 1 (word 9) wins three events of the first week
- **THEN** the first week slot holds 11

### Requirement: Manager's fall
When the manager falls in a match the heartbeat SHALL fade, every game screen and the power tree's bar SHALL close, the fall's tip SHALL show (3 fatal, else 2) and the controls SHALL be blocked. With a company fighter still standing the spectate session SHALL start with its room walk forced to the arena's spawn spot.

#### Scenario: A fatal fall with a teammate standing
- **WHEN** the manager dies while a company fighter stands
- **THEN** the steps hold the heartbeat's fade and tip 3, and the spectate session runs forced to the spawn's room

### Requirement: Arena exit
The in-game page's EXIT in the arena SHALL close the screens and the power tree's bar, send the first unfit specialist away, save the career without the death rule, run the arena's leave and leave to the title behind the transition.

#### Scenario: Exit with an unfit physician
- **WHEN** EXIT is chosen with a physician at health 500
- **THEN** the physician leaves the roster, a save without the death rule is asked for and the hub is busy with no click hook

### Requirement: Specialist leaving
A merchant's leaving SHALL empty his stock. A merchant's or a physician's leaving SHALL clear the items of the first character with that profession nibble (4 or 5), remove its body and let the hub place a new hire of that profession.

#### Scenario: A merchant leaves
- **WHEN** the staff's merchant leaves while his figure stands in the hub
- **THEN** the merchant stock is empty, the figure's items are cleared and its body is asked away

## MODIFIED Requirements

### Requirement: Season week winners
The week's verdict SHALL record its winner in the season's week slot: 1 for the company, a rival as the field's word at the winner's team plus two, nothing on a tie or fewer than three wins. The fourth week's verdict SHALL clear the four slots. The board's season table SHALL read them, and a save SHALL keep them.

#### Scenario: A rival's week
- **WHEN** the team at field place 1 (word 9) wins three events of the first week
- **THEN** the first week slot holds 11 and the company's week is not won
