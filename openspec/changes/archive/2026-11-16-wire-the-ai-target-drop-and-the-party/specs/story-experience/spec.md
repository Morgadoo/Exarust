## ADDED Requirements

### Requirement: The party's companions
The story experience SHALL keep the party's companions as bodies with their learners: a joining body SHALL be added once, its learner read from its record and its conversion clock started at the game clock when it never converted; a leaving body SHALL be removed, the last companion taking its place; a companion whose body is gone SHALL leave. After a discovery pass each companion's experience, skills and powers SHALL be written back to its record.

#### Scenario: A follower joins and dies
- **WHEN** a follower's roles are applied twice and it then dies, and another follower joins and is despawned
- **THEN** the party holds the first once with its record's experience and its clock at the game clock, is empty after its death, holds the second, and is empty once the second is gone
