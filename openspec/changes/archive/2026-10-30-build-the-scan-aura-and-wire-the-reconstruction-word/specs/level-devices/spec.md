## ADDED Requirements

### Requirement: Scan aura
The system SHALL give a reconstruction-device scan's node the `_ScanAura01` material with a 184-byte parameter block starting with the colour 0.25, 0.25, 0.25, 1.0, state bits 0x9000, one sub-object of buffer usage 35044 whose parent is the node, material flag 0x20 and the shared `_ForceNoise` picture, making the aura's shader object once per run.

#### Scenario: A second scan
- **WHEN** a second scan's aura is built
- **THEN** no new shader object is made and the node still carries the state bits 0x9000

### Requirement: Reconstruction word
The system SHALL set the story session's reconstruction word to rebuildable and the identity beside it to the scanned player's record identity when a reconstruction-device scan starts, SHALL clear the word when the device powers down, and SHALL strip the live player's body when a story death of that identity finds the word rebuildable.

#### Scenario: A scanned death
- **WHEN** a player of record identity 7 is scanned and then dies in the story
- **THEN** the player's body holds nothing, wears the underwear rule and has yellow and red health at 0x7FFF of 0xFFFF of the maximum

### Requirement: Open destination screen
The system SHALL open no destination screen when a teleporter is used while one is already showing.

#### Scenario: A second use
- **WHEN** a teleporter is used while its screen shows
- **THEN** no screen request is written
