## ADDED Requirements

### Requirement: Unwatched draw
A counted team SHALL score (count times 0.5 plus 0.5) times its strength sum. An aside fighter SHALL be left out, and mode 16 SHALL leave out the player's team. A weight SHALL be the square of that score over the largest, times 100, rounded to the nearest even integer. The roll SHALL become roll times 0x08088405 plus 1. The high half of that word times one more than the weight total SHALL pick the first team whose weights reach it. No counted fighter SHALL step the roll and name no team.

#### Scenario: A pair outscores a single until the roll passes it
- **WHEN** teams of strength 4, then 2 and 2, then 10 are drawn from rolls 1, 4 and 12, that list is drawn in mode 16 for team 1 from roll 1, a strength of 100 set aside is drawn from roll 1, and an empty list is drawn from roll 1
- **THEN** the winners are teams 1, 2 and 3 at rolls 0x08088406, 0x20221015 and 0x6066303d, mode 16 names team 2 at roll 0x08088406, the aside draw names team 1 at that roll, and the empty list names nobody at that roll
