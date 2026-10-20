## ADDED Requirements

### Requirement: Arena rank roll
The system SHALL roll a template's rank to −1 when unranked, to the slot when flag 2 is set, and to `trunc(slot·(1−r⁴))` otherwise; Challenger SHALL open a 0–5 window and others the slot to the rolled rank.

#### Scenario: Ranked and unranked
- **WHEN** Elimination is rolled at slot 3 with r 0.5
- **THEN** its rank is 2 and its window is 3–2
- **WHEN** Brawl is rolled at slot 4
- **THEN** its rank is −1
- **WHEN** Challenger is rolled at slot 0
- **THEN** its window is 0–5

### Requirement: Arena generation
The system SHALL accept a template only when its per-pass cap is not reached, the slot is at least its minimum and the 0–7 roll misses the reject mask; draws SHALL use the engine generator and retry.

#### Scenario: Filters
- **WHEN** Valiance is generated at slot 0
- **THEN** it is rejected
- **WHEN** its cap of 2 is already used
- **THEN** it is rejected
- **WHEN** Brawl rolls 1 against its mask 3
- **THEN** it is rejected

### Requirement: Arena settlement
The system SHALL pay only on a win, log 0x20000/0x40000/0x30000 for a win/practice/other, count a Beast win and always count the match played; the season armour set SHALL need the last week with all four wins.

#### Scenario: Win and season
- **WHEN** a Beast match is won
- **THEN** it pays, logs 0x20000 and counts the beast
- **WHEN** week 3 has all four weekly wins
- **THEN** the last season set is awarded
