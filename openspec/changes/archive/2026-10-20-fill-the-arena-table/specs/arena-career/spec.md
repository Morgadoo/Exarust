## ADDED Requirements

### Requirement: Arena match templates
The system SHALL provide the six standard, seven ranked and five tournament templates with the original's names, ids, flags, teams, fighters, rounds, cash, XP, slot limits and descriptions.

#### Scenario: Three templates
- **WHEN** template 3 is read
- **THEN** it is Skirmish with 3 teams, 1536 cash and 3900 XP
- **WHEN** template 11 is read
- **THEN** it is The Beast with 196608 cash, 10000 XP, minimum slot 2 and 1 per pass
- **WHEN** the name of id 11 is looked up
- **THEN** it is the lower-case "beast"

### Requirement: Arena money and prizes
The system SHALL pack cash as three base-12 bytes, pay `unpack(cash)·(slot+1)`, and compute the XP pot as `trunc(xp·(1+0.25·slot))`, scaled by the survivors' health, doubled by half for a manager and split by damage.

#### Scenario: A duel
- **WHEN** Duel is fought at slot 3
- **THEN** its pot is 1750
- **WHEN** half the health remains with a manager
- **THEN** the award is 1312
- **WHEN** two fighters dealt equal damage
- **THEN** they split the total equally

### Requirement: Arena ranks and wages
The system SHALL compute rank points as `1+Σxp/5000`, the loadout tier as `min((rank−1)/2, 5)`, one award as `min(total/rank, 5000)`, and wages as rank−1, 2, 12 or tier+1 by class mask.

#### Scenario: Ranks and wages
- **WHEN** the pools hold 5000, 4999 and 10000
- **THEN** the rank points are 4
- **WHEN** the class mask is 0x50000000
- **THEN** the wage is 12

### Requirement: Arena condition and calendar
The system SHALL recover 0x1999 or 0x2666 condition (saturating), treat below 0xCCC as unfit, start a week on day 1 mod 7, end it on day 0 and divide the season into four weeks.

#### Scenario: Recovery and phases
- **WHEN** condition 1000 recovers with a physician
- **THEN** it is 10830
- **WHEN** the day is 27
- **THEN** it is week 3
