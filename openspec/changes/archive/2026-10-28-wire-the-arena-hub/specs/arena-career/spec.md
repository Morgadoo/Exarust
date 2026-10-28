## ADDED Requirements

### Requirement: Running hub session
When the arena hub's level is loaded in arena mode, or by the play mode that loads it, the game SHALL run the hub entry once, play its track and install its hooks, and each frame a dead manager SHALL end the session. Outside those modes the hub's systems SHALL do nothing.

#### Scenario: Entry and fall
- **WHEN** the hub level loads in arena mode with roll 0 and the alternative music, a second frame runs, and the manager is dead with health 0
- **THEN** the entry plays msc_hub01_dnb at 0.1 once and the fall ends the session with a wipe

#### Scenario: Other modes
- **WHEN** the plugin runs three frames in story mode with the hub level present
- **THEN** the hub is not entered and nothing panics

### Requirement: Running hub clicks
A click that is not a drag SHALL reach the hub's click hook with the hub's busy flag, the mode and whether a hub window is open, and SHALL open the answered window. The hook SHALL compare the node's stored sixteen name bytes, including the library's leading 0xAA.

#### Scenario: Clicks in the hub
- **WHEN** an ArenaShop node, an ArenaMach node at side 1 and at side -1, a standing and a downed merchant are clicked, and the shop is clicked while busy
- **THEN** the shop opens the trade window, side 1 the board, side -1 Manage Company, the standing merchant his stock, and the downed merchant and the busy click nothing

### Requirement: Hub busy flag
Opening the match board or Manage Company SHALL make the hub busy and closing either SHALL clear it; the trade window SHALL leave the flag alone.

#### Scenario: Board and manage
- **WHEN** the board is opened, then Manage Company is opened and closed with Escape
- **THEN** the hub is busy while each is open and not busy after the close
