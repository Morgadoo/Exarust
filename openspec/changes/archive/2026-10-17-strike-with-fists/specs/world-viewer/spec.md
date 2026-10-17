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
- **WHEN** `exarust --still-opponent --at 0,0,-1.1 --press LMB@120 --pause-on-hit 3 --screenshot p.png --frames 10 play arenao1` runs
- **THEN** `p.png` shows the player's right fist on the duellist three ticks after the punch landed

### Requirement: Still opponent
`--still-opponent` SHALL spawn the play mode duellist as `--opponent` does, standing its ground: it SHALL face the player and take its stance, but neither walk nor strike.

#### Scenario: Stances at a distance
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs
- **THEN** the duellist stays 8 m from the player, facing it in the Defence stance

#### Scenario: Unattended attacks
- **WHEN** `exarust --opponent --weapon ub05ssworda --press LMB@100-1400 play arena1` runs
- **THEN** the player, in combat mode from the fight start, swings its sword from frame 100, and again 426 ms after each swing's last tick while the button is held
