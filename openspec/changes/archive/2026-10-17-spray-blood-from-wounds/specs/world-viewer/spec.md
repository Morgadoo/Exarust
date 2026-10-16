## ADDED Requirements

### Requirement: Debug blood jets
`--debug-spray bone,R[,dx,dy,dz]`, repeatable, SHALL fire a blood jet from the player's body or the character view's body every second of game time once it is dressed and bound: at its body part's front (as a debug wound's), as a bleeding blow of size R (over 0) whose striking point swung along d (body frame, +Y up, +Z forward; 0,0,−1 by default) would. A bone SHALL be given as for `--debug-wound`; a bad value SHALL print the expected form and exit with code 2.

#### Scenario: Chest jet
- **WHEN** `exarust --debug-spray chest,1 character` runs
- **THEN** every second 36 droplets spray out of the chest's front and fall

#### Scenario: Bad value
- **WHEN** `--debug-spray chest` is given without a size
- **THEN** exarust prints the expected form and exits with code 2

## MODIFIED Requirements

### Requirement: Scripted pauses
`--pause-at-tick <n>` SHALL pause the simulation at the end of body tick n, `--pause-on-stretch <cm>` at the end of the tick in which a body's link first goes that far past its slack, logging the body, part and link, and `--pause-after-jet <n>` at the end of the nth tick after the tick that first finds a blood jet started. With any of them, `--screenshot` SHALL count its frames from the pause.

#### Scenario: Freeze a fold
- **WHEN** `exarust --debug-bodies --opponent --pause-on-stretch 1 --screenshot s.png --frames 30 play arena` runs
- **THEN** `s.png` shows the duel stopped at the tick a link first opened 1 cm past its slack, with that part's row red

#### Scenario: Freeze a jet
- **WHEN** `exarust --opponent --weapon ub05ssworda --click 100,125,150,175,200 --pause-after-jet 12 --screenshot j.png --frames 20 play arena1` runs until a blow bleeds
- **THEN** `j.png` shows the duel stopped 12 ticks after that blow, with the jet's droplets flying off the struck part
