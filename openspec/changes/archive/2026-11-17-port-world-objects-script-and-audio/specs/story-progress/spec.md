## ADDED Requirements

### Requirement: Device revelation
A usable device of kind 4 SHALL run its record's event word through the revelation: while the session's phase is playing and the event is 0xDA0115, a player without Raise Dead SHALL be granted it by the mind tree, and the narrator SHALL speak RVL_RaiseDead_S, or RVL_RaiseDead_F when the grant fails, after 1000 ms.

#### Scenario: Not yet a necromancer
- **WHEN** a playing player without Raise Dead uses the device and the grant fails
- **THEN** RVL_RaiseDead_F is narrated after a second, and nothing once the player knows the power

### Requirement: Seal unlock
The use of the object with id 0x24c SHALL answer yes only while bit 2 of the background unlock word is clear, set it and write the word to the settings file.

#### Scenario: Two uses
- **WHEN** the seal is used twice
- **THEN** the word gains bit 2 and is written once, and the second use answers no

### Requirement: Journey releases the controls
A journey's level change and the adapter journey's end SHALL release the player's controls from their block.

#### Scenario: A blocked player travels
- **WHEN** an adapter journey ends while the controls are blocked
- **THEN** they are released
