# simulation-timing Specification

## Purpose
The simulation's per-tick environment conversion and its frame interpolation fraction.

## Requirements

### Requirement: Environment tick conversion
The system SHALL convert an environment to one tick by raising each per-second retention that is above zero to `1/ticks` and storing zero for any other retention, a NaN included, scaling the gravity vector by the reciprocal of `ticks` squared as a float multiply, and squaring each speed threshold after dividing it by the tick count.

#### Scenario: The inert environment
- **WHEN** the inert environment is converted at 83 ticks
- **THEN** every retention and the gravity are zero

#### Scenario: A NaN retention
- **WHEN** an environment whose fast retention is NaN, slow retention −1 and resting retention 0.05 is converted at 83 ticks
- **THEN** the retentions are 0, 0 and 0.96455044

### Requirement: Frame fraction
The system SHALL compute the frame interpolation fraction as `1 + (since - time)/step` when the time since the base exceeds the simulation's game time and `1 - (time - since)/step` otherwise, in double and rounded to a float, clamped to [-2, 2].

#### Scenario: A late frame
- **WHEN** the time since the base is 100 ms, the game time 0 and the step 12 ms
- **THEN** the fraction is 2.0

### Requirement: Scene sector creation
The simulation SHALL create one sector per sector node of a scene batch; a new sector that holds no static nodes yet SHALL hand its node's children back for immediate registration, and when the scene shares the first sector every node of the batch SHALL append its children to that sector's pending list without changing its scene word or node.

#### Scenario: A shared batch
- **WHEN** a shared batch of two sector nodes with children 30 and 31, 32 is added after a first sector exists
- **THEN** no new sector is created, the first sector keeps its scene and node, and its pending list is 30, 31, 32

### Requirement: Step driver due test
The simulation's step driver SHALL count time in wrapping 32-bit milliseconds from a base and SHALL find the next step due once the time since the base, compared unsigned, reaches the simulation's time plus one step. Each step taken SHALL add one to the tick count and one step to the simulation's time.

#### Scenario: Back to back
- **WHEN** a 12 ms driver started at 0 runs every step due at 59 ms
- **THEN** four steps run and the simulation's time is 48 ms

### Requirement: Step driver backlog
When a due step is more than four steps late and the simulation does not keep its backlog, the step driver SHALL move the base on by the lateness so that the step ends now.

#### Scenario: Four steps late
- **WHEN** a 12 ms driver started at 0 polls at 60 ms
- **THEN** a step is due and the base stays at 0

#### Scenario: More than four steps late
- **WHEN** a 12 ms driver started at 0 polls at 61 ms
- **THEN** a step is due, the base moves to 49 ms, and after that step the driver waits 11 ms

#### Scenario: A pause dropped
- **WHEN** a 12 ms driver that has taken one step is polled again a second later
- **THEN** 988 ms are dropped and one step is due

### Requirement: Step driver wait
When no step is due the step driver SHALL wait one millisecond less than the time left to the next step, at most 100 ms.

#### Scenario: Just started
- **WHEN** a 12 ms driver started at 1000 ms polls at 1000 ms
- **THEN** it waits 11 ms

### Requirement: Fixed step boundary map
The game SHALL, when given `--sim-boundary <file>`, write, once the loading screen is gone and the fixed step has run 200 more ticks, every component and resource type that a system of the fixed step's schedules and a system of the frame's schedules both access, with its direction (written by the fixed step only, by the frame only, by both, or read by both) and each side's reading and writing systems, and SHALL list each side's systems that take the whole world.

#### Scenario: A body read by the frame
- **WHEN** a fixed-step system writes a component that an `Update` system only reads
- **THEN** the map lists the component as going from the simulation to the frame

### Requirement: Simulation world in lockstep
The game SHALL keep the simulation's own world, run on a simulation thread or, when asked, on the frame's thread, and SHALL run exactly one step of it at the start of every pass of the fixed step, waiting for it, so that the fixed step's results are the same bit for bit with or without it.

