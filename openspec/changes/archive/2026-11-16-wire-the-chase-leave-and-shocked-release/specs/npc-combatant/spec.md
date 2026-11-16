## ADDED Requirements

### Requirement: Chase leave on a lost foe
A fighter whose chase follows no route SHALL leave combat when its brain's integration no longer sees its foe (a fighter without a brain always sees it): its target, focus and level SHALL be cleared and it SHALL return to its idle state.

#### Scenario: The foe is out of sight
- **WHEN** a stuck fighter reaches the chase while its brain's fresh mark is clear
- **THEN** it leaves combat with no target, no focus and level 0

### Requirement: Chased foe not in the way
Each chase tick SHALL remember the body's target as the body in the way of its walk, so the route it follows is not stopped by the foe it runs after.

#### Scenario: A seen foe is chased
- **WHEN** a stuck fighter that sees its foe reaches the chase
- **THEN** it follows its route and its driver's body in the way is the foe
