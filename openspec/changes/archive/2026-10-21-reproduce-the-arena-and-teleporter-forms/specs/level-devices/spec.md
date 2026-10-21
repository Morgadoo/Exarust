## ADDED Requirements

### Requirement: Teleporter journey start
A click on a destination SHALL close the screen and start a journey: a bubble 98 cm above the teleporter (held for the large-crystal adapter) and the teleporter glowing 1.5 s up and 7 s held for the adapter, or 1 s up and 1.4 s held for another crystal.

#### Scenario: The adapter's glow
- **WHEN** a journey paid by the adapter starts
- **THEN** the teleporter glows over a 1 500 ms rise and a 7 000 ms hold

### Requirement: Teleporter journey charge
A journey paid by a Power Crystal A SHALL spend it a point per 12 ms from its charge at the start, and every journey SHALL leave its crystal fully spent when the bubble is freed.

#### Scenario: A small crystal
- **WHEN** a journey paid by a Power Crystal A of charge 3 has run 120 ms
- **THEN** the crystal's charge is 13

### Requirement: Teleporter journey end
The level SHALL change to the row's level when the resting bubble is told to shrink; when an adapter's held bubble is freed the teleporter SHALL show one row from then on and the game SHALL save while the player's red health is above 0.01.

#### Scenario: The adapter
- **WHEN** the adapter's held bubble is freed with the player's red health at 0.5
- **THEN** the teleporter shows one row from then on and the game saves
