## MODIFIED Requirements

### Requirement: Roam walk in play
The followed leg SHALL be the character's route at urgency 0, walked by the AI's walk toward the follower's carrot with the walk clip, and SHALL end by the follower's stops (a ground point within 30 cm of the carrot at the route's end); a leg's end SHALL settle the look before the pause is drawn.

#### Scenario: Through the door
- **WHEN** a roaming brain in a room whose only door leads east waits out its deadline and gets its turn
- **THEN** its goal is a quarter of the way through the door, its route follows a path there, and after the heading eases it walks east with the walk clip
