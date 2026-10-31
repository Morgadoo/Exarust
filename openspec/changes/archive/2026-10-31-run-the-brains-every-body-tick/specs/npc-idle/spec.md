# Spec Delta

## MODIFIED Requirements

### Requirement: Roam planning in play
A roaming or patrolling brain in the scene, standing, out of combat and not fleeing SHALL plan on its turn of the one-planner-a-tick turn, picking its goal only then. A turn SHALL end the planning phase whether or not the route is followed; a wander route SHALL be followed only past 150 cm end to end and a home-side route always.

#### Scenario: Short route
- **WHEN** a patrol's wander route is 50 cm long
- **THEN** it is not followed, the walk ends on the next step and the patrol pauses 0.5 to 2.5 s
