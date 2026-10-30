# Spec Delta

## ADDED Requirements

### Requirement: Background unlock write
A story script's `GameEvent(4)` SHALL, when the background word lacks bit 4, set it and write the whole 4-byte word under settings path [1, 0x6431ac02] once, whatever the game mode; a second call SHALL write nothing.

#### Scenario: Two calls on a word of 1
- **WHEN** a script calls `GameEvent(4)` twice with the background word 1
- **THEN** the first call writes [5, 0, 0, 0] under [1, 0x6431ac02] and the second writes nothing
