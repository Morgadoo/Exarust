## ADDED Requirements

### Requirement: Exit save records the level left
A level exit's save SHALL be written before the destination loads: it SHALL record the level being left as its locale and level section, with the player's position in that level.

#### Scenario: Leaving exanima03
- **WHEN** the player takes `exanima03`'s exit to `exanima04` with a current slot
- **THEN** the slot's save loads `exanima03`

### Requirement: Level-change arrival
A level change SHALL stand the arriving player on the last transition marker of the destination whose entrance word is the change's entrance and whose previous-level word is the `locales.rdb` id of the level left, at the marker's position and heading, probed onto the floor; the simulation's player SHALL move there too.

#### Scenario: From exanima01 into exanima02
- **WHEN** the player takes `exanima01`'s exit (entrance 0)
- **THEN** it stands in `exanima02` at the marker that names entrance 0 and level 2, by the exit back

### Requirement: Arrival exceptions
A story load SHALL name no level left, so no marker applies to a continue; the flow's own placement, a save's landing plan and an arena's layout SHALL take precedence over the arrival, and with no matching marker the level's own start SHALL stay. `--at` SHALL place only the first level.

#### Scenario: A continue after an exit save
- **WHEN** a save written at `exanima01`'s exit is continued
- **THEN** the player stands in `exanima01` at the saved position, nudged out of the exit
