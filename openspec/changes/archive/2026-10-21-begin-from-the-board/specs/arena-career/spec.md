## ADDED Requirements

### Requirement: Next tournament event
The board's Begin on the tournament page SHALL start the first of the week's six events with no winner and an entrant, and nothing when none is left. While the manager warning is armed and the manager is a participant, Begin SHALL disarm it and show the warning once.

#### Scenario: Events 2 and 4 entered
- **WHEN** no event is entered, then events 2 and 4 are, then event 2 is won, then event 4, and the warning is asked without and twice with the manager
- **THEN** nothing starts, then event 2, then event 4, then nothing; the warning shows once

### Requirement: Day gate
The manage window SHALL let the day end on a tournament day with entries only when no entered event is still pending, and on any other day only once a match was fought.

#### Scenario: A tournament day
- **WHEN** day 3 has no match and one match, day 7 has two pending events and three matches, day 7 has both events won, and day 8 has no match
- **THEN** day 3 ends only after its match, day 7 waits for its events whatever the matches and then ends, and day 8 does not end

### Requirement: Health bar
The company tables SHALL fill a fighter's health bar with its health word times the double 1/65535, stored as single.

#### Scenario: Full and half
- **WHEN** the words 65535 and 32768 are drawn
- **THEN** the bars are bits 0x3f800000 and 0x3f000080
