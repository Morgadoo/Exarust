## ADDED Requirements

### Requirement: Controls blocked
The player's controls SHALL have a block byte, set by the knock-out, the death, the practice ladder's last win and the arena manager's fall, and cleared by the revive and the start of play. While it is set the player's movement, behaviour, mouse capture, attack, casting and kinesis inputs SHALL read nothing and the left press SHALL pick nothing; the hotkeys SHALL still run. Setting it SHALL clear the move request and locomotion flags once, and the idle camera SHALL turn ten seconds later.

#### Scenario: The idle camera turns behind a block
- **WHEN** the controls have been blocked for more than ten seconds
- **THEN** the camera turns whatever the pointer does

#### Scenario: A knock-out blocks the controls until the revive
- **WHEN** the story player is knocked out and revived five seconds later
- **THEN** the controls are blocked at the knock-out and released at the revive

### Requirement: Controls second-blocked under the loadout form
The player's controls SHALL have a second block byte, set while the arena hub's loadout form is open. While it is set the same inputs SHALL read nothing, the idle camera SHALL not run and the hotkeys SHALL be refused; the locomotion flags SHALL be left as they are.

#### Scenario: Walking behind the hub's loadout form
- **WHEN** the arena hub's loadout form is open and the forward key is held, then Tab pressed
- **THEN** the player does not move and combat mode is not entered
