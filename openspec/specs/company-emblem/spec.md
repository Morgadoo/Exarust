# company-emblem Specification

## Purpose
The company emblem a name can select, and the picture that index names.

## Requirements

### Requirement: Klean preset
The name klean SHALL select background 52, design 81, and the four layer bytes 7, 0, 0 and 6. Any other name SHALL select nothing. Background 52 SHALL be named emblembkg052. Design 81 SHALL be named emblemdes081.

#### Scenario: The preset and its pictures
- **WHEN** the name klean is read, and the names Klean and an empty name are read
- **THEN** klean selects background 52, design 81, and the bytes 7, 0, 0 and 6, those pictures are named emblembkg052 and emblemdes081, and the other two names select nothing

### Requirement: Emblem editor start and roll
The emblem editor SHALL start on background 1, design 1, no mirror and the palette indices 0xd3, 0x72, 0x5a and 0x4e, and its randomiser SHALL step `state · 0x8088405 + 1`, take each draw as the high half of `state · n`, roll only the colours' palette bytes, and keep four records of history.

#### Scenario: Seed 1
- **WHEN** the randomiser is seeded with 1
- **THEN** the palette indices are 8, 220, 66 and 70, no mirror is set and the pictures are background 22 and design 7
