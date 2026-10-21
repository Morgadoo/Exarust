## ADDED Requirements

### Requirement: Manage window rows
Each roster row of the Manage Company window SHALL show its record's name, a rank label and a health bar. The label SHALL be "Merchant", "Trainer" or "Physician" for those professions, else the rank name with the mark "+" while the fighter is ready to train. Recruit SHALL be disabled at eight fighters (the manager counted) and Hire at ten employees.

#### Scenario: Labels and gates
- **WHEN** a merchant, a ready Novice and a Master who is not ready are listed, with seven and then eight fighters, and nine and then ten employees
- **THEN** the labels are Merchant, Novice with "+", and Master; Recruit is open at seven and closed at eight, Hire open at nine and closed at ten

### Requirement: Roster name ink
A roster name SHALL be drawn in red 255, green 210, blue 100 with a fifth of the skin's alpha times three for the manager; 220, 180, 160 with a third of the alpha times two for a merchant, trainer or physician; 160, 180, 240 with the same alpha for a hired combatant; and in the skin's own text colour for a recruit.

#### Scenario: Alpha 255 and 100
- **WHEN** the alpha is 255 for the manager and a hired combatant, and 100 for a specialist
- **THEN** the alphas are 153, 170 and 66

### Requirement: Report line click
A click on a report line SHALL open the match board's tournament tab for the tournament line; the hire list on the first candidate of the line's profession (or the first candidate when none) for a merchant, physician or trainer line; and the roster on its first row, remembering the fighter id in the line code's top byte, for a training or rank line. Log lines SHALL open nothing.

#### Scenario: A hire list of combatant, trainer, merchant
- **WHEN** the merchant line, the trainer line, the physician line and the training line 0x03140000 are clicked
- **THEN** the hire list opens on candidates 2, 1 and 0, and the roster opens remembering fighter 3

### Requirement: Arsenal form selection
The arsenal form SHALL clamp its selection to its rows. Except on an event's lineup page, Next SHALL be disabled on the last row, and on the first row Previous SHALL be disabled and Dismiss disabled too; off the first row Previous SHALL be enabled, and Dismiss enabled on the roster and specialist pages.

#### Scenario: Three employees
- **WHEN** row 0 is shown, then row 7 is asked for
- **THEN** row 0 disables Previous and Dismiss; row 7 shows row 3 with Next disabled and Previous and Dismiss enabled

### Requirement: Arsenal form steps
A step to another row SHALL load that row's record (a candidate on the hire page, the manager on row 0, else the employee one place up) and SHALL be shown when committed; on the lineup page a step SHALL only be remembered. A step to the row shown SHALL do nothing.

#### Scenario: Next from row 1
- **WHEN** Next is pressed from row 1 of three employees and the step committed
- **THEN** employee 1 is loaded and the commit shows row 2

### Requirement: Arsenal form dismissal
Confirming the dismiss dialog ("Are you sure you want to dismiss " with the name and "?") SHALL load the row above and, when committed, SHALL dismiss the employee of the row shown, rebuild the rows and show the row above.

#### Scenario: Dismiss from row 2
- **WHEN** row 2 is shown and the dismissal confirmed and committed
- **THEN** employee 0 is loaded, employee 1 is dismissed and row 1 is shown

### Requirement: Arsenal form pages
A specialist's page SHALL filter the arsenal by flag 0x200, hide the rank controls and show no loadout tab. The roster, hire and lineup pages SHALL filter by 0x400 and return to the kept tab, the roster and hire pages showing the rank controls. A record SHALL open the specialist's page for a merchant, trainer or physician and the roster page otherwise; the lineup page SHALL keep its page and the hire page SHALL not change it. The hire page SHALL open on tab 5.

#### Scenario: A trainer, then a fighter
- **WHEN** the kept tab is 3 and a trainer's record, then a fighter's, are opened
- **THEN** the trainer's page filters by 0x200 with tab −1, and the fighter's by 0x400 with tab 3

### Requirement: Rank slider and points
The rank slider SHALL name the rank of its value times five, rounded to even in 64 bits and cut to 32 (a NaN names rank 0). The points line SHALL read "Points: " followed by the loadout cost plus one halved, " / " and the tab's loadout limit; below tab 0 the line SHALL not change and the stored cost and limit SHALL be 0 and 100.

#### Scenario: Values and tabs
- **WHEN** the slider is at 0.3, 0.5 and 0.7, and a cost of 7 is shown on tab 0
- **THEN** the ranks are 2, 2 and 4, and the line reads "Points: 4 / 5"

### Requirement: Record texts
A record's title SHALL be its name followed by its profession for a merchant, trainer or physician, or its rank name, in brackets. A candidate SHALL show "Profession: " with its profession and the profession's description. The skills panel SHALL be shown up to a trainer's role.

#### Scenario: Bors
- **WHEN** Bors is a trainer, then an Adept combatant
- **THEN** the titles are "Bors (Trainer)" and "Bors (Adept)"

### Requirement: Company status
Company Status SHALL show the company's name, renown and reputation "Unknown", morale "Good", the staff with the manager, and as masters the manager and the fighters whose rank is above 4, or "None".

