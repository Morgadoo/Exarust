## ADDED Requirements

### Requirement: Unregistered types stop debug builds
When a moved fixed-step system touches a type no plugin registered with the bridge, a build with debug assertions SHALL stop at the first step naming the types, and a play build SHALL warn and skip the system.

#### Scenario: A new marker
- **WHEN** a test app moves a system that filters on a component nobody registered
- **THEN** the first fixed step stops with the component's name

### Requirement: Once-a-frame simulation work
Work the fixed step does once at the start of each frame SHALL run in the simulation's world behind the bridge, at the first step after a frame began, and give the same results as before the frame's fixed loop in one world.

#### Scenario: Blows between frames
- **WHEN** blows are sent between frames while bodies tick
- **THEN** each body's poses match the one-world run step for step

### Requirement: Cloth mesh snapshots
The frame SHALL take each cloth's mesh positions from the steps' snapshots, the newest per mesh, without reading the cloth itself.

#### Scenario: Four ticks a frame
- **WHEN** a cloth steps four times in a frame
- **THEN** its mesh takes the positions of the fourth step, in metres
