## ADDED Requirements

### Requirement: Script relations reach the AI
A script's `AddRelation` and `RemRelation` SHALL change the AI's relations of the actor as well as the script cast, so a relation role decides the attitude the AI works out. An add or a remove that leaves an unknown role among the actor's roles or relation roles toward the partner SHALL report "Invalid role identifier." under "Add Relation" or "Remove Relation".

#### Scenario: A script adds and removes a relation role
- **WHEN** a script adds relation role 0x1E (attitude −60, full weight) toward a partner, then removes it
- **THEN** the AI's attitude toward the partner is −60, then 0 again

### Requirement: Stored topic memory reaches the AI
A dialogue's topic memory stored under a new relation-role entry or block SHALL add that entry and its key to the AI's relations of the owner, with no report.

#### Scenario: A new memory block
- **WHEN** a dialogue stores its memory under relation role 0x1E with key 4
- **THEN** the AI's relations hold role 0x1E toward the partner with value 4

### Requirement: Arrival unselects the level relations
A follower arriving on a new level SHALL drop its current level relation table selection and SHALL raise its readiness to at least 5.

#### Scenario: A follower arrives
- **WHEN** a follower with a stored level table selected arrives
- **THEN** no table is current, the stored table stays, and its readiness is 5
