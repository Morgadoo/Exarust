## ADDED Requirements

### Requirement: The flight walk
Once a body tick a brain whose target pass stands at level 4, not fighting and present, SHALL run toward the way out chosen from its room's exits away from its foe, choosing again within 150 cm of it, and SHALL keep that velocity on the frames between ticks; when the flight ends it SHALL stop and put its idle clip back.

#### Scenario: Out of the room
- **WHEN** a brain at level 4 stands in the first of two rooms with its foe far away
- **THEN** it turns toward the centre of the cell behind the door and jogs there, and stops in its idle clip once its level drops to 0
