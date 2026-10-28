## ADDED Requirements

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
