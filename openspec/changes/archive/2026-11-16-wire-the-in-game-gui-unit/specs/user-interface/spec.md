## ADDED Requirements

### Requirement: Heartbeat fade-out
A death, a knock-out and the death fade's reset SHALL start a running heartbeat's fade-out; its sound's gain SHALL then be `0.3 · (1 − t/1000)` for the milliseconds `t` since the start, and the heartbeat SHALL stop once that share falls under the GUI's invisible threshold. A heartbeat that is not running SHALL take no fade.

#### Scenario: Death during a heartbeat
- **WHEN** the player dies while the heartbeat plays
- **THEN** its sound fades out over one second and the border stops with it

### Requirement: HUD shield gauge input
The HUD SHALL show the block-stamina gauge's input from a shield in the player's off hand, filled to the shield arm's stamina, so the gauge rises in combat mode with a shield.

#### Scenario: Shield in combat
- **WHEN** the player holds a shield in combat mode
- **THEN** the gauge above the bars shows the arm's stamina

### Requirement: HUD power bar flag
The HUD's bars SHALL be held shown while the power bar is up: from its opening toggle until its 900 ms closing animation has run.

#### Scenario: Bar put away
- **WHEN** the power bar is put away and its closing still runs
- **THEN** the bars are still held shown

### Requirement: HUD bar holds
A quick-bar cast from a slot holding a power, and a step of the player's healing charge, SHALL hold the HUD's bars shown for 1000 ms of game time.

#### Scenario: Quick-bar cast
- **WHEN** the player casts from a set quick-bar slot
- **THEN** the bars stay shown for a second

### Requirement: HUD reset clears the hold
The HUD's reset, run while no player is shown, SHALL clear the bars' hold clock with its fades and last values.

#### Scenario: New session
- **WHEN** a session starts after a hold was set
- **THEN** the hold does not carry into it

### Requirement: Dead powers close the body's windows
Raise Dead and Command Dead SHALL close the loot window open on the body they raise; a Loyalty or Command Dead mark's end SHALL focus the GUI's root and close an inventory open on its body.

#### Scenario: Raising a looted corpse
- **WHEN** the player raises a corpse whose loot window is open
- **THEN** that window closes

### Requirement: AI search sees the open containers
The AI search's open check SHALL read the GUI's registry of open containers: the inventory's and the loot windows' characters and the open container popups.

#### Scenario: An open inventory
- **WHEN** the player has a character's inventory open
- **THEN** that character's pack counts as open to the search

### Requirement: Deactivation reaches every task
The window's focus loss SHALL also deselect the main menu's and the power tree's focused controls, their roots keeping the focus with control -1.

#### Scenario: Alt-tab on the menu
- **WHEN** the window loses the focus with the main menu up
- **THEN** the menu's focused control is deselected

### Requirement: Container popups rescale
On a GUI unit change an open container popup SHALL take the new unit: its panel laid out from the record's cells at it, its items' places scaled and its form resized at its scaled place.

#### Scenario: Window resize
- **WHEN** the window's height changes the unit with a popup open
- **THEN** the popup and its items follow the new unit

### Requirement: Inventory mind window
The inventory's fill SHALL set its view's mind window when the shown body carries a Loyalty or possession marker; a drag out of that pack, on a body with Volition, SHALL end the player's hold on it, refuse the drag and close the window.

#### Scenario: Robbing a willful follower
- **WHEN** the player drags an item out of the pack of a Loyalty-bound body with Volition
- **THEN** the hold ends and the inventory closes

### Requirement: Open packs save on a game save
A game save SHALL write the open inventory's and loot windows' loaded packs into their bodies' carried lists, in the panels' order and at their places, without closing them.

#### Scenario: Saving with the inventory open
- **WHEN** the game saves while the inventory is open
- **THEN** the carried list takes the pack's order and places
