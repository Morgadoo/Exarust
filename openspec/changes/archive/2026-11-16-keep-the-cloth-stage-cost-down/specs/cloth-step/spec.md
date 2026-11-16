## ADDED Requirements

### Requirement: Cloth part index keeps unmoved links
The cloth stage's part index SHALL leave a changed body's links as they are when every part it links stays in its cell with the same place and parts, since linking it again would give every cell the same parts; a body that leaves a cell SHALL be linked afresh.

#### Scenario: One centimetre on
- **WHEN** an indexed body is marked changed after moving one centimetre inside its cells
- **THEN** every cell's list is as it was, and the index holds what a fresh index of every body holds

### Requirement: Cloth cost bench on the game's path
The cloth stage's cost benches SHALL hand the solver's object-contact hook the bodies split and ordered as the body stage hands them to the solver, without moving them, so they time the stage as the game runs it, and SHALL print the stage's time by piece.

#### Scenario: Fifteen hairs among sleepers
- **WHEN** the level bench runs fifteen hairs among 4 800 sleeping bodies for 400 ticks
- **THEN** the hair checksum is 5a45a5c364b1c3b3, as without the solver's lists
