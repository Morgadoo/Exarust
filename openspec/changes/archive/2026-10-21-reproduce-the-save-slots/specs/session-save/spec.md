## MODIFIED Requirements

### Requirement: Refused file
A file that is not an Exarust session package SHALL be refused, including one that is empty, cut short, a retail package whose directory does not fit, or a package whose player data has no Exarust chunk.

#### Scenario: Not ours
- **WHEN** the bytes are empty, the old `EXRS` magic alone, the original save magic with a directory that does not fit, or the session package cut by one byte
- **THEN** the read is an error

## ADDED Requirements

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
