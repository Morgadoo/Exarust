# Spec Delta

## ADDED Requirements

### Requirement: Bar cast entry in the game
The game SHALL hand the chosen slot's word, unchanged, to the bar's cast entry, which SHALL dispatch by the word's domain byte (Force, Energy, Displacement; a Mind word is refused) and create the started power's class after the front end answers a start.

#### Scenario: A power the tree added
- **WHEN** the record's second slot holds the Shock id although the player was granted only the barricade, and that slot is picked and used
- **THEN** the cast starts a Shock

#### Scenario: A barricade
- **WHEN** the barricade is cast with a pool of 1.0
- **THEN** the cast answers started, 0.25 of focus is reserved and spent, and one wall entity exists

### Requirement: Active power walk
Each body tick SHALL pass control event 0 to every active power, and event 2 to a power marked ending, and SHALL take a power that answers with bit 2 out with the last entry in its place. The body's release flag SHALL NOT end a power.

#### Scenario: A shell pressed again
- **WHEN** the shell runs and is cast again
- **THEN** the cast answers ended, the shell is marked ending and the next tick removes it, clears the cast id and returns its reserve

#### Scenario: The release flag
- **WHEN** the body's release flag is set while a shock runs
- **THEN** the shock keeps running

### Requirement: Vanish pressed again
Casting vanish while it runs SHALL answer released and tell the running vanish to let go.

#### Scenario: Second press
- **WHEN** vanish is cast twice
- **THEN** the second answer is released and the running vanish is marked released

### Requirement: One record for the tree and the caster
The caster SHALL take the screen's record and bar when the tree closes, and the screen's closed bar SHALL close the caster's bar.

#### Scenario: A tree session
- **WHEN** the tree opens, fills slot 3 and a learnt pair, and closes
- **THEN** the caster's record holds both and its known powers number three
