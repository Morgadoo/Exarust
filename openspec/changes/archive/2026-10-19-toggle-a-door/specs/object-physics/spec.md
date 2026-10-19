# Spec Delta

## ADDED Requirements

### Requirement: Hinge toggle
A toggle SHALL open a latched door before it chooses. A hinge's openness SHALL be its angle minus closed, times -0.7. Above 0.7 the aim fraction SHALL be 1, and otherwise 0, including exactly 0.7. The hinge SHALL drive that aim for 100 ticks, or 60 when fast, and wake for 256 ticks.

#### Scenario: Shut opens, and 1.1 rad short closes
- **WHEN** a shut hinge is toggled, a hinge exactly 1 rad short of closed is toggled, a hinge 1.1 rad short is toggled, a fast shut hinge is toggled, and a latched shut hinge is toggled
- **THEN** the shut hinge aims at 0 for 100 ticks and wakes for 256, the hinge 1 rad short aims at 0, the hinge 1.1 rad short aims at 1, the fast hinge runs 60 ticks, and the latched hinge is unlatched and shoved 3 cm

### Requirement: Slider toggle
A slider's openness SHALL be 1 minus its travel. Above 0.7 it SHALL close for 60 ticks. At or below 0.7 it SHALL open for 60 ticks and clear the shut flag. A latched plain slider SHALL jump to 0.6 before that choice, so a shut one then opens for 60 ticks.

#### Scenario: An open slider closes, and a shut one opens
- **WHEN** a slider at travel 0 is toggled, a shut slider at travel 0.3 is toggled, and a latched plain slider at travel 1 is toggled
- **THEN** the open slider closes for 60 ticks and stays shut, the slider at 0.3 opens for 60 ticks and is no longer shut, and the latched slider is at 0.6 and opens for 60 ticks
