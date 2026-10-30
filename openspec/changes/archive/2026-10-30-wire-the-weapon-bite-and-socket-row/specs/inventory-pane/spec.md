# Spec Delta

## ADDED Requirements

### Requirement: Item card socket row
A stats card whose item has its first crystal socket SHALL draw the socket row after its striking rows, 0x18 high and 0x78 wide at unit 4 and scaled with the unit, and its description SHALL start that much lower. The row's cells SHALL be the interface level's large-icon size, centred in its height. Each existing socket SHALL draw the rating-dot sheet's empty ring five units after the last, in the label group's eighth ink.

#### Scenario: Row size
- **WHEN** the socket row is laid out at unit 4 and at unit 8
- **THEN** it is 0x18 and 0x30 high, and only a stats card whose content has the row with socket words draws it

#### Scenario: Installed socketed records
- **WHEN** the installed object database is read
- **THEN** it has socketed records, each with its first socket's existence bit

### Requirement: Item card socket crystals
After the rings, each socket with a crystal SHALL draw at its own slot, five units apart, the crystal sheet's frame (cell 1, or 2 for a kind with bit 0x10) in opaque white, then the crystal (cells 3, 4, 5 and 6 for A, B, C and AR) added in white at alpha 255 less its charge.

#### Scenario: A row of three sockets
- **WHEN** the sockets 0x0280, 0xC884, 0x0791 and 0 are drawn at x 100 in a row 24 high at y 50 with 20-pixel cells at unit 4
- **THEN** three rings are drawn at 100, 120 and 140, the C crystal's frame cell 1 opaque and crystal cell 5 at alpha 55 at 120, the AR crystal's frame cell 2 and crystal cell 6 at alpha 248 at 140, all at y 52, and a row with no socket draws nothing