#### Scenario: Three passes
- **WHEN** the fixed step runs three passes
- **THEN** the simulation's world has stepped three times and its last snapshot is tick 3

#### Scenario: A body unchanged
- **WHEN** a shoved, muscle-driven body runs 480 ticks with no simulation world, with one inline, and with one on its thread
- **THEN** every part's pose after every frame is the same bit for bit in the three runs

### Requirement: Commands to the simulation
Commands the frame posts for the simulation SHALL run on the simulation's world at the start of the next step, before its schedule, in the order they were posted, each once.

#### Scenario: Three commands
- **WHEN** commands A, B and C are posted before a step
- **THEN** they run in that order before that step's schedule, and not again

### Requirement: Events to the frame
Events the simulation sends during a step SHALL run on the frame's world after that step, in the order they were sent, each once.

#### Scenario: Two steps
- **WHEN** a step sends events A and B
- **THEN** the frame runs A then B after that step, and neither after the next

### Requirement: Simulation snapshots
After each step the simulation SHALL publish a snapshot of its tick and game time into a slot that keeps only the newest one, and the frame SHALL keep the last two it took.

#### Scenario: Two published before a take
- **WHEN** snapshots 1 and 2 are published before the frame takes one
- **THEN** the frame takes 2, and nothing more until the next is published

### Requirement: Objects named in both worlds
An object the frame hands to the simulation SHALL be named by an id issued in order and linked to its entity in each world; removing it SHALL unlink it in both.

#### Scenario: Spawned and removed
- **WHEN** the frame hands over an object with a mass of 70 and then removes it
- **THEN** after the first step the simulation's entity for that id has a mass of 70, and after the second neither world links the id

### Requirement: Fixed step behind the bridge
When the bridge is on, the game SHALL run the fixed step's schedules, with their order and sets, in the simulation's world in lockstep, and SHALL copy the data they touch between the worlds before and after each step, so that the fixed step's results are the same as with the schedules in the frame's world.

#### Scenario: A body unchanged
- **WHEN** a shoved, muscle-driven body runs 480 ticks with the fixed step in the frame's world, and with it behind the bridge inline and on the simulation's thread
- **THEN** every part's pose after every frame is the same bit for bit in the three runs

#### Scenario: A character dies in its posture stage
- **WHEN** a collapsing body whose tone falls to a quarter is ticked three times through the fixed step, in one world and behind the bridge
- **THEN** in every run it is dead and limp after its posture stage and only the other body runs the later stages

### Requirement: Change detection across the bridge
A change, addition, removal, spawn, despawn or message made on one side SHALL be seen once by the other side's systems at the same point of the step order as in one world, and SHALL NOT be sent back.

#### Scenario: A scripted run
- **WHEN** the frame moves, adds, removes and despawns components and sends messages between steps while the fixed step moves bodies, spawns and despawns entities and answers messages
- **THEN** the fixed step's and the frame's logs of what changed are the same as in one world, and a value nobody changed is not copied

### Requirement: Shared entity ids
An entity SHALL have the same id in both worlds, whichever world spawned it.

#### Scenario: Spawned by the simulation
- **WHEN** the fixed step spawns entities behind the bridge
- **THEN** the frame's world has them at the same ids

### Requirement: Bridge coverage
When the bridge starts it SHALL list every data type the moved systems touch that is not bridged, and the boundary map SHALL list the same types.

#### Scenario: Fully registered groups
- **WHEN** the bodies' or the character tick's tests start the bridge
- **THEN** the list is empty

### Requirement: Whole fixed step bridged
With the bridge on, every data type the moved fixed step's systems and run conditions touch SHALL be bridged, and no moved system SHALL be skipped for missing data.

#### Scenario: Three levels
- **WHEN** the arena duel, exanima01 and exanima06 run with the bridge
- **THEN** the bridge reports no unbridged type and no system skipped

### Requirement: Copy cost reported
The bridge SHALL report the time its copies take per step and the costliest types on each side.

#### Scenario: A level running
- **WHEN** a level runs 1000 steps with the bridge
- **THEN** the log gives the copy time per step and each side's costliest types

