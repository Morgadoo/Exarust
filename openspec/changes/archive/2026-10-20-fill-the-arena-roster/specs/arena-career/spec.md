## ADDED Requirements

### Requirement: Arena roster
The system SHALL provide six 0x30-byte roster rows with two entrants, the 8-byte spawn profile, the unfit filter and the 2..6 round-id assignment.

#### Scenario: Profile and unfit
- **WHEN** tier 3 is spawned for company 15 unarmed
- **THEN** the profile is 0x0B, 3, 100, 10, 10, 4, 92, 0
- **WHEN** an entrant at 0x0CCB is filtered
- **THEN** it is removed and a 0x0CCC entrant is kept

### Requirement: Arena resolution
The system SHALL tally the rows by team id, award a player tally of 3+ without a tie, record company wins as the shuffled id+2, and grant the season set at `(day % 28) % 4` on week 3 with all four flags.

#### Scenario: Champion and company
- **WHEN** the rows are [1,1,1,2,3,2] on day 27
- **THEN** the player wins, the prize and last season set are granted, and championships and seasons rise
- **WHEN** the rows are [2,2,2,1,3,4]
- **THEN** no champion is recorded and week one holds company 15+2
- **WHEN** the rows tie at two each
- **THEN** no champion is recorded
