# npc-idle Specification

## Purpose
How an idle character roams, patrols or stands sentry when nothing else has its attention.

## Requirements

### Requirement: Idle cycles
A roam SHALL wait, walk to a wander point, wait 500 ms plus a draw of 2000 ms, walk to a home-side point, wait 1000 ms plus a draw of 4000 ms, and repeat. A patrol SHALL wait 500 ms plus a draw of 2000 ms between wander walks and SHALL NOT walk home. A sentry SHALL hold the guard pose. A deadline equal to the clock SHALL keep the wait. During a wait, a disturbance above 0.3 SHALL be walked to without advancing the phase.

#### Scenario: Out and back
- **WHEN** a roam whose wait ended at 1000 ms is stepped at 1001 ms, accepts a path, arrives at the wander point at 2000 ms, accepts the home path, and arrives home at 3000 ms, the generator having started at 0, and a patrol does the same arrival at 10 ms, and a waiting roam hears a disturbance of 0.31
- **THEN** the roam's first pause ends at 2500 ms and its home pause ends at 4125 ms, the patrol's pause ends at 510 ms and its next plan is another wander, and the noise leaves the roam waiting
