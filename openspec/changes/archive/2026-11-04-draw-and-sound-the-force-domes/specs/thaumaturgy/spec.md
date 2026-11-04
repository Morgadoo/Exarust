## ADDED Requirements

### Requirement: Force dome draw
The system SHALL run each force Barrier's and force Bubble's frame step every frame at the fixed step's fraction and draw its mesh (the Barrier's cap with y and z exchanged, the Bubble's sphere, at the dome's radius, every face reversed then as built): the Barrier turned to its blended facing at its blended place, the Bubble unturned at its place. The draw SHALL leave with the dome.

#### Scenario: A Barrier ends
- **WHEN** a standing Barrier's hook is called
- **THEN** its draw shows `Fade` as the shrinking share and leaves once the share passes 1

### Requirement: Barrier texture position
The Barrier's texture position SHALL be the facing's yaw over 2π plus its first seed and a half, and its drawn place's height times 0.001 plus its second seed.

#### Scenario: A raised Barrier
- **WHEN** the drawn place rises by 100 cm
- **THEN** the second coordinate grows by 0.1

### Requirement: Force dome sounds
The system SHALL play a force dome's cast sound `thmfrc_ss` at the cast's place, volume and pitch, `thmfrc_bi` at each contact's volume and pitch, and the Bubble's `thmfrc_brf` at its place, volume 0.3 and pitch `0.6 + 0.2·r`, when it breaks.

#### Scenario: The Bubble breaks
- **WHEN** a standing Bubble's power ends
- **THEN** `thmfrc_brf` plays once at the Bubble's place

### Requirement: Force dome hum
The system SHALL start a looping `thmfrc_bc02` voice for each force dome at volume 0 and pitch 1, 0.8·r of its length in, set its volume, pitch and place from each frame (the Bubble's place, 80 cm ahead of the Barrier's bounds centre along its facing) and free it with the dome.

#### Scenario: A dome leaves
- **WHEN** a dome leaves the scene
- **THEN** its hum voice is freed
