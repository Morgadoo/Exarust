## MODIFIED Requirements

### Requirement: Live effects
The game SHALL run live arcs, barricades and wisps after the bodies, an arc's shock reaching a fighter as the live shock timing states. A barricade's push SHALL move a part's particles by their weights of the struck sphere's centre and restore its edges past the push strain; each part of a character it strikes in front SHALL lower the character's energy while it is above 0.25. `--debug-arcs`, `--debug-barricade` and `--debug-wisps` SHALL put the effects on the player.

#### Scenario: Two shocks within the gap
- **WHEN** a fighter never shocked takes a shock of 0.05 at 0 ms with the generator's state at 0, and then at 348 ms and at 360 ms
- **THEN** its yellow health falls from 0.25 to 0.20, stays at 348 ms (its next shock is due at 500 ms), then falls to 0.15, its next shock due at 866 ms

#### Scenario: Three parts in front
- **WHEN** three parts of a character with energy 0.26 are struck in front, each losing 0.004
- **THEN** its energy is 0.248

## ADDED Requirements

### Requirement: Live shock timing
A shock SHALL reach a fighter once the game clock is within 150 ms of its next shock time, or when it was never shocked. Its next shock time SHALL then be the clock plus 500 ms plus the engine generator's draw below 200 ms, and a fighter that is not collapsing or dead SHALL lose the strength in yellow health, floored at 0.

#### Scenario: Two shocks within the gap
- **WHEN** a fighter never shocked takes a shock of 0.05 at 0 ms with the generator's state at 0, and then at 348 ms and at 360 ms
- **THEN** its yellow health falls from 0.25 to 0.20, stays at 348 ms (its next shock is due at 500 ms), then falls to 0.15, its next shock due at 866 ms

#### Scenario: A collapsing fighter
- **WHEN** a collapsing fighter with 0.1 yellow health takes a shock of 0.05
- **THEN** its yellow health stays 0.1 and its next shock time still moves
