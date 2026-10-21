## ADDED Requirements

### Requirement: Debug path
`--debug-path x,y,z` in play mode SHALL plan from the player's feet to the point with size class 3 and doubled penalties, the player's own footprint lifted while it plans, again whenever the player has moved more than a metre, log each plan and draw it (yellow reaching the goal, orange ending short). Without the flag nothing SHALL run.

#### Scenario: A plan on flat ground
- **WHEN** the player stands on flat ground four cells from the probe's goal
- **THEN** the plan has two points at cost 20 and the player's footprint is back afterwards, and half a metre of movement does not plan again
