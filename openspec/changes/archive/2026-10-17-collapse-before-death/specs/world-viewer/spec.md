## ADDED Requirements

### Requirement: Pause after a collapse
`--pause-after-collapse <n>` SHALL pause the simulation n ticks (at least 1) after the tick in which a killing blow first starts a body's collapse, logging the body and the tick; `--screenshot` SHALL count its frames from the pause.

#### Scenario: One second into the player's collapse
- **WHEN** `exarust --opponent --health 0.05,0.05 --pause-after-collapse 83 --screenshot c.png --frames 10 play arena` runs until the duellist's blow kills the player
- **THEN** `c.png` shows the player 83 ticks (1 s) into its collapse, still on its feet in Fall Backward's lead-in
