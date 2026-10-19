# npc-hearing Specification

## Purpose
What an idle character does about a noise or an alert before its own activity.

## Requirements

### Requirement: Noise and alert
A disturbance above 0.3 SHALL name the point to walk to, ahead of an alert. An alert of 0.2 or more SHALL face a known target more than 45° off, and otherwise SHALL look around once a gap of 0.7 to 3 seconds has elapsed. A disturbance of 0.3, or an alert under 0.2, SHALL leave the character on its own activity.

#### Scenario: A noise, then a look
- **WHEN** the disturbance is 0.3 with no alert, then 0.31 at a point, then an alert of 0.2 with a target just past 45°, then the same alert with a near target at 0.69 seconds of a 0.7 second gap, then at 0.7 seconds
- **THEN** the answers are the idle, a walk to that point, facing the target, the idle, and a look around
