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
A file that is not an Exarust session package SHALL be refused, including one that is empty, cut short, a retail package whose directory does not fit, or a package whose player data has no Exarust chunk.

#### Scenario: Not ours
- **WHEN** the bytes are empty, the old `EXRS` magic alone, the original save magic with a directory that does not fit, or the session package cut by one byte
- **THEN** the read is an error

### Requirement: Season round trip
A save SHALL restore the day, which weeks were won, the seasons and tournaments won, the reward count, and the reward item ids. A career saved on day 8 with the first two weeks won SHALL still grant season 0 when the remaining weeks are swept.

#### Scenario: Two weeks, then the season
- **WHEN** a company has swept two weeks, is saved on day 8, loaded, and then sweeps through day 22, and that result is saved and loaded again
- **THEN** the first load is on day 8 with the first two weeks won, two tournaments won, and no reward ids, and the second load is on day 22 with one season won, four rewards, and the ids 1175, 1174, 1177 and 1176

### Requirement: Package shape
A session SHALL be written as a save package whose `#playerdata` carries the mode's header (story: the level as locale, seed, start background with the started bit, clock; arena: `arenahub`, the company's copper, data version 9, clock, day), the journal chunk and an Exarust chunk with the rest of the session.

#### Scenario: Arena and story
- **WHEN** a career on day 5 with 45 copper, clock 90 and one journal page is written
- **THEN** the arena header reads locale arenahub, money 45, version 9, clock 90, day 5, and the journal holds the page
- **WHEN** a story two levels in with seed 0x1234, background 2 and clock 10 is written
- **THEN** the story header reads exanima03, seed 0x1234, flags 0x10002, clock 10, and the session reads back at exanima03

### Requirement: Saving over a retail save
Saving a session over a save of the same mode SHALL keep the save's other sections, its character placement and the player-data chunks Exarust does not produce, so the original's reader still finds the placement first.

#### Scenario: Retail-shaped story save
- **WHEN** a session is saved over a story save holding a character database, a placement with reference 0x40000001, discovery and experience chunks, and a level
- **THEN** the character database and level sections are unchanged, the reference is still 0x40000001, the chunks read discovery, experience, journal and the Exarust chunk, and the session loads back

### Requirement: Arena tables read back exactly
Loading an arena save SHALL fill the session's arena tables only when the save carries a table chunk and writing the decoded tables reproduces every table chunk byte for byte; otherwise the session SHALL carry no tables, so a later save keeps the previous chunks unchanged.

#### Scenario: Exarust's own tables
- **WHEN** an arena session captures a company with stock 11 and 12, a fighter with world character 0x55 and a stash item 31, and is written and read back
- **THEN** the read session holds tables with that stock, that employee and that stash, and applying them restores the same arsenal and staff

#### Scenario: A table the model cannot rebuild
- **WHEN** the shop stock chunk carries four bytes after its list
- **THEN** the exact read gives no tables

### Requirement: World clock across a save
A load SHALL restore the world clock from the header's saved seconds at the load's tick, and a save SHALL stamp the clock's current whole seconds into the header.

#### Scenario: Restore and stamp
- **WHEN** a save with clock 8684 is restored at tick 1000 ms and stamped at 61000 ms
- **THEN** the header carries 8744, and that save restored at 0 ms reads 8747 at 3000 ms

### Requirement: Scene preset at a level load
A level load SHALL apply the indoor preset to `arena1`–`arena7` and the story levels and the clock-seeded outdoor preset to `arenao1`–`arenao5`, allowing a night only on `arenao1`–`arenao4`; other levels SHALL keep their record's lighting.

#### Scenario: Indoor levels keep their look
- **WHEN** each shipped indoor level is lit with and without the indoor preset
- **THEN** both lightings are equal

#### Scenario: Night only where the table allows it
- **WHEN** the outdoor preset is drawn with seed 1 with and without the night flag
- **THEN** the time of day is 0.9666516 with the flag and 0.32082716 without