### Requirement: Bridged fixed step by default
The game SHALL run its fixed step in the simulation's world behind the bridge unless asked not to, and the bridged run SHALL follow the one-world run within the run-to-run variation the one-world run has with itself.

#### Scenario: The arena duel
- **WHEN** the arena duel runs 900 fixed frames with joint traces, in one world twice and behind the bridge
- **THEN** over the first 100 ticks the bridged trace differs from the first one-world trace no more than the second one-world trace does

### Requirement: Query filters bridged
Every type a moved system's query filters name SHALL be bridged, and the coverage SHALL list any that is not.

#### Scenario: A marker filter
- **WHEN** a moved system's query asks for entities with a marker component
- **THEN** the marker is bridged, so the query matches the same entities in the simulation's world

### Requirement: Step digest
On request the game SHALL write, after every fixed step, a hash of every digestible bridged type's values that does not depend on entity ids, in one world and behind the bridge alike.

#### Scenario: Two one-world runs
- **WHEN** two runs write the digest with fixed frames and game time held while loading
- **THEN** the types without hash maps or frame-dependent state hash alike step by step

### Requirement: Unregistered types stop debug builds
When a moved fixed-step system touches a type no plugin registered with the bridge, a build with debug assertions SHALL stop at the first step naming the types, and a play build SHALL warn and skip the system.

#### Scenario: A new marker
- **WHEN** a test app moves a system that filters on a component nobody registered
- **THEN** the first fixed step stops with the component's name

### Requirement: Once-a-frame simulation work
Work the fixed step does once at the start of each frame SHALL run in the simulation's world behind the bridge, at the first step after a frame began, and give the same results as before the frame's fixed loop in one world.

#### Scenario: Tick counts between frames
- **WHEN** bodies tick several times in a frame
- **THEN** each body's poses match the one-world run step for step

### Requirement: Cloth mesh snapshots
The frame SHALL take each cloth's mesh positions from the steps' snapshots, the newest per mesh, without reading the cloth itself.

#### Scenario: Four ticks a frame
- **WHEN** a cloth steps four times in a frame
- **THEN** its mesh takes the positions of the fourth step, in metres

### Requirement: Frame writes as fixed-step commands
A frame system that changes simulation data SHALL be able to post the change, and every posted change SHALL be applied at the start of the next fixed step, in post order, on the world that runs the fixed step, alike with and without the bridge.

#### Scenario: A posted write
- **WHEN** the frame posts a write to an object's position between two steps
- **THEN** the next step's systems see the new position, and the logs of what changed match between one world and the bridge

#### Scenario: Going limp
- **WHEN** a body is asked to go limp in a frame
- **THEN** its muscles go at the start of the next fixed step, before that step's systems run, with and without the bridge

### Requirement: Rise in the simulation
A dead character due to rise SHALL be revived, and its clips and controller given back, in the simulation's once-a-frame work before the frame's ticks, alike with and without the bridge.

#### Scenario: A corpse past its clock
- **WHEN** a dead body with the rise rule and enough health passes its revival time
- **THEN** after the next frame's ticks it is down, has its muscles, clips and controller back, and has no death clock, in one world and behind the bridge alike

### Requirement: Hit chain in the simulation
The hit chain SHALL run every simulation step, in the step's post-step, from the strikers' and the targets' frames where that step left their bodies, followed by the systems that react to its results; the blows it sends SHALL be dealt first in the next step, alike with and without the bridge.

#### Scenario: A scripted walk
- **WHEN** the arena runs a scripted walk with fixed frames, in one world and behind the bridge
- **THEN** every joint of every traced tick is the same in both runs

#### Scenario: A duel
- **WHEN** the arena duel runs with fixed frames in one world and behind the bridge
- **THEN** the same hits land at the same ticks with the same damage, and every joint of every traced tick is the same

#### Scenario: A blow in a frame without a tick
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it is dealt by the next tick

