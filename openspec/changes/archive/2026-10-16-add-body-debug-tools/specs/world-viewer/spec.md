## ADDED Requirements

### Requirement: Body overlay
`--debug-bodies`, or F3 in any view, SHALL draw every simulated body over its mesh between its last two ticks: each part's particles and tetrahedron; each link from its point on the part to its point on the parent, green within its slack (1 mm tolerance), yellow up to 1 cm past it, red beyond; pivots as crosses; a living body's muscle anchors, faint. F4 SHALL draw only the inspected body, and F5 SHALL inspect the body under the cursor.

#### Scenario: Standing duel
- **WHEN** `exarust --debug-bodies --opponent play arena` runs and both fighters stand
- **THEN** both are drawn over their meshes and their links are green

#### Scenario: Limp body
- **WHEN** `exarust --debug-bodies --limp character` runs
- **THEN** the falling body's parts and links are drawn on it as it falls and once it sleeps

#### Scenario: Off
- **WHEN** neither the flag nor F3 turned the overlay on
- **THEN** nothing is drawn and no measure is taken

### Requirement: Joint readout
With the overlay, the window SHALL show, for the inspected body (the player's unless another is picked), each joint's turn from its rest, its swing and twist about the bone axis, its distance from the clip's turn, the share of its range its slack links use, its pivot's gap and its worst link past its slack, each row coloured as that link; and the body's sleep state, support, balance, energy and tone, and the simulation's tick, speed and pause.

#### Scenario: A blow opens an elbow
- **WHEN** a hit makes the player's elbow pivot open more than 1 cm
- **THEN** the forearm's row is red and reads how far the elbow is from the clip's turn

### Requirement: Simulation controls
P SHALL pause and resume the fixed step (character controllers, character ticks and the body solver), `.` SHALL run exactly one tick while paused, and `[` and `]` SHALL step its speed through ×0.1, ×0.25, ×0.5, ×1 and ×2; `--sim-speed` SHALL set the starting speed. While paused or slowed, rendering SHALL go on and the cameras SHALL move on real time.

#### Scenario: One step
- **WHEN** the simulation is paused at tick 300 and `.` is pressed three times
- **THEN** ticks 301, 302 and 303 run and the simulation stays paused at 303

#### Scenario: Orbit while paused
- **WHEN** the simulation is paused in play mode and the middle mouse button drags
- **THEN** the camera orbits the frozen bodies

### Requirement: Scripted pauses
`--pause-at-tick <n>` SHALL pause the simulation at the end of body tick n, and `--pause-on-stretch <cm>` at the end of the tick in which a body's link first goes that far past its slack, logging the body, part and link. With either, `--screenshot` SHALL count its frames from the pause.

#### Scenario: Freeze a fold
- **WHEN** `exarust --debug-bodies --opponent --pause-on-stretch 1 --screenshot s.png --frames 30 play arena` runs
- **THEN** `s.png` shows the duel stopped at the tick a link first opened 1 cm past its slack, with that part's row red

### Requirement: Scripted input and camera
`--press key@frame,...` SHALL press each key at its frame counted from the end of the load and release it the frame after. In play mode, `--camera x,y,z,yaw,pitch` SHALL place a fixed camera there instead of the chase camera.

#### Scenario: Scripted steps
- **WHEN** `exarust --pause-at-tick 300 --press .@500,.@510,.@520 character` runs
- **THEN** the simulation stops at tick 303

#### Scenario: Side view of a duel
- **WHEN** `exarust --opponent --camera 5.5,1.3,6.3,90,-6 play arena` runs
- **THEN** the camera stays at that pose, looking at the duel from its side
