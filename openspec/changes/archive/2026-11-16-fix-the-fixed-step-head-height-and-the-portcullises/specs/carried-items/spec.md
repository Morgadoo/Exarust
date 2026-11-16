## ADDED Requirements

### Requirement: Head height first in the fixed step
The armour pass's head height SHALL move a held weapon's arm link first in the fixed step, before the tick deals its blows and before the grip builder, so every stage of the tick reads the same arm link whatever other systems the schedule holds.

#### Scenario: The game's fixed step
- **WHEN** the game's App is built
- **THEN** its fixed step orders the head height before the blows, the character ticks, the grip builder and the solver
