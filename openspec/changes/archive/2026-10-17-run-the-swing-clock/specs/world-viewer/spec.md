## ADDED Requirements

### Requirement: Scripted perks
In play mode `--perks a,b,...` SHALL give the player those perks by name, case aside (e.g. `remise,feint`); an unknown name SHALL stop the launch with an error. Placed characters SHALL keep their records' perks.

#### Scenario: Remise
- **WHEN** `exarust --opponent --perks remise --press LMB@200-600 play arenao1` runs
- **THEN** the log names the player's perks and each held main-side attack chains the other side's

#### Scenario: Unknown perk
- **WHEN** `exarust --perks remise,parry play arena` runs
- **THEN** it stops with an error naming the list

## MODIFIED Requirements

### Requirement: Scripted pauses
`--pause-at-tick <n>` SHALL pause the simulation at the end of body tick n, `--pause-on-stretch <cm>` at the end of the tick in which a body's link first goes that far past its slack, logging the body, part and link, `--pause-after-jet <n>` at the end of the nth tick after the tick that first finds a blood jet started, and `--pause-on-hit <n>` n ticks after the tick in which the player's first blow lands, logging it. With any of them, `--screenshot` SHALL count its frames from the pause.

#### Scenario: Freeze a fold
- **WHEN** `exarust --debug-bodies --opponent --pause-on-stretch 1 --screenshot s.png --frames 30 play arena` runs
- **THEN** `s.png` shows the duel stopped at the tick a link first opened 1 cm past its slack, with that part's row red

#### Scenario: Freeze a jet
- **WHEN** `exarust --opponent --weapon ub05ssworda --click 100,125,150,175,200 --pause-after-jet 12 --screenshot j.png --frames 20 play arena1` runs until a blow bleeds
- **THEN** `j.png` shows the duel stopped 12 ticks after that blow, with the jet's droplets flying off the struck part

#### Scenario: Freeze a punch
- **WHEN** `exarust --still-opponent --at 0,0,-1.1 --press LMB@120-200 --pause-on-hit 3 --screenshot p.png --frames 10 play arenao1` runs (the button held)
- **THEN** `p.png` shows the player's right fist on the duellist three ticks after the punch landed
