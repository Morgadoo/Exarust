## ADDED Requirements

### Requirement: Hub music
Entering the hub SHALL step the roll and play Msc_Hub01 when the high half of the word times 3 is under 2, else Msc_Hub02 (the DnB tracks with the alternative music option), at 0.1. It SHALL step again and start the track at none, 41, 126.8 or 152 seconds by the high half of the word times 4. In play mode the hub SHALL draw its music from the counter roll.

#### Scenario: Two hub entries
- **WHEN** the hub is entered from rolls 0, 0xf0000000 (with the option), 4, 2 and 1
- **THEN** roll 0 plays Msc_Hub01 from its start and ends at 0x08088406; 0xf0000000 plays Msc_Hub02_DnB from 41 seconds and ends at 0x78088406; 4, 2 and 1 start at 41, 126.8 and 152 seconds

### Requirement: Hub click
A hub click SHALL open nothing while the hub is busy, outside the arena mode or while a confirmation is open. An object named ArenaShop (its sixteen bytes compared with a to z matching capitals) SHALL open the shop; an ArenaMach object SHALL open the match board when its side value is above 0 and the manage window otherwise; an employed merchant who is not helpless SHALL open his stock.

#### Scenario: Clicks
- **WHEN** the shop, a lower-case shop name, the machine at sides 1, 0 and NaN, a name without the first byte, a merchant standing and down, and a stranger are clicked, and the shop is clicked while busy, in mode 4 and while confirming
- **THEN** both shop names open the shop, side 1 opens the board, 0 and NaN the manage window, the bare name nothing, the standing merchant his stock, the downed merchant and the stranger nothing, and the gated clicks nothing

### Requirement: Hub fall
While the hub's session runs, a manager whose condition is dead SHALL end the session; health under 0.01 compared in double SHALL first wipe the career's saves.

#### Scenario: Falls in the hub
- **WHEN** a dead manager has health 0, 0.01, 0.02 and NaN, a collapsing manager has 0, and the session is not running
- **THEN** 0 and 0.01 wipe, 0.02 and NaN only end the session, and the others do nothing
