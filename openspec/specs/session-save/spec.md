# session-save Specification

## Purpose
An Exarust save that restores the session and refuses the original's packages.

## Requirements

### Requirement: Round trip
A save SHALL restore the mode, the story level, the journal text, the chosen topics, the company's copper, roster and experience, and the focus.

#### Scenario: Career session
- **WHEN** a career that has advanced a level, written a journal line, chosen a topic, won a duel and set its focus is written and read back
- **THEN** those fields match and the topic cannot be chosen again

### Requirement: Refused file
A file that is not this save SHALL be refused, including one that is empty, cut short, or starts with the original save magic.

#### Scenario: Not ours
- **WHEN** the bytes are empty, cut short, or start with the original save magic
- **THEN** the read is an error

### Requirement: Season round trip
A save SHALL restore the day, which weeks were won, the seasons and tournaments won, the reward count, and the reward item ids. A career saved on day 8 with the first two weeks won SHALL still grant season 0 when the remaining weeks are swept.

#### Scenario: Two weeks, then the season
- **WHEN** a company has swept two weeks, is saved on day 8, loaded, and then sweeps through day 22, and that result is saved and loaded again
- **THEN** the first load is on day 8 with the first two weeks won, two tournaments won, and no reward ids, and the second load is on day 22 with one season won, four rewards, and the ids 1175, 1174, 1177 and 1176
