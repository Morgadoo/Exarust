# Spec Delta

## ADDED Requirements

### Requirement: Combat mode closes the game screens
Entering combat mode, by the Tab key or by a script's combat-mode call that turns it on, SHALL close every open in-game screen (the inventory, journal, loot and container windows) back to front and then close the power tree; a shown power bar SHALL stay. A call that leaves the mode as it was SHALL close nothing.

#### Scenario: Tab over the open inventory and journal
- **WHEN** `exarust grid "play exanima01 --story-session --press I@100,J@130,Tab@200 --frames 260 …"` runs
- **THEN** both windows are closed at frame 260 and the combat bars show

### Requirement: Control Mind closes the screen family
Control Mind's handover and hand-back SHALL first close the inventory, container, journal and loot windows front to back, then close the power tree, or let the shown power bar fall.

#### Scenario: Handover with the journal open
- **WHEN** the player takes another body by Control Mind while the journal is open
- **THEN** the journal closes before the handover

### Requirement: The journal leaves combat mode
Pressing the journal key SHALL turn combat mode off before the journal opens, is focused or closes, when the player is in action and alive.

#### Scenario: J in combat mode
- **WHEN** `exarust grid "play exanima01 --story-session --press Tab@80,J@130 --frames 200 …"` runs
- **THEN** the journal is open and the combat bars are gone

### Requirement: A save saves the open screens first
A save to a new slot and a writing save step SHALL first save the open screens without closing them: an open container popup's panel SHALL be saved into its container record before the level section is written, and the journal SHALL store its text.

#### Scenario: Saving with a container open
- **WHEN** the game saves while a chest's popup is open
- **THEN** the save holds the chest's contents as the popup shows them
