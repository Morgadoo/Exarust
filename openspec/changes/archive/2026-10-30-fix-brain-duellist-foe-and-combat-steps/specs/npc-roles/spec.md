# Spec Delta

## MODIFIED Requirements

### Requirement: Level characters carry roles
Each living placed character SHALL carry its record's roles, empty relations and a brain that applies the roles when it appears. Every frame the brain SHALL rate the player from both sides' roles and keep that attitude, tolerance and its record's traits in its contact rating. The player SHALL carry the new-character roles in the story, only the company's Faction 1 in the arena, and only Unaligned in practice and in scenes no mode plays.

#### Scenario: A patrolling skeleton
- **WHEN** a skeleton holding Activity Patrol with traits 20 and −30 appears while the player is Player, Human and the unknown background
- **THEN** it patrols and rates the player −50 at tolerance 0 with traits 20 and −30

#### Scenario: The arena sides
- **WHEN** an Unaligned fighter rates the Unaligned practice player, a Faction 2 opponent rates the Faction 1 career player, Faction 1 staff rate that player, and an Unaligned fighter rates a story player
- **THEN** they are −100 at tolerance 0, −100 at tolerance 0, +100 at tolerance 100, and 0
