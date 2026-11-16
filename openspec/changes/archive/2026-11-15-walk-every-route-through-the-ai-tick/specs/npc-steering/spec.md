## ADDED Requirements

### Requirement: One route per character
A character SHALL keep one route: its points, the follower's segment and fraction, whether it is followed, whether a plan is wanted, the walk's urgency and the after-route count. Every state that walks SHALL plan into that route, and a plan the start test refuses SHALL stop it.

#### Scenario: A roam plans its leg
- **WHEN** a roaming character plans a leg on its turn
- **THEN** its route holds the leg's points, is followed at urgency 0 and belongs to the roam

### Requirement: The walk follows the state
The AI's walk SHALL run once a body tick after every state of the brains' tick, for each character whose route is followed, and SHALL not run for a character whose route is not followed.

#### Scenario: The leg is walked in the tick it is planned
- **WHEN** a roaming character on its locomotion plans its leg facing the route
- **THEN** in the same tick its request's goal lies 150 cm out along the follower's carrot, its flags ask only to move, it moves, and the AI's look yaw is the eased heading's

### Requirement: The walk writes its flags after the follower
The walk SHALL write the request's flags after the follower ran, so a soft stop the follower made at the route's end SHALL leave the request asking only to move toward the pulled-back goal, the route not followed and the after-route count at 80.

#### Scenario: The leg's end
- **WHEN** the roaming character stands on its route's last point
- **THEN** its route is no longer followed, the after-route count is 80, its request asks only to move toward that point and does not move, and the roam settles on the next tick

### Requirement: The after-route count
A walk asked while no route is followed SHALL answer 2 and count down while the after-route count is above 0, and SHALL answer 0 once it is 0.

#### Scenario: Counting down
- **WHEN** the walk is asked twice after its route ended, then with the count at 0
- **THEN** it answers 2 with the count 79, then 0

### Requirement: The head looks at the look target
While walking, the head SHALL look at the AI's look target when it lies within a quarter turn of the look (the offset easing 0.02 toward its yaw), and otherwise SHALL ease its offset 0.02 toward 0 in double; the cursor SHALL be the look yaw plus the offset, wrapped once.

#### Scenario: A target in front, then none
- **WHEN** a walk looks at a point 45° to the side, then walks with no target
- **THEN** the offset's target is that yaw and the offset 0.02 of it, then the target is 0 and the offset eases back 0.02 of the way

### Requirement: The follower asks no end test
The path follower SHALL end a route only by its own stops (fewer than two points, a step of no length, the soft stops at its end); the goal record's end test SHALL be asked only by the planner.

#### Scenario: A route's end within 150 cm
- **WHEN** a roam's route, whose goal record holds the flat test at 150 cm, is walked to 120 cm from its last point
- **THEN** the route is still followed

### Requirement: A downed body stops its route
The AI tick SHALL stop the route of a character whose condition has the down bit, whichever state planned it.

#### Scenario: Knocked down mid-leg
- **WHEN** a roaming character is knocked down while its leg is followed
- **THEN** its route is no longer followed

## MODIFIED Requirements

### Requirement: Path carrot
A ground point SHALL project onto the nearest path segment in 3-D and keep the unclamped fraction. The goal SHALL be 100 cm further along that path, measured from the unclamped foot. An equal distance SHALL keep the earlier segment. A segment with no horizontal length SHALL be skipped. Exactly 100 cm left SHALL spill onto the next segment, and a step of no length SHALL stop the route.

#### Scenario: One hundred centimetres past the foot
- **WHEN** a ground point 100 cm along a 150 cm segment, 20 cm off it, is projected onto a path that then turns
- **THEN** its fraction is two thirds and the goal lies 50 cm along the next segment

### Requirement: Out of scene
Out of the scene a step SHALL advance 2 cm along the segment. The standing height SHALL add the pelvis base, and the ground point SHALL keep the path height. A length not strictly past the fraction times the length plus 2 SHALL cross once: with `a = 1 − (1 − f)·L`, the new fraction SHALL be `a − (f·a + 2)` over the next segment's flat length. No horizontal length SHALL count as 0.1.

#### Scenario: Two centimetres, then a crossing
- **WHEN** a body out of the scene steps twice from the start of a 3 cm segment with a 400 cm segment after it
- **THEN** the first fraction is two thirds, and the second step crosses onto the next segment at the original's sum

### Requirement: Run and stop
Urgency 0 SHALL set the run bit until the carrot fraction, as a double, is past 0.99 on the last segment; past that the run bit SHALL clear and a ground point under 30 cm of the carrot (3-D) SHALL soft-stop. Urgency above 0 SHALL clear the run bit and soft-stop under 100 cm of the last point (3-D).

#### Scenario: The run bit clears, then a close body stops
- **WHEN** a walk at urgency 0 stands 120, 104 and 20 cm short of its route's end, and a hurried walk 101 and 99 cm short
- **THEN** 120 cm keeps the run bit, 104 cm clears it without stopping, 20 cm soft-stops, 101 cm goes on and 99 cm soft-stops with the goal pulled to 20 cm ahead

### Requirement: The AI walks on its locomotion
A character whose body has muscles SHALL walk its route through the AI's one walk, which writes its body's locomotion request each tick, and its character SHALL take no walk velocity; its body's movement decision SHALL turn and walk it on its feet. A body without muscles SHALL be steered toward the follower's goal by the stand-in, with its walk clip.

#### Scenario: A roaming NPC
- **WHEN** a roaming NPC with muscles follows a route
- **THEN** its request moves toward the route's carrot and its body walks with its feet

### Requirement: The driver's request
While the body steps the walk SHALL write nothing. Otherwise it SHALL ease the heading 0.1 toward the goal's direction, ask only to move (moving past half the gate) while the pelvis's alignment with the goal is not over the gate (0.7 past 100 cm, else `0.007·|d|` in double) or the route is no longer followed, and otherwise put the goal 150 cm out, move, and run at urgency above 0 when the grid's cost at the goal is not over 0.5 and the alignment passes 0.9, with the sprint latch at urgency 2.

#### Scenario: Urgency on cheap ground
- **WHEN** a route runs 10 m ahead at urgencies 0, 1 and 2 with no grid
- **THEN** the goal is 150 cm out and the flags are move, move and run, and move, run and sprint

### Requirement: The soft stop
A soft stop SHALL end the route, clear the run bit, set the stopping bit, pull a goal farther than `8·speed + 20` cm to that distance along the frame's forward, and walk the line from the ground point to the goal through the grid, which clamps it where the grid blocks.

#### Scenario: A far goal
- **WHEN** a route soft-stops with the goal 100 cm ahead at 0.5 cm a tick
- **THEN** the goal is 24 cm ahead and the stopping bit is set

### Requirement: The out-of-scene slide
While a character is out of the scene and follows a route, whichever state planned it, its standing point SHALL slide 2 cm a tick along the route with no physics, and the route SHALL end by the follower's own stops.

#### Scenario: A walker leaves the range
- **WHEN** a walker leaves the activation range mid-route
- **THEN** its standing point moves 2 cm a tick along the route until the route ends

### Requirement: One AI state drives the walk
When combat, a flight or the scene takes a roaming character's body, a leg of its idle walk that is followed SHALL end with the soft stop once; otherwise the idle walk SHALL NOT touch the character's route, so a fighter's chase keeps its own.

#### Scenario: A fighter chasing
- **WHEN** an engaged fighter's driver is driving and its route is not the roam's
- **THEN** after the idle walk's tick the driver still drives and the stopping bit is not set
