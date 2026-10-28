## MODIFIED Requirements

### Requirement: Wander point
A walker with no room or no exits SHALL not pick nor step the roll. A pick SHALL draw one exit, and draw once more (keeping that draw) when it leads to the avoided room. The goal SHALL be a quarter of the way from the door's near cell centre to the centre of the cell behind it, at height 0, stop 20 cm, radius 150 cm, ignoring height; the pick SHALL remember the room behind the exit and its place.

#### Scenario: A quarter of the way, unless the exit leads back
- **WHEN** a walker in room 0 draws its only door, from a near cell at the origin to a cell at 600 cm on x, leading to room 5, the roll starting at 1, and another walker whose first draw leads to the avoided room has a second door leading to room 7 from the origin to −600 cm on x
- **THEN** the first goal is 150 cm on x at height 0, stop 20 cm, radius 150 cm, room 5, place 0, with the roll at 134775814, and the second walker draws again, takes the second door and its goal is −150 cm on x

## ADDED Requirements

### Requirement: Idle attention
An idle state SHALL attend the contact the target pass attends when its notice score `(concern + hostility)·6 + (noticed − attention)` is not under 1 (a NaN score attends). The look SHALL take the yaw to the contact when the wrapped gap from the look passes the window `clamp(1 − 0.1·score, 0.25, 0.7)·π` (50° while the scripted focus holds). Attending SHALL turn the body and put a roam or patrol back in its wait.

#### Scenario: Window
- **WHEN** a contact with score 5 lies 1 rad from the look, then 2 rad
- **THEN** the first leaves the look and the second takes it, both attend, and a score of 0.5 does not attend

### Requirement: Idle head
Every waiting brain SHALL first run the alert look-around (panic at least 0.2 with the body's in-combat mark), then the attention; only when neither takes the tick SHALL it clear the in-combat mark and run its phase. Fighting SHALL set the mark.

#### Scenario: Calm brain
- **WHEN** a waiting brain has panic 0 and nothing attended
- **THEN** its mark is cleared and its roam phase runs

### Requirement: Roam planning in play
A roaming or patrolling brain in the scene, standing, out of combat and not fleeing SHALL plan on its turn of the one-planner-a-frame turn, picking its goal only then. A turn SHALL end the planning phase whether or not the route is followed; a wander route SHALL be followed only past 150 cm end to end and a home-side route always.

#### Scenario: Short route
- **WHEN** a patrol's wander route is 50 cm long
- **THEN** it is not followed, the walk ends on the next step and the patrol pauses 0.5 to 2.5 s

### Requirement: Roam walk in play
The followed route SHALL be walked at the walk speed toward its carrot with the walk clip and SHALL end within 100 cm of its last point; a leg's end SHALL settle the look before the pause is drawn.

#### Scenario: Through the door
- **WHEN** a roaming brain in a room whose only door leads east waits out its deadline and gets its turn
- **THEN** its goal is a quarter of the way through the door, it follows a route there, and after the heading eases it walks east with the walk clip

### Requirement: Entering walker facing
A brain the scene takes back while its walk follows a route SHALL face the route's goal; without a route, or collapsing, it SHALL keep its facing.

#### Scenario: Mid-leg
- **WHEN** a brain following a route ending 500 cm along x enters the scene
- **THEN** it faces +x
