# Spec Delta

## MODIFIED Requirements

### Requirement: Locomotion request
The player SHALL move through a locomotion request (heading, look, flag word, moving, free walk, goal) that its body tick decides each tick: a free walk never brakes; toward a goal a walking body SHALL brake within its gait's stop distance (50/200/250/50 cm, blended by the run; the sprint row's only with the sprint latch, a stance level above 0.9 and over 3.5 cm a tick) and stop within 25 cm at 0.5 cm a tick or less; a running body never brakes. A body under its most energy SHALL not walk.

#### Scenario: Walking into a wall
- **WHEN** the player walks straight at a wall
- **THEN** the look-ahead ends its free walk and it brakes to a stop before the wall

#### Scenario: Dazed
- **WHEN** a blow leaves the player under its most energy while W is held
- **THEN** it stands until its energy is full again, then walks

#### Scenario: The sprint row's stop distance
- **WHEN** a body with the sprint latch moves at 3.6 cm a tick at a half run blend, on a stance level of 1, then of 0.9
- **THEN** its stop distance is 150 cm, then the jog's 125 cm
