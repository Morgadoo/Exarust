## ADDED Requirements

### Requirement: Lifetime coin records
Ending a day SHALL add the day's earned and spent copper to the lifetime earned and spent records, then set the day's spent to the night's wages and its earned to 0. The records window SHALL show the lifetime totals.

#### Scenario: A night with wages
- **WHEN** a company with one unskilled combatant, 900 copper earned and 400 spent in its lifetime, ends a day that earned 30 and spent 40
- **THEN** the lifetime totals are 930 and 440, the day's earned is 0 and its spent is the wage of 1

### Requirement: Live record booking
The match return SHALL book its wins, losses, forfeits and Beast wins and the company deaths its falls counted onto the career's records and leave the others. A hire SHALL count one more hire and log the joining name; a confirmed dismissal SHALL count one more fired, log the leaving name and drop the id from the entries and the log's saved records. A week the company entered and did not take SHALL count one more tournament lost.

#### Scenario: A Beast win with a death
- **WHEN** a career with 3 deaths, 4 hires and 2 fired wins the Beast for 432 copper with one company fall
- **THEN** it has one win, one beast defeated, 4 deaths, 4 hires, 2 fired and 432 earned today

#### Scenario: A lost week
- **WHEN** day 7 ends with event 0 entered and company 2 the winner of all six events, and again with nothing entered
- **THEN** the first counts one tournament lost and clears the entries, and the second counts none

### Requirement: Live hire list
Each candidate SHALL take its role from the counter-seeded LCG draw, its trainer or combatant level from the career's run-time library generator, and its record from the random-NPC pass continuing the LCG; a trainer SHALL then be snapped. Its six levels SHALL be its skill words over 5000, which the hire fee and the upkeep read. A hire SHALL pass the funds, room and duplicate checks, take the fee and remove the candidate.

#### Scenario: Default list and a hire
- **WHEN** a default career lists its candidates and hires the first with 1000 copper
- **THEN** ten are listed, each level is its skill word over 5000, the roster gains the candidate, the purse loses its fee, one hire is counted and the list keeps nine

### Requirement: Week board and reset
A new week's board SHALL clear the event entries and make its sixteen companies the career's rival field. The arena reset SHALL clear the records and both day tallies, keep the hire list and the week's events, and roll the day's matches; a new career SHALL then add 343 copper, roll the tournament and fill the hire list.

#### Scenario: New week
- **WHEN** a career with event 2 entered rolls a new week
- **THEN** no event is entered and the rival field equals the board's companies, each from 1 to 19

### Requirement: Arena coin words
The company chunk's lifetime earned and spent records and the day chunk's spent and earned words SHALL be coin words: gold the copper over 144, silver the rest over 12, copper the remainder. The load SHALL read them back as copper. The tournament chunk's prize history SHALL be the career's.

#### Scenario: Coin words
- **WHEN** lifetime totals of 550 and 400 and day tallies of 40 spent and 55 earned are written and loaded
- **THEN** the words are 0x3090a, 0x20904, 0x304 and 0x407, and the load gives 550, 400, 55 and 40 back
