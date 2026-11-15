## ADDED Requirements

### Requirement: The flight ends without a threat
The flee state SHALL end a flight whose body has neither a target nor an object focus, as it ends an escaped one: level 0 and a calm clock 10 to 20 s out. Ending a flight SHALL clear the body's target and the object focus.

#### Scenario: A flight whose threat was dropped
- **WHEN** a brain at level 4 with no body target and no object focus runs its flee tick
- **THEN** its level is 0 and its calm clock is at least 10 s out

### Requirement: A cornered flight fights
When the flight's way out finds no exit, or the brain stands in no room, the flight SHALL be cornered: the level becomes 3, the courage doubles and the combat state becomes current, fighting the body's target.

#### Scenario: A brain in no room
- **WHEN** a fleeing brain with courage 0.4 and a character as its body's target plans its way out from a place in no room
- **THEN** its level is 3, its courage 0.8, it has no flight goal and its duellist is engaged

### Requirement: The flight marks the body in combat
Every flee tick SHALL set the body's in-combat mark, so the alert look-around runs once the flight is over.

#### Scenario: A flight that ends
- **WHEN** a flight ends while the brain's panic is still above 0.2
- **THEN** the body keeps its in-combat mark for the idle state's alert look
