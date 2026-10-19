## ADDED Requirements

### Requirement: Losing bracket
A slot whose id is above 0 SHALL add its strength to the team. The team SHALL score (count times 0.5 plus 0.5) times that sum. The first group SHALL keep a forced id above 0 without stepping the roll. Any other resolution SHALL step the roll by times 0x08088405 plus 1 and keep the first team whose rounded weights pass the high half of the weight total times that word. Every other counted id SHALL become 0. More than one group SHALL say so.

#### Scenario: A forced team stays and the next group is drawn
- **WHEN** width 1 and two teams, ids 5 and 9 then 3 and 7, are cleared with force 5 from roll 1, one group of ids 1 and 9 is cleared from rolls 1 and 100, a width of 2 with a zero id of strength 100 beside id 1 against two ids of 3 is cleared from roll 4, and an empty list is cleared from roll 7
- **THEN** the two groups leave ids 5, 0, 3 and 0, step the roll to 0x08088406, and report another group, roll 1 leaves id 1, roll 100 leaves id 9 at roll 0x235391f5, the zero id stays 0 while id 1 is cleared and both ids of 3 stay at roll 0x20221015, and the empty list reports no other group with roll 7
