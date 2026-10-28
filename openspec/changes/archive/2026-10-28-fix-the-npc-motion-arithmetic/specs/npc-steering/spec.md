## ADDED Requirements

### Requirement: The walk driver's arithmetic
The walk SHALL scale the flat goal offset by its reciprocal length, use the facing at length 1 when the squared length is 1 cm² or less or NaN, form the short facing threshold as length·0.007 in double, take the near branch when the threshold is not below the facing dot (NaN included), and, with no foe looked at, zero the head offset's target before the offset eases 0.02 toward it.

#### Scenario: A NaN goal
- **WHEN** the step goal has a NaN coordinate
- **THEN** the walk moves without running or steering

### Requirement: The alert look-around's draws
With the body's in-combat mark set and a panic not under 0.2 (a NaN passes), the alert look SHALL step the generator for the wide pan (one in 300) and the narrow pan (one in 75) every tick before reading their timers; the wide pan spreads an f32 draw over ±π, the narrow one spreads the centred f32 draw times the double π. With a foe more than 45° (in double) from the look, the look SHALL take the foe's yaw, and the body turn SHALL run either way.

#### Scenario: The narrow draw is made with its timer pending
- **WHEN** the wide pan draws on a tick whose narrow timer has not passed, from a generator at 0
- **THEN** the generator ends three steps on, at 3698175007

### Requirement: The body in the way
The body-in-way test SHALL skip dead bodies, the body already remembered as in the way and the carried body, shift each other body 20 cm along the walk direction and return the first the steer line along that direction reaches.

#### Scenario: The remembered body is skipped
- **WHEN** bodies stand 100 cm and 150 cm ahead and the first is remembered
- **THEN** the second is returned

### Requirement: Look writes on leaving and settling
Leaving combat SHALL set the look, the heading and the eased offset to the body's look yaw and keep the head look, its target and the sway timer. The settle after a room walk SHALL set the look, the heading and the head look to the body's look yaw, keep the eased offset and draw the next sway 1–4 s out. The aim snap SHALL also set the body's look, look smoothing and heading directions.

#### Scenario: Leaving combat
- **WHEN** combat is left with the body's look yaw 0.75
- **THEN** the look, heading and offset are 0.75 and the head look and the sway timer are unchanged

### Requirement: The idle room walk
The room walk SHALL mark the current cell visited; the candidates are the exits not leading to the previous cell nor to an exhausted cell. With no unexcluded candidate and the current cell not exhausted it SHALL mark the cell exhausted when there is no candidate at all, clear the exclusion word and go back through the previous cell's exit. Otherwise it SHALL draw among every unvisited unexcluded exit (the previous cell's included), or among the candidates when none is unvisited.

#### Scenario: Only the way back is unvisited
- **WHEN** both new cells beside the current one are visited and the previous cell is not
- **THEN** the exit to the previous cell is drawn

### Requirement: The idle driver's walk and use states
The driver's walk state SHALL walk while the clock is not past its deadline; failing the exit test, the first avoided exit SHALL set the marker bit and later ones their exit's bit. The use state's timeout SHALL fire when the arrival time plus 10000 ms, summed without a wrap, is below the clock, even on the tick the use completes, and SHALL set the exit's own bit.

#### Scenario: The use completes as the timeout passes
- **WHEN** the use completes 10001 ms after the arrival with no exit avoided yet
- **THEN** the driver plans again and the exclusion word holds the exit's bit only

### Requirement: The investigation's gates
The investigation SHALL compare the gap to the kept value with 0.02 and the disturbance with 0.3 in double precision; the same cell within the gap SHALL only stamp a value over 0.5, and a changed cell or value over 0.3 SHALL be kept and walked to.

#### Scenario: The f32 0.3 walks
- **WHEN** a changed disturbance of the f32 0.3 is sampled
- **THEN** the investigation walks to it

### Requirement: The entering body's placement
A collapsed entering body SHALL keep its stored placement. Any other SHALL take its current point and keep its stored facing, or, while following a path, face its path goal from its reference point on the ground plane; a goal straight above or below keeps the stored facing.

#### Scenario: A walking body enters
- **WHEN** a body following a path to a goal 300 cm across and 400 cm along from its reference enters
- **THEN** it faces (0.6, 0, 0.8) at its current point
