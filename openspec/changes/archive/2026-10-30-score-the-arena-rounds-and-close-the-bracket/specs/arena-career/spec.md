## ADDED Requirements

### Requirement: Combat score
Every fighter SHALL keep a combat score starting at 0. A wound SHALL add 3 to the striker's score; a clash or block SHALL add 1 to the struck fighter's score only while it is parrying.

#### Scenario: Wounds and guards
- **WHEN** a fighter lands two wounds, and another takes a clash and a block while parrying and a block while not
- **THEN** the first scores 6 and the second 2

### Requirement: Live round experience
At every round's end the arena SHALL compute the experience pool from the scene's bodies (the match's slot as the tier) and grant each company body with a record its share as a skill lump, clearing that body's score. A company fighter who died that tick SHALL get nothing.

#### Scenario: Manager credited
- **WHEN** a slot-3 Duel ends with the opponent missing half its health, the manager having scored 3 and a roster fighter having died
- **THEN** the manager's trained skill gains 875 and the roster fighter's nothing

### Requirement: Bracket end
A tournament's end or forfeit SHALL draw the scene's winning team, clear the brackets again with no forced team while a pass resolves more than one group, and give the first event without a winner the first surviving team.

#### Scenario: Four teams
- **WHEN** teams 5, 9, 3 and 7 of strengths 4, 10, 4 and 2 end with team 5 drawn and roll 1
- **THEN** only team 3 survives, the roll ends at 0x33dc589c and the first open event takes team 3

### Requirement: Arena reset
Loading an arena slot SHALL first clear the shop's shelf, the merchant's stock (kinds 0, rank −1), the company's records and day tallies, the day, today's matches, the log, the roster, the purse, the week verdicts, the stash, the arsenal and the staff, keeping the company's name and the tournament entries.

#### Scenario: Loaded after play
- **WHEN** a career with a hire, stock, day 9 and an entry is reset
- **THEN** the roster and stock are empty, the day and purse are 0, the merchant rank is −1, and the name and entry stay

### Requirement: Auto rematch
With the auto-match switch on, 40 timer ticks of 100 ms after the return to the hub the last board match SHALL start again with the recruits and combatants after the manager, as many as a side holds.

#### Scenario: Lineup
- **WHEN** a duel pair rematches with a roster of combatant, merchant, combatant
- **THEN** pool slots 1 and 3 stand, 4000 ms after the return