### Requirement: Scripted grid runs repeat
A grid quadrant running with fixed frames SHALL ignore the host's forwarded input, so that two runs of the same scripted scene repeat whatever the host's mouse does.

#### Scenario: Four duels in one grid
- **WHEN** the arena duel runs twice in one world and twice behind the bridge in one grid, with fixed frames
- **THEN** over their first 100 traced ticks the joint angles differ by no more than a hundredth of a degree on average

### Requirement: Alarm zones in the simulation
The golem alarm's zone check SHALL run in the simulation's once-a-frame work after the rise and its restore, alike with and without the bridge.

#### Scenario: The player crosses an alarm zone
- **WHEN** the player, without the zone's pass, moves into an alarm zone
- **THEN** the alarm rings at the same tick with and without the bridge

### Requirement: Bodies as snapshots
Behind the bridge the frame SHALL read bodies and the mechanisms on them as snapshots of the simulation, and its writes to them SHALL reach the simulation as posted writes at the start of the next step, before the step runs, alike with and without the bridge.

#### Scenario: A scripted corpse drag
- **WHEN** the player drags a corpse with a scripted press and pointer move, with fixed frames and the load held, once in one world and once behind the bridge
- **THEN** the dragged grab point is the same every frame and the corpse's joint angles do not differ

#### Scenario: A frame write that is not posted
- **WHEN** a frame system changes a body behind the bridge without posting the change
- **THEN** the change does not reach the simulation, the next copy out replaces it, and the bridge warns once

### Requirement: No fixed step behind the loading screen
While the loading screen shows, the fixed step SHALL not run and the time the frames bring SHALL not be kept for it, so the first tick after a load comes with the first frame after it, however many frames the load took.

#### Scenario: Loads of different lengths
- **WHEN** one run's load takes 3 frames and another's 47, and both then run 20 frames
- **THEN** no tick runs during either load and both runs tick alike after it

### Requirement: Static colliders in hand-out order
A level's static colliders SHALL join the physics world in the order their builds were handed out, whatever order the builds finish in, and a frame SHALL hand out builds by a triangle budget, not by its clock.

#### Scenario: Two scripted runs
- **WHEN** a scripted scene (exanima01's walk, exanima06's W, D and S script, the arena duel) runs twice with `--fixed-frames`
- **THEN** both runs' body traces agree from their first tick

### Requirement: Hierarchy copied for the simulation's entities only
Behind the bridge the frame SHALL send the hierarchy and transforms only of the entities the simulation holds and their ancestors, and the simulation SHALL step alike with and without the bridge.

#### Scenario: A walk in a story level
- **WHEN** the player walks in exanima01 or exanima06 with fixed frames, once in one world and once behind the bridge
- **THEN** the player's joint angles do not differ, and the frame's meshes and bones are not copied to the simulation

### Requirement: The manager frame before every step
The rise, the restore and the golem alarm's zone check SHALL run before every simulation step, alike with and without the bridge.

#### Scenario: An undead opponent rises
- **WHEN** a debug duellist with the story's body rules dies in the arena, once in one world and once behind the bridge
- **THEN** it rises at the same game time in both, on a step boundary

### Requirement: One-way resources across the bridge
A resource that only the frame writes SHALL cross to the simulation only, and one that only the simulation writes SHALL cross to the frame only (and in when the frame inserts it); a write on the other side SHALL not cross, alike inline and on the simulation's thread.

#### Scenario: A rogue write
- **WHEN** the simulation writes a resource the frame owns, or the frame writes one the simulation owns
- **THEN** the other side never sees that write, and without such writes both sides see what one world sees

### Requirement: Step writes to frame data
A fixed-step system that changes data the frame owns SHALL see its own write at once and SHALL have the same write applied on the frame's copy after the step, before the step's other events, alike in one world, inline and on the simulation's thread.

#### Scenario: A step appends to frame data
- **WHEN** the frame and each fixed step append to the same frame-owned list
- **THEN** both sides see the same list after every frame as one world does

