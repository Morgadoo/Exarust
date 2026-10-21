## ADDED Requirements

### Requirement: Staff ids
The manager SHALL be fighter 1 with id word 0x10000000. Each employee SHALL have the id word of its place plus one, with 0x10000000 added for a recruited fighter. A combatant or a recruit SHALL count as a fighter.

#### Scenario: Ids and fighters
- **WHEN** one combatant is hired, and eight fighters are recruited
- **THEN** there are 2 fighters and the combatant's id word is 1; the eight recruits make 9 fighters and the third's id word is 0x10000003

### Requirement: Hire
A hire SHALL be refused with "Insufficient funds" when the purse holds less than the price, with "You do not have room for any more fighters." when a combatant finds eight fighters, and with "You already have a merchant in your employ." (or a physician, or a trainer) when one is already employed. Otherwise the price SHALL leave the purse and join today's spending, the candidate SHALL leave the hire list, one more hire SHALL be counted and the joining SHALL be logged with the name.

#### Scenario: A combatant for 20
- **WHEN** a purse of 20 hires a combatant named Ash for 21, then for 20, seven combatants fill the company, an eighth is asked for, and two trainers are asked for
- **THEN** 21 is refused for funds, 20 leaves 0 with 20 spent, one hire and a joining logged with 0x00687341; the eighth combatant finds no room, the first trainer is hired and the second is refused

### Requirement: Hire kit
A hired combatant's kit SHALL be every set hand item and every worn item but item 8 of its six loadouts, each once, in order, and SHALL go to the arsenal with flags 0x400. A specialist's own items SHALL go with flags 0x200.

#### Scenario: Two loadouts
- **WHEN** a kit is built from loadouts with hands 5 and none and items 8, 9, 10, then hands 6 and 5 and items 10, 11, and the combatant Ash with hand 5 and items 8, 9 is hired
- **THEN** the kit is 5, 9, 10, 6, 11; Ash's kit is 5, 9 and the weapons shelf holds 5 and 9 with 0x400

### Requirement: Recruitment
A recruitment SHALL cost 12 copper, refused with "Insufficient funds" below that, and SHALL NOT check the room. The recruit's six loadouts SHALL copy its outfit and its kit SHALL go to the arsenal with 0x400; one more hire SHALL be counted and the joining logged.

#### Scenario: Nine fighters
- **WHEN** 11 copper recruits, then 108 copper recruits eight times with an outfit of hands 3 and 4 and items 8 and 7
- **THEN** 11 is refused, the eight recruits make nine fighters and leave 12, and a recruit's kit is 3, 4, 7

### Requirement: Leaving
An employee who leaves SHALL release a fighter's kit from flags 0x400, or a specialist's items, hands and worn items without flag 0x100 from 0x200. A merchant SHALL empty his stock and his figure SHALL leave the hub, as SHALL a physician's figure. The id SHALL leave the tournament entries and the log's saves, and later ids SHALL move down by one. A dismissal SHALL log the name and count one more fired.

#### Scenario: The second of three leaves
- **WHEN** of three combatants the second leaves while an entry holds ids 2 and 3 and the log saves 2, 3 and 1, and a merchant is dismissed
- **THEN** the entry holds id 2 alone, the saves are 2 and 1, the first two kit entries are released; the merchant empties his stock, one is fired and the dismissal is logged

### Requirement: Order list
The manage table's order list SHALL grow by the employee count on a hire or a recruitment only when it already holds an entry. A leaving SHALL make it one shorter: each entry but the last equal to the leaver's number SHALL close up, the scan moving past the entry that took its place, and later numbers SHALL stay.

#### Scenario: Removals
- **WHEN** 2 leaves 1, 2, 3, 4; 9 leaves 1, 2, 3; 1 leaves 5; 2 leaves 2, 2, 3
- **THEN** the lists are 1, 3, 4; 1, 2; empty; and 2, 3

### Requirement: Report title
The report's title SHALL be "Day" and the day from 1 before day 7, else "Week", the week, "Day" and the day of the week from 1.

#### Scenario: Titles
- **WHEN** days 0, 6, 7 and 20 are titled
- **THEN** they read Day 1, Day 7, Week 1 Day 1 and Week 2 Day 7

### Requirement: Report log lines
A won, lost or forfeited match SHALL read "You have won", "You have lost" or "You forfeited", the match in lower case after "a " or, before a vowel, "an ", and "match."; the Beast SHALL read "beast". A death, save, dismissal, joining and leaving SHALL read the name and " has died.", " was saved from lethal injury.", " has been dismissed.", " has joined the company." and " has left the company.".

#### Scenario: Lines
- **WHEN** the log holds a won Elimination, Ash joining, the manager Boss saved and a leaving
- **THEN** the lines read "You have won an elimination match.", "Ash has joined the company.", "Boss was saved from lethal injury." and the leaver's name with " has left the company."

### Requirement: Report fighters and hire list
After the log, newest first, each fighter and then the manager SHALL be listed when ready to train (no skill below level 5 with its next technique set, and a level sum under 15) as " has trained a new technique.", with " has reached " and the rank of half the sum and " rank." when the sum is even and under 11. The hire list's merchants, trainers and physicians SHALL follow, then "The tournament takes place today.", "begins tomorrow." or "begins in N days.".

#### Scenario: Day 3
- **WHEN** on day 3 the log holds a lost duel and a forfeited doubles, Ash has levels 2 and 2, the manager level 1, and the list holds a merchant and two trainers
- **THEN** the report reads the forfeit, the loss, Ash trained and Novice rank, the manager trained, "A merchant is available for hire.", "2 trainers are available for hire." and "The tournament begins in 4 days."
