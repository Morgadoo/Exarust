## ADDED Requirements

### Requirement: Emblem editor start and roll
The emblem editor SHALL start on background 1, design 1, no mirror and the palette indices 0xd3, 0x72, 0x5a and 0x4e, and its randomiser SHALL step `state · 0x8088405 + 1`, take each draw as the high half of `state · n`, roll only the colours' palette bytes, and keep four records of history.

#### Scenario: Seed 1
- **WHEN** the randomiser is seeded with 1
- **THEN** the palette indices are 8, 220, 66 and 70, no mirror is set and the pictures are background 22 and design 7
