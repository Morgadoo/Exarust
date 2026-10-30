# Spec Delta

## MODIFIED Requirements

### Requirement: Trigger zones
A zone SHALL be entered within its radius at a shape distance of at most 0.1 after one of 0.1 or more, at the character's frame anchor (root x and z, pelvis height). An entered `0x1xx` zone of kind `0x10` SHALL run its routine for anyone; for the player, kind 2 is the golem alarm and bit `0x20` the level exit (`0x21` flagged) or a host's release. A `0x2xx` zone of kind `0x10` SHALL run while the player is inside.

#### Scenario: Zone kinds
- **WHEN** a character enters a script zone, the player enters an alarm zone while held, and enters an exit zone while hosting
- **THEN** the routine runs, the held player ends the pass, and the host is released

## ADDED Requirements

### Requirement: Live script zones
Every frame, each character that moved more than 3 cm from its last checked point SHALL check the level's script zones in placement order, move its checked point, and run the routine of each zone it has just entered.

#### Scenario: Entered once
- **WHEN** a character steps from 2 m outside the checkpoint zone into it and then 5 cm further inside
- **THEN** the routine runs once
