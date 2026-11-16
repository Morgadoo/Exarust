## ADDED Requirements

### Requirement: Alarm reset keeps the pass
A level start's alarm reset SHALL keep the pass word of the zone that last armed the alarm.

#### Scenario: A level start after a ring
- **WHEN** the gate with pass 474 armed the alarm and a level starts
- **THEN** the alarm is disarmed with its count at 0 and its pass still 474