### Requirement: Frame work from step results
Work only the frame does with a step's results (sounds the step decided on) SHALL run on the frame's world after that step, in the order the step queued it, alike in one world and behind the bridge.

#### Scenario: A prop strikes the floor
- **WHEN** a plain body's part rings an impact in a step
- **THEN** the frame plays its set's sound after that step, at the part's mesh centre, in one world and behind the bridge alike

### Requirement: Scripts and sounds on the frame
Sounds a fixed step starts and the scripts an AI's target pass reports SHALL be run by the frame after the step, in the step's order, with the step keeping only what it decides (which sound, which scripts); a voice that a later step fades SHALL be found by a key the step gave it.

#### Scenario: A throw that ends before its drop
- **WHEN** a throw's lift sound starts and the throw ends before its drop
- **THEN** the frame fades the lift's voice found by the key the step gave it

### Requirement: Collision replica for frame picks
The frame's picks SHALL query a replica of the simulation's collision world that has the same collider handles and gives the same pick results as the simulation's world after the last finished step, through the frame's inserts and removals and the step's moves and removals.

#### Scenario: Picks after moves and removals
- **WHEN** the frame inserts and removes colliders and a step moves and removes others
- **THEN** every pick on the replica after the step returns the same collider and distance as on the simulation's world

### Requirement: Simulation-only data stays in the simulation
Data only the simulation's systems touch SHALL move to the simulation's world once and SHALL not be handed back to the frame between steps; a frame that inserts it again (a level load) SHALL replace the simulation's, and one that removes it SHALL have it removed there before its other changes apply.

#### Scenario: A level change
- **WHEN** the frame removes the moving meshes' record and later inserts a new one
- **THEN** the simulation drops its record before the frame's other changes of that packet and takes the new one, with no copy back to the frame

