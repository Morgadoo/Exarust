## MODIFIED Requirements

### Requirement: The chase walk's arrival
A chasing fighter's route SHALL be the character's route at the chase's gait, walked by the AI's walk; the follower SHALL soft-stop it once the ground point is within 100 cm (3-D) of its last point. A combat tick that does not reach the chase, or stops a running body, and a fighter that leaves combat SHALL let the chase's route go.

#### Scenario: Planned and walked
- **WHEN** a fighter stuck for 21 ticks plans its chase on its turn
- **THEN** its route follows the plan at the chase's gait and the stand-in walks its body toward the foe