#### Scenario: Two masters
- **WHEN** the manager is rank 5 and the staff are a combatant of rank 5, a trainer of rank 5 and a combatant of rank 4
- **THEN** the staff is 4 and the masters 2

### Requirement: Roster drag
A roster row dragged onto another SHALL take that row's place; dropped on no row it SHALL go last, and it SHALL never take the manager's row. The rows between SHALL close up, and every id that moved SHALL be remapped in the log's "saved" records, the tournament entries and the order list's entries up to the staff's count.

#### Scenario: D onto row 2
- **WHEN** of A, B, C, D the row of D is dropped on row 2, with the order list 4, 1, 2, 7, the log saving 4 and 2, and an entry of ids 4 and 3
- **THEN** the staff is A, D, B, C, the order 2, 1, 3, 7, the saves 2 and 3, and the entry 2 and 4

### Requirement: Arsenal sale
An item that leaves the company SHALL leave the arsenal. A unique item (either top id bit set) SHALL leave every box and every holder's record and list. Another item SHALL lose one copy from its shelf: scanning boxes from the last and each from its end, the first copy flagged 0x200, else the first copy found.

#### Scenario: A flagged copy in an earlier box
- **WHEN** box 1 holds the sword flagged 0x200 and flagged 0x400, and box 3 holds it flagged 0x400
- **THEN** box 1 keeps only the copy flagged 0x400 and box 3 keeps its copy

### Requirement: Holders after a sale
After a copy is sold each holder in turn, the manager first, SHALL keep as many copies as were left; the copies past that, from the end of his list, SHALL close up with the list keeping its length, and the allowance SHALL fall by the copies removed. A holder left with fewer than two copies SHALL have the item taken from his loadouts past what he keeps.

#### Scenario: One copy, two holders
- **WHEN** the only copy is sold while the first holder lists the item, 5 and the item, and the second lists the item once
- **THEN** the shelf is empty, the first list is 5, 5 and the item, the second list keeps its entry and both holders' loadouts lose the item

### Requirement: Day's lessons
At the end of the day the first trainer on the staff SHALL teach the manager, then every fighter in staff order. Without a trainer there SHALL be no lessons.

#### Scenario: A trainer, a merchant and a recruit
- **WHEN** the staff are a combatant, a trainer, a merchant and a recruit
- **THEN** the trainer teaches the manager, the combatant and the recruit

### Requirement: Tournament event start
Starting a tournament event SHALL give each event before the first one the company entered, that has no winner and no entrant, the company `2 + high(roll × 6)` with the roll stepped by `× 0x08088405 + 1`. The event SHALL run its rounds less one as extra rounds from round 0, its bracket holding `sides << (rounds − 1)` teams of its team size, team `t` holding id `t + 1`, each slot of strength 1 with its member index.

#### Scenario: Roll 1
- **WHEN** the events are open, won by 5, open, entered, open, open, and the roll is 1
- **THEN** the winners are 2, 5, 7 and the rest stay open, and the roll ends at 0xdc6dac1f; an elimination's bracket holds ids 1 to 16 and a conquest's 16 slots of 8 teams of two

### Requirement: Tourney forfeit
Forfeiting the entire tourney ("Do you wish to forfeit the entire tourney?") SHALL empty every event that has no winner.

#### Scenario: Six events
- **WHEN** events with counts 2, 1, 1, 0, 2, 1 and winners 0, 4, 0, 0, 1, 0 are forfeited
- **THEN** the counts are 0, 1, 0, 0, 2, 0

### Requirement: Hub arrival
Arriving at the hub SHALL remove the physician's mark when a physician is on the staff and SHALL build the tournament board when it holds no prize.

#### Scenario: A physician and an empty board
- **WHEN** the staff holds a physician and the board's prize is 0, then a merchant and a prize of 7
- **THEN** the mark is removed and the board built, then neither

### Requirement: Rematch lineup
The automatic rematch SHALL enter, from the second employee on, those whose class is under 4, until the match's team size is reached or the staff ends.

#### Scenario: Two wanted
- **WHEN** the classes are 2, 8, 1, 2, 2 and two are wanted
- **THEN** employees 2 and 3 enter

### Requirement: Arsenal form boxes
A pressed shelf or box button SHALL store the form's view in the box it showed, then load the same box of the chosen shelf, or the chosen box of the shelf, sorting it first when it was marked and clearing the mark. HIRE SHALL hire the record shown only on the hire page.

#### Scenario: Shelf 3, then box 4
- **WHEN** box 0 of shelf 3 is marked, shelf 3 is pressed from shelf 0 box 0, then box 4
- **THEN** shelf 0 box 0 is stored and shelf 3 box 0 loaded sorted, then shelf 3 box 0 is stored and box 4 loaded unsorted

### Requirement: Event lineup
The board SHALL open an event's lineup only for an event with an entrant, on the first entrant's row, at the loadout tab of the event's place, with the entrant's two weapon words. Closing it SHALL write the record's two hand words back to the first entrant with the second word's item id cleared.

#### Scenario: Event 2
- **WHEN** event 2's first entrant is 3 with weapon words 0x11 and 0x22, and the lineup closes with hands 0x500000007 and 0x900000008
- **THEN** it opens on row 3 at tab 2 with 0x11 and 0x22, and the entrant keeps 0x500000007 and 0x900000000
