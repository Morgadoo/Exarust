# Spec Delta

## ADDED Requirements

### Requirement: Shut door
A shut door SHALL NOT load the room behind it. The same door open SHALL load that room as a neighbour of the active room.

#### Scenario: Closed then open
- **WHEN** the anchor stands in a room whose only door is shut, then the door is open
- **THEN** the room behind the door is hidden, then adjacent

### Requirement: Portal clip
A point SHALL be seen through a doorway only when the segment from the eye crosses the opening and the door is open. A side nibble of 0 SHALL be 82 × 245 cm, any other nibble 115 × 290 cm, and a special tile 200 × 340 cm.

#### Scenario: Through and beside
- **WHEN** the eye looks through the middle of an open doorway, then beside it, then through it while the door is shut
- **THEN** only the first view is seen

### Requirement: Level passage
A passage SHALL load the level its destination id names. An id the list does not name SHALL keep the current level.

#### Scenario: Named destination
- **WHEN** a passage's destination is the id of exanima02
- **THEN** the loaded level is exanima02

#### Scenario: Unknown id
- **WHEN** a passage's destination is not in the level list
- **THEN** the loaded level stays exanima01
