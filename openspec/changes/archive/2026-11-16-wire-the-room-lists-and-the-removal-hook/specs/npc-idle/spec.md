## MODIFIED Requirements

### Requirement: The nearby gather walks the room and its exits
The AI tick SHALL gather the nearby list only while the brain is present: from its level room's list without itself, then from the list of the room behind each of its exits in turn, an exit leading to the same room as the exit before it skipped and the brain not skipped there. Each candidate SHALL pass the sight test, the room-to-room light reach. A brain in no room, or on a level without a room table, SHALL keep its list.

#### Scenario: Two characters in the next room
- **WHEN** a brain's room has an open door to a room holding one character within 2 m of the door point and one far from it
- **THEN** the nearby list holds the first and not the second
