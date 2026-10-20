## ADDED Requirements

### Requirement: Debug path
`--debug-path x,y,z` in play mode SHALL plan a path from the player's feet to the point as an NPC's planner does (its goal, the one-plan-a-tick slot, its own footprint lifted), again whenever the player has moved more than a metre, log each plan and draw it (yellow reaching the goal, orange ending short). The navigation overlay SHALL mark cells with a penalty (footprints) with an orange square.

#### Scenario: Around the opponent
- **WHEN** `exarust --still-opponent --debug-nav --debug-path 0,0,-3 play arena` runs
- **THEN** the log gives the plan from the player's start and the overlay shows the opponent's footprint in orange
