## ADDED Requirements

### Requirement: Board tournament entries
The match board SHALL show the career's six tournament rows: each event's entrants, an entrant's id being its company slot (0 the manager) and its first word's item the event value, and the event's winner. The tournament status, the winner column, the forced tournament tab and the tournament lineup SHALL read them.

#### Scenario: An entered event
- **WHEN** the career's third event holds the first employee with item 0x42 and the company as its winner
- **THEN** the board's third entry lists slot 1 with value 0x42, its winner is 1, and the board has entrants

### Requirement: Board event values
A board fighter's value for a tournament event SHALL be the first hand item of its loadout at that event's place, and the weapon check SHALL compare that item's kind.

#### Scenario: A loadout's hand
- **WHEN** a fighter's third loadout holds item 0x77 in its first hand
- **THEN** its third event value is 0x77 and the others are 0

### Requirement: Board fighter tables
A second left press on the same company or lineup row within the GUI's double-press time SHALL move that fighter: a company fighter into the lineup through the rank check, a lineup fighter back. A row dragged from one table and let go over the other SHALL be dropped there, onto the lineup at the row under the pointer or appended; a drop on its own table SHALL do nothing.

#### Scenario: A double press
- **WHEN** a Duel's lineup holds the first employee and the second is double pressed on the company table
- **THEN** the second employee stands in the lineup and the first is back on the company table

### Requirement: Board tournament lineup kept
On the tournament page a fighter joining an event's lineup SHALL rewrite the event's entrants: the count, each id, the event value into the first word's low half and the second word's low half cleared, the high halves kept. A fighter leaving SHALL drop its entrant and move the later ones down. The lineup SHALL be locked on the tournament's own day.

#### Scenario: A drag onto the tournament lineup
- **WHEN** on a day before the tournament the second employee is dragged onto the first event's lineup and then back
- **THEN** the event holds that employee with its item, the reopened board shows it, and after the way back the event is empty

### Requirement: Board lineup order kept
Off the tournament page a change to the lineup SHALL be kept in the staff's order list when the staff matches the board's fighters, and the next board SHALL open on that order.

#### Scenario: The next board
- **WHEN** a double press moves the second employee into a Duel's lineup and the board is opened again
- **THEN** the staff's order list starts with the second employee and the new board's lineup holds it

### Requirement: Season week winners
The week's verdict SHALL record its winner in the season's week slot: 1 for the company, a rival as its field company id plus one, nothing on a tie or fewer than three wins. The fourth week's verdict SHALL clear the four slots. The board's season table SHALL read them, and a save SHALL keep them.

#### Scenario: A rival's week
- **WHEN** the team at field place 1 (company 9) wins three events of the first week
- **THEN** the first week slot holds 10 and the company's week is not won

### Requirement: Report names and hire lines
Manage Company's report SHALL name the fighter of a saved-from-lethal-injury line by its id, 0 being the manager and an employee its place plus one, and its hire lines SHALL count the hire list's professions.

#### Scenario: Two merchants listed
- **WHEN** the log holds saves for ids 1 and 0 and the hire list two merchants
- **THEN** the report names the employee and the manager and says two merchants are available

### Requirement: End-session arena continue
The end-session menu's arena CONTINUE SHALL run the arena's continue: the next round, or the match's settlement and the way back to the hub, as the arena's own end screen's Continue does. It SHALL NOT leave to the title.

#### Scenario: Continue between rounds
- **WHEN** the arena's end-session menu confirms its first row between two rounds
- **THEN** the menu closes, no flow step is queued and the next round starts

### Requirement: Arena leave
Leaving a session in arena mode SHALL run the arena's leave: the hub becomes busy and its click hook is cleared, so no hub click answers until the next hub entry installs it. Other modes' leaves SHALL leave the hub alone.

#### Scenario: Leaving the arena
- **WHEN** an arena session is left with the hub entered and its click hook installed
- **THEN** the hub is busy and has no click hook; a practice leave changes neither
