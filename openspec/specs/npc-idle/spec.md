# npc-idle Specification

## Purpose
How an idle character roams, patrols or stands sentry when nothing else has its attention.

## Requirements

### Requirement: Idle cycles
A roam SHALL wait, walk to a wander point, wait 500 ms plus a draw of 2000 ms, walk to a home-side point, wait 1000 ms plus a draw of 4000 ms, and repeat. A patrol SHALL wait 500 ms plus a draw of 2000 ms between wander walks and SHALL NOT walk home. A sentry SHALL hold the guard pose. A deadline equal to the clock SHALL keep the wait. During a wait, a disturbance above 0.3 SHALL be walked to without advancing the phase.

#### Scenario: Out and back
- **WHEN** a roam whose wait ended at 1000 ms is stepped at 1001 ms, accepts a path, arrives at the wander point at 2000 ms, accepts the home path, and arrives home at 3000 ms, the generator having started at 0, and a patrol does the same arrival at 10 ms, and a waiting roam hears a disturbance of 0.31
- **THEN** the roam's first pause ends at 2500 ms and its home pause ends at 4125 ms, the patrol's pause ends at 510 ms and its next plan is another wander, and the noise leaves the roam waiting

### Requirement: Wander point
A walker with no room SHALL not pick. A pick SHALL be one exit, drawn by the roll, and a second draw only when that exit leads back to the room the walker came from. The goal SHALL be a quarter of the way from the exit toward the room point, at height 0, with a stop of 20 cm and a radius of 150 cm, ignoring height. A second draw of the same exit SHALL still be kept.

#### Scenario: A quarter of the way, unless the exit leads back
- **WHEN** a walker with no room is asked to pick, a walker draws the only exit from the origin toward a room point 100 cm ahead, a walker draws an exit 100 cm deep toward a room point at the origin, a walker whose first exit leads back draws the next exit 200 cm ahead, and a walker whose only exit leads back draws again
- **THEN** the walker with no room picks nothing and the roll stays, the ahead goal is 25 cm forward at height 0 with stop 20 cm and radius 150 cm after one step of the roll, the deep goal is 75 cm deep at height 0, the retry uses the second exit at 150 cm and steps the roll twice, and the only exit is kept after two steps
