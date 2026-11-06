## ADDED Requirements

### Requirement: The AI walks on its locomotion
A character whose body has muscles SHALL walk its route (the roam's and patrol's legs, the flight, the combat chase and the follower's walk) by writing its body's locomotion request each tick through the AI's driver, and its character SHALL take no walk velocity; its body's movement decision SHALL turn and walk it on its feet.

#### Scenario: A roaming NPC
- **WHEN** a roaming NPC with muscles follows a route
- **THEN** its request moves toward the route's carrot and its body walks with its feet

### Requirement: The driver's request
While the body steps the driver SHALL write nothing. Otherwise it SHALL ease the heading 0.1 toward the goal's direction, ask only to move (moving past half the gate) while the pelvis's alignment with the goal is not over the gate `min(0.007·|d|, 0.7)`, and otherwise put the goal 150 cm out, move, and run at urgency 1 or more when the grid's cost at the goal is at most 0.5 and the alignment passes 0.9, with the sprint latch at urgency 2.

#### Scenario: An aligned goal
- **WHEN** the goal lies 1 m straight ahead at urgency 0
- **THEN** the goal moves to 1.5 m, the body moves and walks

#### Scenario: Urgency on cheap ground
- **WHEN** the goal lies 3 m ahead at urgency 2 with cost 0
- **THEN** the flags are move, run and sprint

### Requirement: The soft stop
A route's end SHALL clear the run bit, set the stopping bit and pull a goal farther than `8·speed + 20` cm to that distance along the frame's forward.

#### Scenario: A far goal
- **WHEN** a route ends with the goal 5 m away at 2 cm a tick
- **THEN** the goal is 36 cm ahead and the stopping bit is set