### Requirement: Free-running simulation
With the free mode on, the simulation SHALL step on its own 12 ms real clock, dropping its backlog when more than four steps late, SHALL not step while the frame holds it (the loading screen, a menu, the splash, the character creator, a paused game clock; not the end of a session's menu, behind which the world goes on) and SHALL leave the held time out of its clock.

#### Scenario: Held and released
- **WHEN** the frame holds the simulation for a while and then releases it
- **THEN** no step runs while held, steps follow at about 83 a second after the release with consecutive ticks, and the published time does not count the held time

### Requirement: Packets between the free simulation and the frame
With the free mode on, the simulation SHALL take every packet the frame sent since its last step, in order, before each step, and SHALL send after each step a packet stamped with the step's tick and time; the frame SHALL not wait for a step and SHALL apply every step packet that arrived, in order.

#### Scenario: The frame's calls
- **WHEN** the frame posts calls over several frames
- **THEN** the simulation runs them in post order before its next steps, and their answers reach the frame in the same order

### Requirement: Drawing at the original's render fraction
With the free mode on, the frame SHALL draw the simulated bodies at 1 + (frame clock − the last step's published time) / step, at most 2 and at least −2, as the moment one step behind the clock: 1 the step before the last, 2 the last, a fraction below 1 held at the step before the last; and SHALL keep the fraction while the simulation is held.

#### Scenario: A frame half a step after the last step
- **WHEN** a frame is drawn 6 ms after the last published step's time
- **THEN** the bodies are drawn at fraction 1.5, halfway between the last two steps

### Requirement: Drawn copies keep the simulation's newer value
With the free mode on, a value the frame writes only to draw it (transforms, the drawn controller) SHALL not replace a value the simulation wrote in a step the frame had not seen when it wrote; other values both sides write SHALL take the frame's write, as in lockstep. In lockstep every step SHALL stay identical to the frame-driven step.

#### Scenario: The drawn controller
- **WHEN** the frame draws a character's controller and the simulation has stepped it since
- **THEN** the simulation keeps its own controller and position

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

### Requirement: Free simulation by default
The game SHALL run the simulation free of the frame unless lockstep is asked for, a fixed-frame scripted run is made, the simulation runs on the frame's thread, or the step stays in the frame's world; in lockstep every body's every tick SHALL stay the same as before.

#### Scenario: A scripted duel
- **WHEN** a fixed-frame scripted duel runs with the default options
- **THEN** it steps in lockstep and every body's trace is the same as the earlier build's

### Requirement: No step after a teardown
With the free mode on, a level's teardown SHALL first hold the simulation and apply every step it took before the hold, and SHALL keep it held until the next level is in; no step value SHALL despawn or replace an entity or resource the frame made after the step began.

#### Scenario: A teardown while the simulation steps
- **WHEN** the simulation steps entities and the frame tears them down and spawns new ones at the same indices
- **THEN** the new entities and resources survive, the old ones stay gone, and the steps go on without a gap after the release

### Requirement: Removals survive values in flight
With the free mode on, a component one side removes SHALL stay removed when the other side sends a value for it that it wrote before it had seen the removal.

#### Scenario: A dead body's clip player
- **WHEN** the frame removes a dying body's clip player while the simulation's steps in flight still carry it
- **THEN** the clip player does not come back on either side, and the body lies as in lockstep

### Requirement: Engine floating-point mode
The main thread, the simulation thread and every task-pool thread SHALL set the SSE control register's flush-to-zero and denormals-are-zero bits and the exception masks (`0x8FC0` ORed in) before running anything, leaving the rounding mode at round-to-nearest.

#### Scenario: A denormal on the simulation thread
- **WHEN** the smallest normal float is halved on a thread in the mode
- **THEN** the result is zero

### Requirement: Lockstep runs visit entities in one order
In lockstep, the simulation SHALL apply each bridged component's values and removals from the frame in entity order, so that its queries visit entities in the same order in every run of a scene, whatever order the frame's own archetypes give them.

#### Scenario: Four identical scenes
- **WHEN** four identical `--fixed-frames --trace-all` scenes of exanima01 run in one grid
- **THEN** their traces are byte-identical, rows of each tick in the same order

#### Scenario: Frame-only markers
- **WHEN** some of the frame's entities carry a component the bridge does not carry, putting them after the others in the frame's queries, and then each gets a bridged component in the same frame
- **THEN** the simulation's queries visit them in the same order as without the marker

### Requirement: Lockstep runs number their entities alike
Two `--fixed-frames` runs of one build and scene SHALL give the simulated bodies the same entity ids. A resource is an entity spawned when first inserted, so the frame's world SHALL spawn an empty entity for every registered resource type at one fixed moment (the start of its second update), and the simulation's world one for each bridged resource when the bridge starts; a resource first inserted later (when a load finishes, or in whichever step its value crosses) lands on that entity.

#### Scenario: The arena duel
- **WHEN** four identical `play arena --opponent --press D@200-260,W@400-460,S@600-700 --frames 3000 --fixed-frames --trace-all` scenes run
- **THEN** their traces are byte-identical, the `body` column included

### Requirement: Single-threaded startup under fixed frames
Under `--fixed-frames` the startup schedules SHALL run their systems one at a time, since a spawn takes its id when it is queued.

#### Scenario: A resource inserted after a spawn
- **WHEN** one world inserts a held resource before spawning an entity and another inserts it after
- **THEN** the spawned entity has the same id in both

### Requirement: Lockstep runs dress their characters alike
Under `--fixed-frames` the characters SHALL be dressed in the frame their parts are attached, every one of them, in spawn order (entity index, ascending), with their garments' fits and paints built in that frame, so the garments, the held items and those items' bodies take the same entity ids in every run. Every bridged resource type SHALL be registered in the frame's world when it is bridged, so the frame's resource hold covers it.

#### Scenario: The armed duel
- **WHEN** four identical `play arena --opponent --weapon ub05ssworda --frames 1500 --fixed-frames --trace-all` scenes run, two to a grid
- **THEN** their traces are byte-identical, the `body` column included, and every run reads the first clash alike

#### Scenario: A resource only the fixed step names
- **WHEN** a resource is bridged and the frame's world holds its resources
- **THEN** its first insert in the frame's world takes no new entity id
