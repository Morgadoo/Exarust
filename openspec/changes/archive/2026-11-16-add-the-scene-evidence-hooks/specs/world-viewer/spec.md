## ADDED Requirements

### Requirement: Debug fire
With `--fire x,y,z[,size]` in play mode the level SHALL get a fire of that size (0.8 when none is given) standing at that point in metres from the load, built as a level fire marker's is; the flag SHALL be repeatable.

#### Scenario: A brazier in the arena
- **WHEN** `play arena --still-opponent --fire 0,0,2` runs
- **THEN** a 0.8 fire stands at the duellist's place and the log names it

### Requirement: Debug weapon seeds
With `--weapon-seeds a,b[,q]` a factory weapon given as `--weapon @name` SHALL be built from those two paint seeds (hex with `0x` or decimal) and quality q (200 when none is given), held with the plain sword's class word; a value that cannot be read SHALL exit 2.

#### Scenario: A gem-set sceptre
- **WHEN** the character view runs with `--weapon @sceptre_a --weapon-seeds 0x51,0x0`
- **THEN** the sceptre is built from those seeds and quality 200

### Requirement: Mechanism listing
With `EXA_MECHANISM_LIST` set, once the level's sliding leaves exist the run SHALL log each leaf's placed id, place, fraction and state bits and each lever's id, place and targets, and every 30 frames after it each awake leaf's fraction and place.

#### Scenario: exanima01's portcullises
- **WHEN** exanima01 loads with the variable set
- **THEN** 37 sliding leaves and every lever are listed

### Requirement: Repeated mechanism pokes
`--mechanism-poke` SHALL take a comma-separated list of frames; at each the nearest lever SHALL be turned to its other detent and held, so a second poke sends the opposite signal.

#### Scenario: A gate closed and opened
- **WHEN** `--mechanism-poke 150,500` runs beside a lever bound to a sliding gate
- **THEN** the gate gets the close order at the first poke and the open order at the second

### Requirement: Debug scan reconstruction
`--debug-scan` SHALL also set the story session's reconstruction word to rebuildable and its checkpoint identity to the player's, as a reconstruction device's scan does, so a later story death shows the reconstructed end menu.

#### Scenario: A scripted death after a scan
- **WHEN** a story session runs with `--debug-scan 120` and the player dies later
- **THEN** the end menu offers CONTINUE and EXIT, and CONTINUE rebuilds the player at exanima05
