## ADDED Requirements

### Requirement: Scene grid of the simulation
The level load SHALL give the simulation its grid chunk's width and height as the grid's columns and rows, with the origin cell half of each (unsigned shifts) plus the sector node's cell rounded half to even; the bodies and the static colliders SHALL be bucketed in that grid, a body by the flat index `z · columns + x`, and the nine cells around one SHALL be the flat offsets 0, −w−1, −w, −w+1, −1, 1, w−1, w, w+1.

#### Scenario: A 33 × 40 grid
- **WHEN** a scene of 33 × 40 cells has its sector node at (1500, 70, −1800) cm
- **THEN** its origin cell is (18, 17) and its neighbour offsets are 0, −34, −33, −32, −1, 1, 32, 33, 34

### Requirement: Step chosen by the scene
A scene with a grid SHALL take the grid step; a scene without one SHALL take the all-pairs step; with no scene given, the stand-in unbounded grid SHALL be used.

#### Scenario: The step takes the scene
- **WHEN** the step runs with a grid scene, then a scene without a grid, then none
- **THEN** the pair stage uses the scene's grid, then the all-pairs walk, then the unbounded cells

### Requirement: Position-overflow reset
After the iterations and the inverted-tetra repair, each awake part whose cell index lies in the scene grid's first row or at or past its last row SHALL be put back where the tick before last left it: its particles, last tick's particles and the tick's start set to the tick-before-last particles, its last-tick frame set to its frame. It SHALL be bucketed in cell `columns + 1` for that tick's detection, and the first overflow of a run SHALL be logged.

#### Scenario: A ball crossing into the last row
- **WHEN** a ball in a 4 × 8 grid moves 0.2 m a tick along z from 17.9 m
- **THEN** after the tick it stands where it stood two ticks ago with no velocity, linked in cell 5, and the next tick it falls from there in cell 26

### Requirement: Static grid shape
The static collider grid SHALL take the scene grid's shape and bucket its records again when the shape changes, keeping their ids; without a scene it SHALL take a 4096 × 4096 grid whose origin cell (2048, 2048) puts the world origin in its middle, so a collider wider than its cell spans cells.

#### Scenario: A long wall
- **WHEN** a wall 30 m long along x is inserted at the origin with no scene grid
- **THEN** a sim object 13 m along it finds it

### Requirement: Pair contact relative frame
The pair contact SHALL put a part's sphere in the other part's frame through the original's relative frame: each axis of the first frame and the offset between the origins dotted with the other frame's axes, summed x, y then z (the other frame's transpose as its inverse).

#### Scenario: A stretched frame
- **WHEN** a point at (3, 5, 0) is put in a frame whose x axis is (2, 0, 0) at (1, 0, 0)
- **THEN** it lands at (4, 5, 0), not at the inverse's (1, 5, 0)

### Requirement: Pair correction precision
The pair pass SHALL compare a point's depth with the 0.01 cm slop in double (a NaN skips), take the correction as the normal times the depth less the slop in double, A's travel as `(b_start − a_start) − (b − a)` and its part across the normal as `t + n·(−t·n)`, and the friction limit as `(0.99·n_y² + 0.01)·μ` in double; a slip whose square the limit reaches, or a NaN, SHALL take `limit · −0.01` of itself, any other the whole slip.

#### Scenario: A limit between the precisions
- **WHEN** a point with n_y 0.8 slides under a friction product of 0.37
- **THEN** the limit is 0.23813201 (not the single sum's 0.238132) and the correction takes the slip times −0.0023813201

### Requirement: Pair push product order
A pair point's push SHALL move each particle by the correction times the share multiplied by the weight clamped at zero (the processor's maximum, so a NaN weight is 0), and the pair friction's limit and its 0.01 scale SHALL be doubles.

#### Scenario: Share and weight first
- **WHEN** a correction of 3 along x is pushed with a share of 0.7 over a weight of 0.45
- **THEN** the particle moves 0.945, not 0.94499993

### Requirement: Level impact gates
A part's level impact SHALL need its body not to be out of the world, its smoothed travel squared over 0.1 and its strength over 0.1, each compared as a double (0.1 in single precision passes, a NaN fails).

#### Scenario: At the gate
- **WHEN** a part's smoothed travel squared is exactly 0.1 in single precision against a push straight back
- **THEN** it makes an impact of strength 0.1; a disabled body landing makes none

### Requirement: Bounds pass stage
The solver SHALL offer a bounds pass stage after the level contacts are dropped and every inverted part is repaired, before the sim objects, over the awake bodies.

#### Scenario: The overflow reset's place
- **WHEN** the pair stage resets an overflowing part in the bounds pass
- **THEN** the tick's detection sees the part where it was put back
