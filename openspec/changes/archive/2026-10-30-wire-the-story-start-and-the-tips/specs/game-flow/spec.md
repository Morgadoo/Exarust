## ADDED Requirements

### Requirement: Steps after a level load
The running game SHALL apply a step batch up to its level load (and any player placement, which crosses the load) at once and SHALL hold the rest until the load's loading has shown and ended for two frames, a player stands in the level and the placement has been applied; a load whose loading never shows SHALL release its held steps after 600 frames. While steps are held the session hook SHALL read no player.

#### Scenario: The story start's knock-out
- **WHEN** a new story starts and its level loads, then the player is spawned and placed
- **THEN** the start record, the controls, the torch, the motion and the 3000 ms knock-out are applied to the spawned player, not before

### Requirement: Story start on the spawned player
Before the load, the story start SHALL write the off-hand item and the carried entry into the record the load spawns the player from; after the load the spawned player SHALL take the spawn request's word (0x200 for the unknown background, 0 for the knight and proctor, 0x600 for the villager), the role list on a cleared list, the off-hand item, the carried entry at its saved cell and the start motion.

#### Scenario: The unknown background's letter
- **WHEN** an unknown background's story starts and the player is spawned
- **THEN** the record and the player's pack carry item 0x10c at the cell (0x20, 0x80), its roles hold 7, 1 and 0x0e, and its condition is 0x200

### Requirement: Story start torch
The story start's light SHALL be the game's torch item 0x1f laid in the world at the start's pose: (8045, 4.7, 4075) cm turned (−48°, 110°, 88°) for the unknown background and (8000, 500, 4000) cm turned (8°, −150°, 50°) for the villager, the frame the engine builds from the three angles. A player that carries Exarust's torch SHALL keep it and no second torch SHALL be laid.

#### Scenario: Without the carried torch
- **WHEN** an unknown background's story starts with `--no-torch`
- **THEN** a burning torch is laid at (80.45, 0.047, 40.75) m with its handle axis within 6° of the floor

### Requirement: Tip banner
`Step::Tip(n)` SHALL show the prelude's entry on the tip banner: 0 a rotating tip (only with ShowGameTips), `n` from 1 to 6 the hint `n − 1` (2 "You have been defeated.", 3 "You have died."); the prelude's close and a level load SHALL hide it. A new story SHALL show no tip.

#### Scenario: Knocked out
- **WHEN** the story player is knocked out
- **THEN** the banner shows "You have been defeated." until the prelude closes or a level loads

### Requirement: Manual window
The main menu's MANUAL rows SHALL open the manual over the covered menu, from `manualkm.fds` (or `manualcn.fds` in controller mode) in the resource pack: a centred "MANUAL" form of 0x2fc × 0x1e6 at the 4-pixel unit, its topic list 0x90 wide from 0x14 down, and the first listed topic's record, without its last byte, in the page box. A pack that does not parse SHALL open nothing.

#### Scenario: F1 then MANUAL
- **WHEN** F1 opens the in-game menu and its MANUAL row is chosen
- **THEN** the manual shows the topic list with its first topic's page

### Requirement: Manual input
While the manual is open, a list click or the arrow keys SHALL pick a topic, and Escape, F1 or a right click SHALL close it and uncover the menu; the menu SHALL take no input until then.

#### Scenario: Closing with F1
- **WHEN** F1 is pressed with the manual open over the menu
- **THEN** the manual closes and the menu stays open
