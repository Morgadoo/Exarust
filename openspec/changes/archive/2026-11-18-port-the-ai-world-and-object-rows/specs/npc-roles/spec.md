## ADDED Requirements

### Requirement: A follower's former level
A character placed from a record with an origin SHALL carry that level and its local id there, and the trust and caster relations asked about it SHALL move the relation an observer kept in that level's table into the global table under the character's whole id, answering the moved record; a global record already under that id SHALL refuse the move.

#### Scenario: A follower who left level 5
- **WHEN** an observer that kept attitude 100 about local character 7 of level 5 sees a barricade raised by world character 0x40000003 whose former level is 5 and id 0x80000007
- **THEN** the barricade's fear is 0 and the observer's level table no longer holds the record

### Requirement: The moved relation answers once
The lookups SHALL key a world character by its id less the tag bits, so a relation moved under the tagged id SHALL answer only the ask that moved it, and the next ask SHALL make a new relation answering 0, as the original does.

#### Scenario: Asked twice
- **WHEN** the last attitude about a follower whose moved record holds 33 is asked twice
- **THEN** the first answer is 33 and the second is 0
