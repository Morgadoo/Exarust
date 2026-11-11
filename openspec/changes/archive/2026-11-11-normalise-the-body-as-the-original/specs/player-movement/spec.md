## MODIFIED Requirements

### Requirement: Walk start
When locomotion finds another clip playing, the walk SHALL start from where the body is: more than 600 ms of game time after the last locomotion at key 2 ahead, 4 backwards, 5 (8 at a run blend of 0.5 or less) with the heading more than 1 rad on the yaw's negative side, its clock at the key before; within 600 ms at the kept clock. Beyond 1 rad on the yaw's positive side (a keyless start), and within 600 ms, the walk SHALL keep the jump-in key the playing clip left: the key it still leads in to from its capture, or none once its lead-in has passed. A keyless start SHALL take its clock at the key before the kept one (0 for key 0 or none); with none, the walk SHALL have no lead-in from the capture. The boost ramp SHALL be 0 at a start and rise 0.5/83 a tick to 1 while the walk plays, × the alignment.

#### Scenario: From idle
- **WHEN** a body that has not walked starts moving ahead from Idle
- **THEN** the walk plays from frame 10 with its capture meant for key 2

#### Scenario: Within 600 ms
- **WHEN** it stops and moves again 600 ms after it last walked, and again 612 ms after
- **THEN** the walk resumes at its kept clock, then starts at key 2 again

#### Scenario: Turning the other way from a settled stance
- **WHEN** a body whose stance clip has passed its lead-in starts walking with its heading more than 1 rad on the yaw's positive side
- **THEN** the walk starts at clock 0 and enters from its own last key, not from the capture

#### Scenario: Turning the other way during a lead-in
- **WHEN** the playing clip still leads in to its key 3
- **THEN** the walk starts at its key 2's time and leads in to its key 3 from the capture
