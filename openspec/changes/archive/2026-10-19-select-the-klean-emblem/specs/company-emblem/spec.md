# Spec Delta

## Purpose
The company emblem a name can select, and the picture that index names.

## ADDED Requirements

### Requirement: Klean preset
The name klean SHALL select background 52, design 81, and the four layer bytes 7, 0, 0 and 6. Any other name SHALL select nothing. Background 52 SHALL be named emblembkg052. Design 81 SHALL be named emblemdes081.

#### Scenario: The preset and its pictures
- **WHEN** the name klean is read, and the names Klean and an empty name are read
- **THEN** klean selects background 52, design 81, and the bytes 7, 0, 0 and 6, those pictures are named emblembkg052 and emblemdes081, and the other two names select nothing
