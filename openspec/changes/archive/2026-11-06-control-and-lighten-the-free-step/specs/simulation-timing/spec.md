## ADDED Requirements

### Requirement: Debug controls of the free simulation
With the free mode on, a held simulation SHALL take each single step the frame asks for, moving its clock on by a step, and a game-clock speed the frame sets SHALL scale the simulation's clock and the frame's render fraction alike; a scripted fixed-frame run SHALL keep lockstep.

#### Scenario: Three single steps, then a quarter speed
- **WHEN** the frame holds the simulation, asks for three single steps, then releases it at a quarter speed
- **THEN** exactly three steps arrive while held, and afterwards steps come about every 48 ms

### Requirement: The step on its own thread
The fixed step's systems SHALL run one at a time on the simulation's own thread, not on the task pool the frame shares, and every body's every tick SHALL stay the same as when they ran on the pool.

#### Scenario: A scripted walk
- **WHEN** a scripted fixed-frame walk runs with the step on its own thread and on the pool
- **THEN** every body's trace is the same row for row
