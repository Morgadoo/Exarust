# `exa_physics`

Rigid-body and articulated-body simulation: the core of Exanima's feel (physically driven characters, momentum-based combat, cloth).

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started — the original's
body solver (`body`), its level contacts (`contact`), props and items as bodies (`prop`), a cloth step (`cloth`), the
character's place (`lib`, `CharacterBody`), the dynamics world unit (`dynamics`), the rigid-body shape
builders (`shape`), the shared intersection maths (`intersect`), the engine's own maths unit (`engine_math`) and the weapon part tree's mass redistribution (`weapon_mass`).

`weapon_mass` redistributes a weapon's part tree in two passes, parent before children. A saved mass of zero is latched from the live mass. The live mass becomes 1.25 times that saved mass and the inverse word its reciprocal. Both words then become the reciprocal of (0.5/count + (inverse/sum)·0.5) · scale. A second pass inflates the saved mass again. The equip pass calls it over a held weapon model's rigid nodes, with the record's table mass as the scale, divides the root part's mass word by the holder's size, and keeps the distributed parts as `WeaponMass` for the solver (`exa_character::weapon_mass_of`); no solver stage reads it yet.

`step_driver` is the original's simulation step clock, without a thread:
wrapping 32-bit milliseconds from a base, a step due once the time since the
base reaches the next step's end, the backlog dropped (the base moved so the
step ends now) when a step is more than four steps late, and otherwise a wait
one millisecond short of the next step, at most 100 ms. The frame-coupled
fixed loop does not use it yet: it is the clock of the simulation thread the
fixed step is moving to (design: OpenSpec `plan-the-simulation-thread`).

`float_mode` is the engine's floating-point mode: `enter` ORs `0x8FC0` into
MXCSR (flush-to-zero, denormals-are-zero, exception masks; rounding left at
nearest), as every thread of the original does first. The binary's main
thread, both `exa-sim` thread kinds (`enter_logged`) and every task-pool
thread (`task_pool_options`) enter it. The original's simulation thread is
time-critical (priority 15); Exarust's keeps the default priority (raising
it needs privileges on Linux, and the step's own clock paces it).

`sim_app` is the simulation's own world, the shell the fixed step moves
into: `SimShell` owns a Bevy `SubApp` with the `SimStep` schedule, run on a
thread named `exa-sim` (or inline, `--sim-inline`). `SimInbox` queues the
frame's commands, run on the simulation's world at the start of the next
step in post order (the original's deferred-call ring); `SimOutbox` queues
the simulation's events, run on the frame's world after the step in send
order; a latest-wins `SnapshotBuffer` hands the frame each step's
`SimSnapshot`, of which `Snapshots` keeps the last two; `SimId`s, issued by
`SimIds`, name an object in both worlds through each world's `SimLinks`.
For now it runs in lockstep: the `SimLockstep` schedule, before `FixedFirst`
in every pass of `FixedMain`, runs exactly one simulation step and waits
for it, and `LockstepCheck` counts the steps against the snapshots' ticks
(logged every 1000 steps). Without the bridge the simulation's world holds
only its clock, so a body's every pose is the same bit for bit with or
without the shell (`body_floor_test`). `EXA_ORPHAN_TRACE=1` (environment) logs, on either world, a `ChildOf` inserted with a parent that world does not hold (`sim_app::trace_orphans`).

With `SimMode::Free` (the game's default; `--sim-lockstep`, `--fixed-frames`,
`--sim-inline` and `--no-sim-bridge` step in lockstep) the
simulation runs on its own thread as the original's does
(context/re/sim-thread-input.md): a 12 ms real-clock step from
`step_driver` (more than four steps late drops the backlog), asleep 11 ms a
pass while the frame's packets say it is held (`SimPause`: the loading
screen, the menus, the splash, the creator, a paused game
clock; the paused time stays out of its clock), the frame's packets
drained before each step (its changes, its writes and posted calls, Bevy's
messages updated at most once a step) and a packet stamped with the step's
tick and time (`FreeStamp`) sent after it. Its `Time` and `Time<Fixed>`
are its own, advanced a timestep a step. The frame's fixed loop does not
run: once a frame (`SimExchange`, where the loop ran) the frame sends its
packet without waiting and applies every step packet that arrived, in
order, with its events; held, it keeps its changes and posts until it lets
go. The original's render fraction, 1 + (frame clock − published time) /
step clamped to [−2, 2], goes into the frame's `Time<Fixed>` as its
overstep: the original draws between the two older of its three history
entries at that fraction, one step behind the clock; Exarust's bodies keep
the last two, so the same moment is fraction − 1 between them (0 to 1,
`overstep_of`), and every drawing that reads `overstep_fraction` draws it,
as the lockstep frame draws one tick behind. Data both sides write meets a frame write made
against an older step: a drawn copy (`bridge_copy_drawn`: transforms, the
drawn controller) keeps the simulation's newer value; any other keeps the
frame's, as in lockstep; both are counted in the log every 1000 steps
("frame values after a newer simulation write"). A lent resource moves to
the simulation for good in free mode; `--fixed-frames` keeps lockstep. The
pause key holds a free simulation, the step key takes a single step while
it is held and the speed keys scale its clock (`SimPause`; Exarust's debug
controls, the original's loop reads the real clock unscaled). The moved
schedules run on the `SingleThreadedExecutor` on the simulation's own
thread, as the original's step runs on its thread: on the task pool the
frame and the renderer share, a step waited for their work (exanima01
29.5 ms a step on the pool, 18.7 ms on its own, in a busy grid; every
body's every tick the same in lockstep). The body grid
(`sector_lists::SectorBodies`) keeps the cells of bodies that did not
change since its last rebuild instead of taking every sleeper's pose each
step. `EXA_SIM_TIMING=1` logs each schedule's time a step.
Entering a hold, the frame waits for the simulation's answer and applies
the steps taken before it, so no step arrives after the frame went on;
a level's teardown holds the simulation first (`hold_and_drain`, the
original's level change raising the pause counter) until the next level
is in, and the frame never despawns an entity of its own (a newer one at
the index, a resource) for a value a step sent; a value a side sent before it had
seen the other side's removal of that component does not put it back (a
dead body's clip player, removed by the frame's go-limp, came back from
steps in flight and stood the corpse up); and a frame value the frame only
touched (still as the simulation last sent it) is not sent back; the lent types only the simulation
touches are moved in every mode (`bridge_move_resource`), and the alarm's
zones and state are copied in and a snapshot (the frame reads them).

`sim_bridge` is the lockstep copy bridge (stage 3): `SimBridgePlugin` moves
the fixed step's schedules (`FixedMainScheduleOrder`'s, plus the ones it
names, such as the character tick and the brain chain) whole, with their
sets and order, from the frame's world into the simulation's, on the first
lockstep step. Each type their systems, their run conditions and their query filters
(`With`, `Without`: a marker the bridge does not carry would match nothing)
touch is
registered on the app by the plugin whose systems touch it
(`SimBridgeAppExt`: `bridge_copy` for a cloneable component,
`bridge_relationship` for a relationship such as `ChildOf`, whose target
crosses too and whose `Children` order follows, `bridge_lend` for one moved
across for the step and back, the same for resources (a lent resource swaps
with a kept spare), `bridge_message` for a message type forwarded both ways,
`bridge_move_resource` for one the frame opens and only the step uses (the
trace files), `bridge_sim_only` for data only the step touches). The frame's
world never holds a moved or simulation-only type (nor a lent resource while
the simulation runs free), so a frame system reading one sees `None`: the
bridge's audit (`sim_bridge::audit`, run in the frame's `First` and `Last`)
checks every frame schedule as it is built for systems whose access names
such a type, logs the list of those types and each reader as an error, and
stops a debug build unless the reader was allowed (`allow_frame_read`, with
a reason). The start-up schedules, which ran before the bridge started, are
skipped; exclusive systems and run conditions are not seen. Work on such data
is posted to the next fixed step (`FixedCommands::run`), and data the frame
reports is a snapshot of the step's. Every step
the frame sends what changed in its world since the last step (values with
a newer change tick, removals, despawns, messages), the simulation applies
it, runs, and sends its own changes back the same way; each side's own
writes from an apply are not sent back, so `Changed`, `Added` and
`RemovedComponents` see on each side what they would see in one world.
The simulation applies each type's values and removals in entity order
(by index: the frame's spawn order, mostly what one world would give), not
in the order the frame's queries found them:
that order moves the entities between the simulation's archetypes, and its
queries (a system's commands, the trace's rows) visit them in it, while the
frame's own archetypes hang on load timing; without it two lockstep runs of
exanima01 visited the spawn watch's bodies in two orders.
Entity ids are the same in both worlds: the frame's allocator hands out every
id, the simulation spawns its own entities from ids the frame reserved for
it, and a despawn is mirrored without freeing the id. A resource is an
entity too, spawned when it is first inserted: `resource_hold`
(`ResourceHoldPlugin`, which `SimBridgePlugin` adds) spawns an empty entity
for every registered resource type at the start of the frame's second update
(the simulation's world holds its bridged resources' when the bridge
starts), so a resource first inserted when some load finishes (the
navigation grid's async build, the low field once the camera has a size)
lands on its held entity instead of taking an id among the characters'
(the arena duel's body ids took two values across runs). Bridging a resource
registers its type in the frame's world (`BridgeHalf::register`), so the hold
covers resources only the fixed step names (the navigation movers). When the bridge starts
it builds the moved schedules and lists every type their systems or run
conditions touch that is not bridged (`BridgeCoverage`, logged; while one
is missing, a failing system is skipped with one warning); after each step
it warns once for any component the simulation's entities carry that is not
bridged (one a system inserts by command). Changed values are found in the storage's change
ticks (`Changed<T>` with the half's own tick), not one `Ref` per value.
`BridgeCost` counts the values and the time each half of the copy took, and
each side logs its costliest types every 1000 steps; `EXA_BRIDGE_ALL=1` copies every value every step (a
debug aid). The hierarchy and transforms (`ChildOf`, `Children`'s order,
`Transform`, `GlobalTransform`, `Name`) cross only for the entities the
simulation holds: on the frame's side an entity is marked `SimShared` once
another bridged type crosses for it (either way), with its ancestors, and
only marked entities are scanned; a newly marked one sends its hierarchy
whole. The frame's own entities (meshes, bones, lights, the GUI) stay home:
in exanima01 and exanima06 the simulation holds about a third to a half of
the frame's transforms. A `Children` order moves only the children it
names. `EXA_BRIDGE_WIDE=1` scans every entity again (a debug aid). Every plugin with fixed-step systems registers its data (a
`bridge_*_types` function next to its build); bodies are copied when they
change, the collision world, the navigation grid, the cloths and the
solver's stages are lent. The game runs its fixed step this way by default
(`--no-sim-bridge` keeps it in the frame's world). The solver and the
sector lists walk their bodies in the original's sector-list order (below),
the character tick in entity order, so neither world's table layout
decides it. A body's
every pose is the same bit for bit behind the bridge, inline and on the
thread (`body_floor_test`), and a scripted run logs the same changes,
removals, spawns, children and messages on both sides as one world
(`sim_bridge_test`).

Walk order. The original appends each body (each part is one there) to
its sector's list when it joins the world, a node before its children, a
node added later (a spawned character, an item put on) at the next
simulation upkeep in the order it was added. Its step walks that list from
the end, filling the awake list the iterations, detection, friction and
post-step all walk; so the last body to join comes first and a tree's
root last. Exarust gives each `Ragdoll` its place (`Ragdoll::list_place`)
when it is first inserted on an entity (a bridged body keeps it), and the
character and prop builders insert their bodies in spawn order (entity
index). `body::walk_order` sorts the solver's bodies (`step_bodies`) and
the body grid (`sector_lists::rebuild_sector_bodies`) by it; the pair
stage walks each body's parts in reverse list order (`Ragdoll::list_parts`,
the tree's node order) and meets the parts of each cell (own cell first)
in the order a cell's list holds them, last joined first. Of two equally
mobile parts, the one earlier in the list brings its spheres: the
original's key word holds the list index under the order key (16 bits
there; Exarust compares the whole place, the same order for any sector of
fewer than 65,536 parts). The all-pairs sector walks its list from the
start, each awake part against every later part.

Cell lists. A grid sector links a body at the head of its cell's list when
it joins, and its step moves an awake part whose cell changed to the head
of its new cell's list (the awake list from its start, after the
iterations and the inverted-tetra repair, before the detection); a
sleeping part is not moved. Each `Part` keeps its cell entry (the cell and
the time it was linked there, `body::pairs::CellEntry`); the pair stage's
`link_cells` joins new bodies in list order and moves the awake parts that
changed cells in awake-list order on its own clock before each detection,
and each cell's parts are met latest-linked first. `EXA_CELL_ORDER=join`
meets them in joining order instead (the order before the head move) and
`EXA_CELL_ORDER=trace` logs the first detections where the two differ. The original steps each
part as a body of its own in the list's reverse order; Exarust steps a tree
in one `Ragdoll::step`, which gives the same values because a part's step
reads only its own particles and the character's words (the carry, the
tone), and the root, last of its tree, restores every part's edges at the
end, as `Ragdoll::step` does.

Stage 4 turns the frame's writers of simulation data into work the
simulation does itself and the copies out into snapshots, group by group.
`sim_app::SimFrameStart` holds the fixed step's once-a-frame work (the
bodies' frame start: their per-frame tick counts reset), run
on the first fixed pass of each frame in either world, so the frame no
longer writes every awake body. Cloths cross as `ClothMeshUpdate`
snapshots (`exa_world::cloth`); the cloth itself is `bridge_copy_in` (sent
when the frame builds one, never back). A type a moved system touches that no
plugin registered stops a debug or test build at the first step; a play
build warns and skips the system.

`sim_app::FixedCommands` carries the frame's writes to simulation data: a
frame system that decides a change posts it (`post`, or `run` with a cached
one-shot system and its input), and `FixedCommandsStep`, first after the
lockstep in the fixed order, runs the queue at the start of the next fixed
step on the world that runs the step (the frame's in one world; behind the
bridge the queue is lent to the simulation for the step). The change lands
where the copy used to land it, and a simulation running apart from the
frame can take it in order. Posted so far: the viewer's fells and mechanism
poke, the door toggle, going limp, finished limb trees, the hand's door and
lever releases, the grab a hotkey's entry into combat lets go of. Coupled writers move into the simulation instead, with their
same-frame readers: the rise and its restore, then the golem alarm's zone
check, run in `SimPreStep` before every fixed step (the original installs
its AI manager's frame routine as its simulation loop's pre-step hook, which
the loop calls before each 12 ms step; the routine has no rate guard of its
own, the zone check only its 3 cm movement test). The hit chain runs every
step in the fixed step's post-step (`body::BodyPostStep`), as the original's
post-step hands a character's pending hit to its damage routine every 12 ms
step: the strikers tracked from the bodies the tick left (a held body's
first part, or the wielder's carrying part and the striker's placement under
it; the tip's velocity over one tick), the held bodies' contacts classified,
the bodyless strikers' sweep against the target's body parts, the damage, the
bites, then the brains' reaction to blows and the debug pauses. The waiting
wounds count down before the characters' ticks (the original's body tick
paints its queued marks), and `body::deal_shoves`, first in every tick, deals
the blows the last post-step (and the frame) sent: the original's damage
routine sets the push, daze, sag and knockback step in the post-step, so the
next body tick already sees them.

The hand stays in the frame, as the original's hand runs on its main thread
per input event (the GUI's cursor handlers) and writes the bodies directly.
Bodies (`Ragdoll`) and the mechanisms on them (`DoorBody`, `LeverBody`,
`PlateBody`) are snapshots behind the bridge (`bridge_snapshot`): copied out
when the simulation changes them, copied in only when the frame inserts
them. A frame system that writes one calls `FixedCommands::write` (or
`write_pair`, `write_many`): the frame's copy takes the write at once without
a change mark, so the frame's later readers see it, and the same write is
queued for the simulation's copy, where it runs at the start of the next
step before the frame's other changes. In one world it is a plain write.
The hand's press, hold and reach, the GUI drop's lift, the use cursor's
unlock, the flow's knock-out, revive and drive steps and the alarm return
write that way; a frame write to a snapshot that is not posted is not sent
and is warned of once. The original appends a grab link to the body's link
array from its main thread with no lock and no deferral, and its simulation
thread takes it at its next step; Exarust lands it at the next step too: a
step's difference at most, no rule change. Cloths stay copied in only (a
cloth snapshot would copy every awake cloth out each step); the hand's cloth
grab and release are posted the same way. `SimFrameStart` runs its systems
one at a time, in its sets' order (`SimFrameStartSystems`: what the frame
ran in `Update`, then in `PostUpdate`, then the bodies' frame start), from
`FrameStartStep`, a step of the fixed order (after the lockstep, before the
commands) that runs it on the first fixed pass of each frame (Bevy's frame
count, bridged, tells the frames apart): the same point in both worlds.

Stage 6 (a simulation running apart from the frame) needs every value
the two sides share to have one owner, since a free-running simulation
cannot borrow the frame's data for a step. Lent resources the frame never
touches stay lent for now; the ones it does take an owner group by group.
`bridge_copy_in_resource` is a resource the frame owns and the simulation
only reads (copied in when the frame changes it, never sent back; a
simulation write is warned of once and stays there);
`bridge_snapshot_resource` is one the simulation owns and the frame only
reads (copied out when it changes, in only when the frame inserts it; a
frame write is warned of once and not sent). The input and settings the
player's steering reads (bindings, pad, hand, cursor ground, hardware
settings, idle watch, spectate), the loading status, the item catalogue,
scripts, role database, the search's items and the level's local items,
the world clock, story mode, possession, the portal screen and the
kinesis inputs are copied in; the effect clock, the path probe, the aim
cursor and the kinesis pick hook are snapshots. The search reads the open
loot windows and popups from `exa_game::npc_search::OpenSearchScreens`,
which the frame keeps before its fixed steps, instead of the screens.

Values both sides write take one owner too. `sim_app::FrameCommands` is
the simulation's queue of writes to data the frame owns: a fixed-step
system writes its own copy at once (without a change mark behind the
bridge, so the copy is not sent back) and queues the same write, which the
frame runs right after the step, before the step's other events; in one
world it is a plain write. `sim_app::write_resource` is the other way, the
resource form of `FixedCommands::write` for data the simulation owns.
Owned by the frame and copied in, with the step's writes queued: the flow,
the item cursor (the step takes the hand's released points), the doll's
item set (its templates and copies behind `Arc`s, so the copy is cheap),
the story's experience, the play session, the death fade and the chase
camera. Owned by the simulation, the frame's writes posted: the attack
latch, the simulation controls' scripted pauses (the frame's stepping and
start-speed state is `exa_game::sim_control::SimStepping`), the blood
sprays (jets now start in the post-step that dealt the bleeding blow, and
tick from the next tick), the cloth debug counters; moved to the
simulation with the frame's writes posted: the weapons' blood list, the
room cells, the pass turn, the navigation revision, the search's world
(its screenshot requests go out as `SearchShot` messages), the scan build
(the debug scan is built at the next step) and the solver stages. `FrameCommands::send` hands the frame
work it alone does with a step's results (behind the bridge after the
step's copy out, in one world in `FrameCommandsStep` after `FixedLast`):
the plain bodies' impact sounds and scrape loops are decided in the step
(whether a set has a variant is read from the sound pack,
`exa_audio::SoundBank::set_size`) and played and driven by the frame,
which resolves the set names (`exa_audio::bodies::FrameBodySounds`). Each
gizmo group keeps its own drawing buffer on each side
(`bridge_own_resource`) and the step's collected gizmos reach the frame as
a snapshot (`bridge_gizmos`). The effects' random streams (the run-time
library's Mersenne twister and the engine's generator) are process-wide in
the original and drawn from by both of its threads with no order between
them (152 and 531 call sites across the GUI, particles, AI, dynamics and
effects): the step keeps `EffectDraws` and `EffectRoll`, the frame its own
`FrameEffectDraws` and `FrameEffectRoll`, each in its own order. The other sounds a step starts
go the same way (`exa_audio::step_sounds`): the fall's scream and pit,
the shock's hit and cry, the kinesis and throw picks and the throw's lift,
whose voice the frame keeps under a key the step hands out so a later
fade finds it; a voice line's "has a set" test reads the sound pack. The
alarm's wake pitch draws from the step's own engine generator
(`exa_game::golem_alarm::AlarmRoll`). The sound bank, the audio
environment and the clips are the frame's alone. The script engine is the
frame's too: the original runs it from the AI's target pass (encounters
and greetings, simulation thread) and from the dialogue (main thread)
with no lock; the step takes the pass's reports and the roles unit's
relation query and hands the scripts to the frame, which runs them with
the session cast after the step (`exa_game::role_scripts`). The collision world is the simulation's (`Physics`, moved);
the frame's picks query a replica with the same handles
(`collision_share::FramePhysics`): the frame's inserts, removals and
clears apply to the replica at once and are posted to the simulation; the
step's removals go to the frame after the step; after a step that moved or
changed anything, the frame takes the moved colliders' poses and a copy of
the step's broad-phase tree (`share_collision`, in `FixedLast`), so a pick
sees the world as the last finished step left it. The original's main
thread reads its simulation's scene with no lock; a pick against the last
finished step is the race-free form of that read. The stand-in list and
the collider-node map are the frame's, copied in, the step's takes
replayed there. The navigation grid is the simulation's; the debug
overlay draws a copy the step sends only while the overlay is on
(`exa_world::nav::view::NavGridView`). Cost (`collision replica` log line,
every 1000 steps; `EXA_SHARE_ALWAYS=1` shares every step for the worst
case): exanima01 0.40 ms and exanima06 0.53 ms on a shared step, 0.05 and
0 ms a step on average in a walk.

`sim_digest` (`--sim-digest <file>`) writes after every fixed step a hash of
each bridged type's values (entity ids left out, values sorted), in its own
schedule after `FixedLast`, which moves with the fixed step: a one-world run
and a bridged one compare tick by tick and type by type, and
`EXA_DIGEST_DUMP=<step>[-<step>]` writes the values themselves. Behind the
bridge the frame's copy is digested too (`<file>.frame.tsv`, after each
step's copy out). It found the
bridged duel's drift: query filters were not in the coverage, so `With<Player>`
matched nothing in the simulation's world.

`engine_math` is the one shared implementation of the engine's fast maths,
checked against the original's instructions: the sign stub, the
`MINSS`/`MAXSS`/clamp compares, the floor and ceiling conversions (the
rounding mode set and restored), the polynomial logarithm, exponential,
sine and sine-cosine (the polynomials picked by mask arithmetic), the
arctangent and octant atan2, arcsine and arccosine, the double-precision
angle wrap, the 4×3 products, triple product, fast and exact barycentrics,
quaternion product, axis-angle and matrix conversions, the pitch/yaw, X,
axis and look-at turns (the look basis normalised with `RSQRTSS`), the
signed XZ heading through the fast arc cosine, the
yaw and elevation directions, the 2D, 3D and ground-plane lengths in the
original's x-and-z-first association, the per-component and folded minimum
and maximum with the SSE NaN rule (which `intersect` folds its slabs with), the approximate vector reciprocal (`RCPPS` on x, y, z and 0; the oriented-box slab tests scale by it), the approximate normalisation (`RSQRTSS`, zero kept), the four-lane quaternion normaliser (`(y² + w²) + (x² + z²)`, its exact reciprocal root: `normalize_quat`), and the projection onto and removal
along a unit vector (the dot summed x and z, then y), component-wise row scaling, Euler angles with the runtime library's
arcsine and arctangent rules, the Gauss-Jordan 4×4 inverse (pivoting
against the diagonal) and the OpenGL frustum, perspective (with the
sub-pixel jitter) and pixel-rectangle matrices. The runtime library's own
transcendentals are stood in for by the platform's double functions. The
UI's `engine_math` forwards here, and the body's twist helpers are
re-exported from it. Every crate that may depend on this one uses these
functions rather than its own copy: the fast logarithm and exponential
(the mesh vertex cache, the damage model, the item randomiser) and the
approximate reciprocal square root (the renderer's mipmaps, the AI's
units, the world's scene environment, the combat's collapse step, and the
body class's own normalisations: the torso twist's first one, the midline's
half turn in the samplers and captures, the thorax hunch, the femur
posture's axis, the daze pose's axis, the turn toward a fall, the get-up's
direction and the dying heading, each read in the assembly as a call to the
engine's `RSQRTSS` normaliser; the leg IK's normalisations are the exact
ones, as the original's). Only
`exa_formats`, which this crate depends on, keeps local mirrors.

`kdtree` holds the pieces of the original's background kd-tree unit that
stand alone: its builder queues' FIFO ring (256 slots, one entry fewer
than its slots, grown by 128 when full), the two-ring `BackgroundBuilder`
(incoming meshes, the reset's discard flag the next push clears, the idle
flag, the built meshes the frame hook drains, skipping deleted meshes),
the seven-slot query table the install copies onto a mesh and every
instance, and a sign-magnitude three-bit decode.
Static colliders are built by `StaticBuild` on the async compute pool, up to
120 000 triangles handed out a frame (a work budget, not a clock), and join
the world in the order they were handed out, so two runs build the same
world; the original's builder thread is modelled but not spawned.

The original's `RF_Intersection` unit is reproduced in `intersect`, a pure-maths
module that follows the original's instructions (sums in its order, the
processor's minimum and maximum, sign-bit answers, NaN branches): sweep
set-up, point–segment and point–box distances, sphere–sphere and
sphere–box tests, capsule–sphere, the slab tests of a swept sphere against
a box and an oriented box, segment against oriented box (slab and
separating-axis), point-set bounds, and the convex volumes: rebasing,
plane refresh (the normal (v2−v1)×(v0−v1) with the normaliser's
x²+z²-then-y² sum, a zero vector left zero), box tests, polygon clipping
into a portal pyramid, cutting a volume by a plane with its cap (the cap
closed by dropping its last point when its ends are nearer than 0.01 cm,
turned round and chained from its start otherwise, a NaN distance
included), and the directional
shadow's caster volume (a face open to the light when its plane dot with
the homogeneous light is ≤ 0, one side plane per open/kept face pair
through their first reversed shared edge whose squared length is at least
1 cm² summed (x² + y²) + z², a NaN length kept, the kept faces closing it; its returned
side count is the original's last index, one less than the number of side
planes). Its cube-face mask lives with its only user,
`exa_render::cube_shadows::sphere_faces`. The body and level paths still
take rapier's broad-phase queries and the pair stage its own bounding-sphere
test (the original's needs each part's mesh box), and the render side has
no directional shadow volume or portal clipping yet, so nothing in the
simulation calls the module yet.

`race_runtime` builds a race's runtime data once per id: its four gait
rows scaled to the 83 Hz tick (the thrust and clock by 1/83, the push by
1/83², the loader's brake distances and the backpedal's signs) and its two
gendered voice templates, with the built-in record for a missing id. The
character body takes a race's gait rows from the same scaling. `shock`
models the electric shock a body takes: the 150 ms window, the `elechit`
sound's draws (pitch first, then gain), the races that are released into
the shocked state instead (and that release), race index 1 that only
rings, and the burn of the others (yellow health, the shock-stagger bit,
the stagger step along the shock and the pain reaction's part and
values). A knockback step's stance is clamped, as any step's, to 1.1 of the
stored step reach × scale around the frame (`body_steps::stance_origin`,
in double): the scaled length the original's feet routine rewrites for a
step of kind 2 has no reader there. The running game's arc shocks (`exa_game::thaum`) apply their
own clock and health rule and do not call `shock` yet; the body tick's
shock stagger and the release of a release race (`Body::force_release`)
are live, and a staggered body on its feet holds the daze pose
(`exa_anim::shock_hold`, through `exa_game`'s sampler stage). A revive's
drive (`Ragdoll::revival_drive`) starts from the balance the body's last
living tick measured, kept when its muscles went.

A subtree marked `StaticGeometry` has its meshes copied into static
triangle-mesh colliders. `NoStaticCollider` on a mesh (or subtree) under it
keeps that mesh out: the original draws terrain vegetation but never
collides with it. A `StaticNode` marks a scene node whose name a pick can
return: `ColliderNodes` keeps one entry per static collider built under
such a node, with no name copy (the node entity's `Name` holds it). A mesh
of generated level geometry has no `StaticNode` above it and keeps no
entry. A rigid body's mesh is its stand-in instead and is not in the map.
A `PickOnly` mesh (a model's `_null` group, never drawn) becomes a
pick-only collider (`PICK_ONLY`, no collision or solver groups): only
`Physics::fixed_pick`, the hand's cursor pick, meets it; the bodies'
level contacts, `fixed_hit`, `fixed_pick_except`
and `level_sweep` pass through it (`solid`); `fixed_pick_where` meets it too,
over the colliders its filter keeps (the casts' picks: the level but the
props' stand-ins, or the stand-ins alone).

`Physics::clear` empties the world (every body, collider and joint) and
re-applies the fixed gravity and step; `body_count` and `collider_count`
report what it holds. A level change uses it after despawning the old
level's entities (characters keep nothing in it).

A character has no capsule and no swept controller, as the original has
none (its character position is its frame, which follows the pelvis; walls,
props and other bodies block the body's own parts). `CharacterBody::step`,
at the start of each fixed step, puts a character at once where its body's
frame was last placed (`CharacterBody::follow`, across the ground; the
carry leaves that move out) and at the character's ground its body measured
(`CharacterBody::ground`, from `Ragdoll::character_ground`; without support
it goes down with the pelvis, never under that ground). A character without
muscles walks by its desired velocity and stands on the level under it (one
ray down), falling where there is none. In the sewer this took the fixed
tick from about 100 ms to under 10 ms (round 62's capsule spent it on its
ground contact against dense prop meshes).

Each fixed tick (83 Hz) first runs the behaviour (`body::TickBehaviour`:
the AI's brains, `exa_ai::BrainTick`; the grips, the mechanisms'
registration and the held weapons' sync are ordered after it), moves the
characters to their bodies' frames (`CharacterControllers`), then runs the
pre-step (`body::BodyPreStep`), the character ticks (`body::CharacterTicks`:
each living character, player last, runs the nine stages of the
`body::CharacterTick` schedule, `body::CharacterStage`, before the next one
starts; `exa_game` fills them: clips sampled and targets set), the body
solver over every awake body (`body::BodySystems`) and the per-character
post-step (`body::BodyPostStep`). The solver calls the registered
`body::SolverStages` at the original's points: after integration, the pair
pass and links between bodies in each of the 20 iterations, sim objects,
body–body detection, sim-object contacts, the pair friction of each of the
8 passes, after the post-step (`src/body_stages.rs`, `body::solve_with`,
`body::solve_among`, whose detection and friction also see the sleeping
bodies). The pair stage (`body::Pairs`, `src/body_pairs.rs`, registered by
the plugin) keeps body parts out of each other as the original's pairs
do: after the iterations every awake part meets the parts near it (awake
or asleep) by the original's group masks within a body and layer masks
between bodies (`body::Pairing` on each part), the more mobile one's
spheres against the other part's collision mesh (on a tie, the part
that joined the sector's list first; see the walk order below) (`body::Pairs`, the
original's mesh query: the swept/static choice on 0.1 cm² of travel, the
mesh-box and mesh-sphere pre-tests, the one-sided per-triangle tests with
the sphere re-read after every hit; a mesh with a kd-tree, `Part::tree`, is
walked leaf by leaf as the original's tree queries walk it, a mesh of ten
triangles or fewer in file order, see `mesh_tree`), or its spheres,
pre-tested against their own bound, where the part has no mesh triangles
(a prop's part gets its node's triangles at registration, from the model's
mesh, so a prop is the mesh side whenever it is the less mobile part of a
pair; a character limb gets its node mesh when its body is built, a held
item its node's and a factory weapon its merged mesh;
`Ragdoll::set_part_triangles`, `set_part_mesh` and `set_limb_mesh` set them
by hand; `body::pairs::swept_touches` asks the same swept query of one part
with a callback that only notes the hit, as an electric arc's segment does;
a part with 64
spheres or more, a door leaf's 1 900, has them bucketed in a grid,
`body::SphereGrid`, so the sphere fallback visits only the spheres a sphere
can reach, in their stored order, with the plain loop's results), with an
immediate
push by mobility shares and φ; up to four points per pair are solved in
the next tick's
iterations before the links (0.01 cm slop, the reversed pair friction,
dropped once satisfied) and rub in the friction passes; a contact
equalises the two bodies' sleep countdowns (`body::equalise_sleep`). The
sector's bookkeeping (`src/sector.rs`) holds the original's 600 cm grid
(cells by round-down of × 1/600 in single precision), the static span
rounded half to even with one cell of margin all round, the body and
sim-object position-overflow guards, each part's sphere-set and mesh
bounds (`sector::Bounds`; a prop's mesh bound is its node's box and
radius, a limb's its node box), the two-way meet test that gates a pair
before contact generation (`sector::meet`, the original's slab test of
each side's travel against the other's box grown by its radius), the
overlap-only, placement and static-collider pre-tests, and the step's
stages and budgets. The placement overlap query is
`sector::placement_scan`: the bodies bucketed in the nine cells around the
probe's own, in the neighbour table's order, each offered when its bound
sphere and the body's mesh sphere touch, the probe's sphere centre lies
within its own radius of the body's oriented mesh box and the body's
sphere centre within its own radius of the probe's
(`sector::mesh_box_distance2`; both strict, a NaN distance passes), the
group words share no bit for a body of the same root and the layer words
name no shared exclusive channel for another root, and the body is not
disabled (0x800, an item on the cursor); each pair says which side goes first (the lower pair key, as
the original orders the two). In the game the query runs on the live body grid
(`sector_lists::placement_touches`) when the item cursor lets go of an item: the item at the
frame the lift saved, each offered part through the pair test (`sector_lists::placement_touch`:
the lower-key side's collision spheres in the other's frame, the pre-test `sector::placement_meet`
with 2 cm of padding, where a NaN distance passes as the original's unordered compare does, then
the other's mesh sphere query at the radius plus 2 cm, a part without triangles standing in with
its spheres), and a touch raises the lower of the two sleep countdowns to the higher
(`equalise_placement`; `place_released` runs it on the simulation's world, posted by the hand).
The release itself wakes the item to at least 128 (`RELEASE_WAKE`) and settles it 8 ticks
(`RELEASE_SETTLE`), as the original's put-back does. While the item cursor carries an item its body is out of the
simulation (`Ragdoll::hold_out`: asleep, disabled, countdown 0, as the
original's lift leaves it after every placement), moved whole so its grab point lands on the
placed point (`Ragdoll::place_grab_at`); the release's raise therefore gives it 128, and a
neighbour woken at its old place 128 too. The original's "is the space free" query
(`sector_lists::space_taken`) answers whether a body at given frames meets another: each part's
bound sphere against the other's mesh sphere, the layer channels, a disabled body skipped, then,
lower key first, the overlap-only body test (`sector::static_meet`) and the overlap pair test
(`overlap_touch`: `sector::static_overlap_meet`, no padding, a NaN passing, then the mesh's
boolean sphere query at the sphere's radius); the item cursor refuses a marker it would overlap
with it. The original walks the probe's nine cells; the frame side asks every body, which the
bound-sphere gate reduces to the same set. Its static-collider grid (`sector::StaticGrid`: one
cell per small collider, the footprint plus a one-cell margin with the
wide bit for a larger one, a scan of the own cell then the eight
neighbours, shared-mask and own-parent skips) and its simulated-object
list and grid (`sector::ObjectGrid`: the object overflow guard that also
takes the first cell of the second row, migration and removal) are live:
`StaticColliders` fills the grid from every level collider as the static
builder inserts it (its world box in the original's centimetres, its
piece and its level mask), and the level's cloth stands in the object
list, migrating after its tick. Each part's world pose (`sector::body_pose`: the
matrix from its particles and rest basis, the world box and sphere of its
bounds, the cell of the mesh box's centre) is what the pair stage buckets
and meets by. A body whose cell index overflows goes back to its
particles of the tick before last and is parked past the first row
(`sector::reset_overflow`). A sector tests every pair unless the scene's flag word asks for
the grid (`BroadPhase::for_scene`), and the registration walk
(`sector::registration`) decides from a node's words whether it builds a
body, only inserts one, registers a simulated object or a mesh as a
static collider.
Registration's impact sound scale (`sector::impact_scale`, `v = min(2m, 1)
+ 1`) is the original's. The level detection's masks are the original's
(`sector::static_mask`, `body_mask`, `level_meets`): a collider is 1 and
gains 4 when flagged or thin and horizontal; a hung body is 1 and a living
foot 4; a pair is skipped when the masks share a bit. The pair stage
buckets parts by their mesh box's world centre and first asks that the
awake part's sphere touch the other's mesh sphere; a small sector's
all-pairs walk
(`body::Pairs::all_pairs`) adds the original's two point-to-mesh-box
tests. A part's mask (`Level::mask`) is tested against its body mask
before any triangle is queried; the body solver hands the detection the
level with the static grid's stored masks (`body::MaskedLevel`), so a thin
horizontal plate (local half height under 1 cm, world up over 0.99) is
mask 1|4 and a living foot's mask-4 part never meets it (`Ragdoll::level_mask`,
`Part::skips_thin`: the leg routine sets the foot's 4 on each lower leg's
first child every living tick that is not a collapse, and the collapse and
death clear it, so a collapsing body's and a corpse's feet meet thin
plates); a piece with no
record answers 1. The original's flagged-mask bit has no traced node
source, so it is never set [guess]. Not wired: the overflow guard's reset
in the game (Exarust's grid is sparse and may hold any cell, so no body
leaves a bounded row).
A pair with exactly one placed body (state bit 0x1000, which the original
keeps only while the settling countdown runs, so `Ragdoll::placed` is
`settling() > 0`) is one-sided: the placed side takes the whole correction
over its non-negative weights and records the opposite push in its offset
array (`Part::placed_offset`, read by `Ragdoll::placed_offsets`), which
its node controller would read; the other side does not move. The
integration clears that array while the body is placed and sleep clears
it too, as the original's contact offset array is
(`Ragdoll::clear_placed_offsets`). Its detection runs the static
(unswept) sphere query only and records contacts without impact hits, so
it never strikes.
The impact path reads the parts' smoothed velocity (a tenth of the way to
each tick's displacement), which the
pair's touch carries as its travel alongside the mean contact point and
the summed push; a pair that takes the hit event exposes the original's
handler payload as `body::pairs::HitEvent` (the other side, the mean
point, the fastest travel and the summed push).
The sector's simulated-object stages (`sector::ObjectContacts`) hand a
cloth, of the parts the grid sector's object stage walks (those bucketed
by their mesh box's world centre in the nine 6 m cells around the
object's own cell, `CellGrid::around`, in the neighbour table's order;
the original's small-scene all-pairs sector offers every body, Exarust's
levels are grid sectors), the parts passing `sector::object_body_meet` (the bound spheres
must not be apart, then the object's centre within its radius of the
part's mesh box, the original's point-to-box distance; a NaN passes both),
each as a `cloth_body::ClothBody` with its body's countdown and its
collision spheres in the object's own centimetres, last sphere first,
padded by the cloth's 2 cm and kept only when it reaches the box about
the cloth's particles, swept when it moved over 0.01 cm² (compared in
double, the cloth's own gate), and the level's triangles of the static
records `StaticGrid::near_object` takes (the nine cells around the
object's cell, the same two tests against each record's world box, where
the original uses the mesh's oriented box), each record one
`cloth_level::LevelPiece`. A sleeping object takes only awake bodies
(`sector::object_takes_body`). Skipping the cloth's own character is a
guess (the sector makes no such test); skipping the static meshes of the
cloth's own scene node is the original's.
The cloth's body contact (`cloth_body`) pushes cloth *triangles*, not
particles: each sphere walks the cloth's triangle records (the sphere
about each triangle at the last tick and now) with the swept query, takes
a triangle it came from and ends within its padded radius of, puts its
sweep's midpoint on the face, finds the triangle's closest point (the
original's branch order and weights) and pushes the three vertices off it
by `depth × weight / largest weight` into a push buffer that never adds
the same direction twice. A body that pushed trades countdowns with the
cloth (the lower takes the higher), so a sleeping body never wakes or
keeps a cloth awake and a moving one hands it its countdown (256 for a
character). An awake cloth applies the buffer after the body stage: the
vertex's travel across its push is cancelled up to 0.2 cm² (else
shortened by √0.1 cm), the vertex moves out by the push, and three more
constraint passes (the shipped iteration count less the step's one) end
by putting it back on its push plane. The body's share (0.002, 0.01 or
0.015 of the push, the friction and rustle energy) is not handed back:
the cloth stage has no mutable body.
A cloth is set up from a parameter record (`cloth_record::ClothRecord`,
the `3DD0C001` block the original's set-up copies): the built-in record
every shipped marker falls back to (`LEVEL`: triangle mode `0x7208`, 0.05
drag, 2 cm padding, 0.1 cm slide, four passes, the `CltMov` rustle) or the
one the character build hands a hair style (`HAIR`: particle mode `0x104`,
0.2 drag, no padding or slide, two passes, no rustle). Both keep half the
velocity a second (`exp(ln 0.5 · 1/83)` in double, times the world's
0.98977) and gravity 1. In the particle mode the step makes all its passes
itself, the post-body pass does nothing, and the body contact
(`cloth_body::touch_particles`, gathered by
`ObjectContacts::gather_particle_bodies`) pushes each particle inside a
body sphere straight out to its surface, padded 1 cm on any body but the
cloth's owner (none there; the box test takes the squared padded radius as
its radius, as the original's does), taking off its travel across the push
less the body's own motion there (the tetrahedron's barycentric weights,
the original's approximate reciprocal) up to a tenth of the push's square,
else shortening it by `√(0.1·push²/travel²)`; a body that pushed hands the
cloth its countdown when that is higher.
The cloth's level contact (`cloth_level`) runs after its step, mesh by
mesh (the radius and slide are the record's; the built-in's below); the
stage gathers the level's records and triangles from where the step left
the cloth (`cloth::step_before_level` then the records, then
`cloth::finish_step`), as the original's object loop gathers its static
contacts after stepping it. Each particle that moved over 0.1 cm² sweeps a
2 cm sphere through
the mesh, any other is a 2 cm sphere where it is, tested against the
piece's box and then the piece's triangles near it (a per-piece grid, as
the mesh's own tree query hands only the triangles around the sphere) in
order with the mesh's own sphere and sweep queries (front faces only,
pushed out to 2 cm); a pushed particle's travel is then shortened by
0.1 cm, or cancelled when shorter. A level push (or a level plane's)
never wakes a cloth. The cloth's countdown reads the squared length of
its particles' summed velocity (not the tick's step, which carries
gravity), only a count that went up is capped at 32, and the tick's end
puts an awake cloth whose count is under 2 to sleep, so a cloth held at
rest falls asleep. The cloth's pins are the original's start-up list
(`cloth::face_flag_pins`): a particle is pinned only when every face about
it carries a pin bit (`0xF0000000`) in the mesh's face flags, at weight 1
when each also carries a firm bit (`0xC0000000`), else 0.1; every shipped
cloth marks its hem with `0x80000000`, so the hem is held and the rest
hangs free (the old guess held every vertex at 0.1 toward its rest pose,
which fought the body pushes of the props beside five exanima01 banners
and kept them awake). The
object/object stage is `sector::object_pairs` (every ordered pair of the
list, never an object with itself, no bound test in the sector); the
cloth's own contact law for another object is outside this component and
nothing consumes the pairs yet.
A part's level contacts also make an impact record (`body::LevelImpact`):
the mean point of its accepted hits, the fastest smoothed velocity against
their summed pushes, and the original's gates (travel square over 0.1 cm²,
strength over 0.1, a part that reacts: every part but a foot).
`exa_combat::impact::level_impacts` consumes them: a character's part
sounds through the limb curve (a third unless the character is down) and,
over volume 0.25, deals the same impact damage and daze a body's blow
does. A plain prop's level impact is still only the record and its clock:
Exarust stores no collision-sound set on a prop.
Blows are queued for the next tick, and parts are drawn between their last
two ticks by interpolating their four particles and building the part's
matrix from them, as the original does (`body::BodyPlugin`). Every part is
pinned at its origin to its parent, and the skeleton's links bound each
joint's range (`body::Ragdoll::new`); a model node with its own body chunk and no skeleton bone (the demon's
tail and wings, `body::OwnPartBody`) is a plain part: its stored spheres
and links (their strength, smoothing and soft welds), links that are never
muscles (the init's tone of 0.01) and its own mobility over the init's limb
mass factor (`body::Ragdoll::make_plain`); a limp body's links also damp each
joint inside its range through their smoothed lengths, reset when the
body goes limp (`body::Ragdoll::go_limp`). In a living body every slack
link is a link muscle that turns its joint toward the clip's angle (its
damping term divided in double, its pull saturated through the processor's
approximate reciprocal root and reciprocal, `RSQRTSS` and `RCPSS`, as the
original's), with the gains the character tick sets from what the character does
(`body::Bearing`), its race (`body::RaceMuscles`) and its own character's
stats from its record (`body::BodyStats`: most energy, the limbs' scale,
the head's gain and factor and the walking arms of the undead)
(`src/body_muscle.rs`; the arm state machine also records the arm setter's
held factor, `Drive::held_contact`: 2, or 1 in an attack or a parry, which
the character crate's grip runtime puts on the held items' contact mobility);
a pose drive turns the pelvis, spine, head and feet toward the clip's
orientation about each part's own origin, scaled by the support S²
(`body::Drive`), and the living step ends with every part's six edges
restored, as the tree's root does when it steps last; and the carry holds the body up in place of gravity, with
a lift servo holding its pelvis at a target height over the ground under
its feet, a full crouch at 60 cm times the body scale (`src/body_lift.rs`). A character moves through a locomotion request its
behaviour writes (`body::locomotion::Locomotion`: heading, look, the
original's flag word, moving, free walk, goal) and the movement decision
its tick makes from it (`body::locomotion::decide`: the move vector, leg
mode and brake by the stop distances, the gates, the stop within 25 cm;
`turn_rate`/`turn`: the body turn by the original's table and the stance
facing; `align`: the alignment of the pelvis with the heading;
`body::locomotion::Locomotor` keeps the body's side), then the original's
locomotion routine while it moves (`body::locomotion::locomote`: the run
blend and its bleeds, the gait columns from the race's per-tick table
`GaitTable`, the walk's start key or resume, the boost ramp, the drive,
the speed along the heading and the locomotion clock's step) or the fall
of its run blend while it stands (`stand`; `src/body_gait.rs`); its
capsule carries the decision out until the body walks by its carry
(`capsule_velocity`, `CapsuleGaits`; `src/body_locomotion.rs`), and the
drive is kept, not yet added to the carry. The pose is put on the original's character
frame, whose origin follows the pelvis by its rules: on the pelvis while
moving, frozen while standing steady, behind dead-bands of 5 and 10 cm
otherwise (`src/body_frame.rs`, `body::Drive::anchor`). The controller
carries a standing body only across the ground and turns it, and follows
the frame (`CharacterBody::follow`, a tenth of the way a step,
at most 2 cm), leaving its follow out of the carry. Once a tick the
carry (`Ragdoll::aim`) sets the support pins' goals as the original's body
tick sets its pin records (`src/body_pins.rs`, `Ragdoll::set_pin_goals`):
the posed points on the frame, a thorax goal shifted 5·scale cm forward
without support, led toward a recovery step (`steps::lead_pose`, which also
writes the exertion value the head tilt reads, and `pelvis_lead_pose`, both
by the class set-up's rate factor 85/83), then clamped about the feet
centre; the thorax goals' mean is kept for the stance score. In each solver
iteration the support pins pull the two thorax pins once and the pelvis
pin twice horizontally toward those goals, each part's pins right after
its own links (`Ragdoll::pull_part_pins`, parts children first, so the
pelvis's come last), as the original's living link iteration does. `EXA_PIN_GOALS=1` logs every goal the carry leads or
clamps (its posed, led and final place and the reach). Each foot's ground is its own: the controller's floor
does not raise the lift (`src/body_lift.rs`). The feet routine's two other modes run
in `measure_balance` (`feet_mode`, kept as `Drive::feet_mode`): on a flat floor
(the state word's splay bit 0x1000, which the original's lying spawn sets;
in Exarust only the play flag `--flat-floor` sets it, on the player) every
leg's ground is 0, its anchor the sole plus 1 cm, its state 0x4a
(`flat_floor_leg`), its lower leg's reach the clip's last measured
hip-to-contact length, and the lift aims 10 cm under its standing target; a hovering race (the race factors' `hovers`, from the race
record's hover flag: the wraith) casts one 20 cm sphere 2000 cm down from
20 cm over its pelvis, 5 cm higher after each miss (`HoverProbe`), its feet
unprobed and its lift aimed at the sampler's hovering standing height as
it is: the clip root's height over the frame less 10 cm
(`pose_heights::hover_pelvis_target`), no ground added. A hovering race
runs its balance and feet with any number of legs: with fewer than two the
balance routine only puts the feet centre at the pelvis (no lean, score,
counter, step or footwork, so the balance stays 1), and its standing
support pins clamp around that centre; so the legless wraith floats about
92 cm up. Both give full support, skip
the per-foot probes, the stance's level, the imbalance decay and the legs'
relaxation (their legs keep the sampler's), and call no leg IK (`bend_legs`
returns). A walking leg's relaxation (`relax_legs`) is 0.6 − 0.001·| hip to
contact² − reach² | (the rest in double), clamped to 0..0.2 and rising by the
double 0.01; the reach is twice the contact's distance from the lower leg's
origin until a get-up before 0.8 of its clip gives each leg in reach the
clip's own hip-to-contact length (`Leg::reach2`), which it keeps. Before the feet a body off its feet (condition
bit 0x200) casts one 10 cm sphere 2000 cm straight down from its pelvis
(`down_probe_origin`, plus its zone's origin, zero in Exarust's one world
frame; `down_probe_ground`): a hit is its ground, a miss keeps it, and a
supported down body keeps that ground through the feet. The samplers' heights (`body::pose_heights`: the stand height over
the lowest posed contact, the leg reach, the placement's and the living
sampler's pelvis targets) and the placement routines' arithmetic
(`body::placement`: the pose lowered by the pelvis's posed height and put
on the frame, the foot anchors on the ground or at a matrix, the matrix
pins 5 cm lower, the means) are tested against the original's
instructions; the carry aims the pelvis from the feet, not from the
samplers' targets. The spawn's placement at a matrix reads the placement's
target (`Ragdoll::placement_target`: the stand height of the posed
contacts plus twice the scale plus 5 cm, the not-walking margin of a
condition word still 0) and puts the pelvis at the feet plus that target
less 8 cm (`placement::SPAWN_PELVIS_DROP`), the pose lowered onto it
(`placed_parts`); the matrix anchors and pins and the means are not
called.
A golem's body links are smoothed at 0.995 (`Ragdoll::smooth_golem_links`,
from `RaceMuscles::golem`); the init's leg order is `legs_swap`. A body
that stands has no locomotion drive (`stand` clears it). A standing leg bends onto
its target by the law of cosines, at most 0.02 rad a tick (0.2 while a
recovery step is on), and the muscles follow that bend on the next tick
(`src/body_bend.rs`). The standing target is the anchor lowered by twice
the lift term, or, anchored, the midpoint of the contact and its anchor
at the anchor's height; while the recovery step runs the reach is measured
to the target and the thigh turns 75% of the hip-to-target angle (decaying
a tenth a tick otherwise). While the body locomotes the other IK runs: the
clip leg's own length against the physical hip's height over the anchor,
the target leg from the hip across to the contact, the knee and hip by the
law of cosines (the cosine's 30° fold compared in double, the engine's arc
cosine), the angle between the clip's leg and the target's negated unless
the clip's foot is ahead of the character frame's origin, all at 0.04 ×
the speed plus 0.01 a tick; a leg where the clip's leg is not longer and
the foot does not touch standing still eases both corrections back with
the identity shown. Both write the knee and hip corrections the sampler folds into the clip's
chain (`Ragdoll::fold_leg_ik`: it records each leg's clip ankle and hip
before the fold, bends the thigh and shin and re-places the foot under the
bent shin, as the original's IK chain places every part by its parent's;
the next aim puts the record in the world after its carry), the chain the
muscles follow, and
the foot's height over its floor, which the foot pitch levels. Each foot's sole floor is the
original's Ground Query (`contact::Level::floor_below`, a point ray from
30 cm over the foot's ground 60 cm down against floor-flagged pieces): the
hit when the low floor (the ground, 10 cm less while touching, plus the
leg's offset) is under it, else the low floor, or the ground plus the
offset where it misses, never over the knee (`feet::sole_floor`); a moving
foot's ground rises to a higher hit. The floor pieces are the colliders
built from a `FloorMesh` (a floor node's mesh: its second flag word's
`0x800000`, or a tile grid cell's floor parts, which the original merges
into a `+Floor` node with that bit), static or a rigid body's stand-in;
the build records them (`StaticColliders::is_floor`) and the body
detection's level answers `Level::floor_piece` from them. The level logs
how many it built ("static colliders built: … of them floor"). Each floor
piece runs the engine's plain mesh ray (`contact::mesh_ray`: triangles in
index order, front faces with the determinant at least the engine's epsilon,
unnormalised barycentric bounds, dot products summed x, y, z, a hit strictly
inside the limit) within the nearest hit so far, and a hit on triangle 0 of a
piece that holds its node mesh's first triangle (`FirstFloorPart`, the part
built from the group the index list starts with:
`StaticColliders::skips_first`, `Level::floor_skips_first`) is dropped, as
the original's query takes a mesh's hit only above index 0; the dropped hit
also hides that piece's farther faces. A grid cell's merged floor is not one
mesh in Exarust, so its first triangle is not reproduced.
A ghoul's thorax pins are scaled about the thorax origin by half its character size (`RaceMuscles::chest_pins`, `Ragdoll::scale_chest_pins`, the character init's re-scale for race classes 6 and 7); other races keep the stored pins, as the original's in-game bodies do (its full re-scale runs only in the creator's preview).
`Physics::line_cast` / `contact::line_cast` is the original's line cast
(front faces of every fixed solid collider, any triangle counting), walked
over the colliders whole so the frame's replica answers before the step has
shared its tree; the viewer puts a new player at the level's own start onto
its hit from 10 m up, 30 m down (`exa_world::start::cast_onto_floor`).
`Physics::static_floor_below` is the original's floor cast (the force
barricade's probe): static floor pieces only, straight down.
The sole turn (`feet::turn_sole`) projects the toe's gap onto the side axis
the leg keeps (`Leg::sole_side`): the foot's animated side axis, taken each
iteration, except while the body moves and the foot touches unlifted, when
the kept one holds.
The push waits for the feet: each walking leg whose contact trails its
animated spot along the facing (negated backpedalling) cuts the carry's
lag factor by `1 − (lead/(60·character size))²`, to nothing at the limit
(`measure_balance`), and the same lead marks a late touching foot with the
swing flag (0x10) against `10·scale·(0.25 − frac(0.5·leg + progress + 1))`,
at a walk raised by the gait's ground difference and capped at 25 cm.
The walk's feet plant and lift by their phases, and a slow foot plants
early (before phase 0.2); a plant starts a footstep at the character plus
`3·speed + 4` cm along its facing, with the original's gain and pitch
from two draws made when it plays (`feet::{gait_step, step_position}`,
the `Footstep` message), and the walking ground weights are in. The
plant's clothing rustle is a tested rule (`walk_rustle`: the foley base ×
(0.5g + 0.5)(0.02·speed + 0.01), pitch (0.2p + 0.9)(0.2·speed + 0.7) /
size, the foley clock 400 ms plus a jitter); nothing plays it yet. The
footstep's gain is multiplied by the character's footstep volume (its
stats': the body scale, doubled over 1.3, the ogre's 5). The balance
routine's foot sounds are live: its footwork's sound pass (between the
stance pick and the landing) compares each foot's anchored bit with the
state it last compared (`Leg::sounded`, then copied) and cues a rustle
when the bit rose (a step started: the squared contact-to-anchor distance,
doubled while attacking) and a footstep when it fell (a step ended: the
body's step loudness, 0.01 + 0.0139 × the last started step's length to
its spot before the pull toward the other foot, at most 1)
(`feet::foot_cues`, `FootCue`, `Ragdoll::take_foot_cues`, `FootSounded`);
`exa_audio` plays them by `anchor_rustle`, `release_step` and their clocks
(`rustle_clock` 400 ms, `release_step_clock` 250 ms, each plus a jitter).
Under the scene's water (`WaterLevel`, −25 cm when the terrain's wetness
is above 0.4, `water_level`) the release step is followed by its wet step
(`wet_release_step`, played at the cue's contact point) and a walking
plant by the walk's (`gait_wet_step`, from the planted contact's height
and the gait thrust the `Footstep` carries). The
post-step advances the walk's travel phase: the pelvis's travel in cm
times 0.0002, wrapped at 1, with the phase before the tick kept
(`body::travel_phase`); its consumer, the scene node's draw offset, is not
reproduced. When a recovery step ends, the free foot nearest the chest
plus five ticks of its fast velocity becomes the new support foot
(`body_balance::end_step`): a foot already anchored for the step is passed
over unless it has reached, the search starts from the original's
just-under-maximum sentinel so a NaN distance never wins, the first leg
stands in when none is free, and the standing position moves so that
foot's spot is where the foot is. A step running when the balance falls
under 0.2 (compared in double) is only cleared, as the original's balance
routine clears it: no foot is landed and the standing position stays.
The trigger runs for a step of any kind (a knockback or a tapped long
stride is re-targeted as any other while the chest is past it), its lead
reads the whole 3-D fast chest velocity, every taken target starts the
episode's landing count again, and a locomoting body still writes the
out-of-balance bit, the target and the count; a balance step from rest
writes no start clock, near-target count or stand-still stamp
(`steps::consider`). The episode checks and the other foot's 200 ms guard
read the start clock the footwork, AI, queued and knockback starts write
(`Steps::started`). With the body's slow-turn word (`Bearing::slow_turn`:
exploring, an AI idle state, a resting statue) a reached foot keeps its
step until the later stand-still stamp is 100 ms old. The foot scores'
reach term is measured from the pelvis eased a quarter of the way toward
the stance origin, and the scores and the anchor's clamp read the stored
reach whatever the crouch. The other foot's landing near the target only
clears the stepping byte (no step end). A queued footwork step is checked
inside the footwork on a tick it changed a foot, and when taken the chosen
foot is not started that tick (`steps::QueueView`, `Work::taken`); a
queued vector along the body's look (`Ragdoll::look_steps`, set by the movement
stage from the request's look) is first bent at the body's target's circle (`approach_bend`;
the target is the behaviour's, `Ragdoll::aim_steps`: the player's auto-target, the duellist's
foe). It is taken
while the queue word waits (`steps::take_queued`,
`Ragdoll::queue_step`): the queue becomes its countdown (by the episode's
landing count), the step kind and balance flag clear. The player's combat footwork and the AI's duellist
read and write the body's step (`Ragdoll::footwork_step`, `write_footwork`,
`FootworkStep`, whose `landed` counts the episode's landings;
`Ragdoll::age_step` ages a step without a tick, for tests and tools):
its start, target, long kind and queue; the body walks it on its feet and
the character follows the frame. The episode's balance-step flag is set
when the balance routine starts a step from rest and cleared by every
other start, the step's end and any tick without a step; the movement
gates, the muscles', the lift's and the frame's stepping read the body's own
episode (the frame's origin goes onto the pelvis every tick of a step, as the
original's anchor routine sets it while its stepping byte is set), and the
long kind clears on any tick without a step and at the step's end.
While a step leads (any kind, not out of balance) the pelvis pin is led
by its own rule (`steps::pelvis_lead_pose`: the error to the posed target
plus the step offset, × 0.7 (0.4 idle) × (0.75·s + 1) × 1.2·scale ×
level; `s` the error's cosine with the frame's forward axis, halved
backwards and scaled by its squared share of the step length forwards, 0
for a long step), and both leads measure the step's offset from the
standing point the anchor routine keeps (the frame's origin, on the
pelvis while stepping), so they fade as the body reaches its target. The
thorax reach narrows by the crouch less while the step episode runs.
The placement on the ground also restarts the chest history (both ticks'
chest at the placed chest, velocities zero), clears every foot state and
puts the feet centre at the anchors' mean. A clip that replaces Walk or Run asks for
the trailing foot (`Animator::take_left_locomotion`,
`Locomotor::plant_request`): the character tick plants it after that
tick's sample and keeps the request for 500 ms (the movement decision's
move branch drops it), then drops it and leaves
`Locomotor::step_done_notice` for the behaviour. A force release ends the running step and
stops the clip rate (`Ragdoll::force_release`, the feet's re-anchor waits
for the next tick). The chest's fast velocity over its slow one, less a
term that first decays ×0.85, is the body's jolt: past 2.5·scale cm a tick
in the walking condition (125 in any other, ×1000 unsupported) it ends the
step and, once in the episode, moves the standing position to the feet's
centre plus 100 × the jolt, at most 100·scale cm, telling the behaviour
while the body locomotes (`steps::jolt`, `Ragdoll::jolt_told`); each jolt
holds the latch 80 ticks, and a body off its feet takes none. The
muscles-off sampler's rules are tested (`body_limp.rs`,
`exa_game::limp::sample_limp`): 0.9 of the race's base rate, the clip's
full root travel, the gait overlay by √run, the height target's (2·run + 2)·scale
while walking (2·scale otherwise) over a −500 cm floor, every part's particles snapped to the pose on the frame
it had when the muscles went, the 0.05 height ease and the feet rule
(`limp_foot`): not walking, every loose foot is put down where it is;
walking with support (both kept when the muscles go,
`Ragdoll::limp_walking`/`limp_supported`), a foot the keys' flags
(`limp_plant`, the capture's own flag as the source while leading in) do
not ask for lifts, one they ask for plants at an anchor height of 3·scale
cm with a step (`limp_anchor_height`, `limp_step`). It has no caller, as in the original: its body tick returns
before the sampler for a dead body, and for a body whose muscles are off
but which is not dead it skips the clip clock, the clip-end test and the
sampler unless the body locomotes this tick (the movement decision, which
needs muscles, never runs for it) or one of two words that nothing in the
executable writes is set. So no body samples with these rules in 0.9.5,
whatever knocks it out: the knock-out writes the condition and the wake
nibble, never the muscles (traced for C8).
The placement on the ground (`Ragdoll::place_on_ground`, run by the
player behaviour's third tick from the nearest walkable floor) runs the
leg snap, takes its frame height as the frame's, puts every part at the
current clip's pose-only sample lowered onto that frame at rest,
re-anchors each foot at its posed contact on the frame less the clip root
at its ground plus the sole (`placement::ground_foot_anchor`) and
restarts the slack links' smoothed lengths; no ground leaves the body
alone. Every placement writes the facing from its matrix's forward
(`body::locomotion::Locomotor::face`, run by the ground placement and,
with the twist and reach reset of `Locomotor::placed`, at the fighters'
spawn): the forward flattened and normalised by `RSQRTSS` as the heading
and the stance facing, the body's yaw the heading's angle by the engine's
arc cosine, the cursor and the look the raw forward's. The character keeps its place (its feet on the character's ground;
moving it to the snap's ground made the earlier squat). The leg
snap (`Ragdoll::snap_placement`):
each probe is the clip ankle plus 5·scale cm
along the shin, a 10 cm sphere from the frame's height plus the leg's
length is dropped 300 cm for the ground, the hits average to the
character's ground, one foot's record (index 1 when leg 0's ground is
lower) picks the frame height (root over thigh, ground, the reach its
clip leaves, sole), each anchor
is the clip ankle at its ground plus the sole, and the standing IK writes
its corrections at once (`Ragdoll::snap_legs`, on the character frame
the body stands on). `Ragdoll::placement_feet` is the character ground the
snap averaged. The placement's resets before its first sample are tested and
not called (a spawned body already starts in that state):
`Locomotor::placed` drops the twist and the move vector's reach and takes
the yaw of the matrix's flattened forward by the original's arc cosine;
`Ragdoll::clear_leg_corrections` holds the legs' knee and hip corrections
at the identity until the IK writes them again. A plain start's capture
keeps half of the twist in the lumbar and thorax (`locomotion::half_lean`:
the forward axis halfway toward the turn out of the twist frame, the up
squared to it); `Locomotor::twist_before` is the twist a start made earlier
in the tick captures against.
The plain body's own collision sound and its scrape sources are in
`body::sound` (`src/body_sound.rs`): an impact over 0.5 sounds after the
cooldown or when it is strictly louder than twice the last one, at volume
`min(impact/100, 1)·1.5`, pitched by the part's mobility (a light part
below 0.1 scales its volume too), with a 220 ms cooldown less a rounded
share of the volume plus a 0–99 ms draw from the engine's generator. A
scrape source follows a point on the body, placed by the matrix the last
tick recorded: its squared move `d` (cm²) over 0.01 smooths to
`(√d·2 + d)·0.05`, at most one, the speed is a fifth of the way to it,
and a speed over 0.02 plays at `speed·volume` and `pitch·(0.1·speed + 0.9)`
at its part's position (the curve, smoothing and pitch in double precision).
The sources are stepped at the start of the post-step, before the body's
matrices; a body without sources skips the step. A prop's stored sound
slots (`0xDDB3` in its body chunk) are attached on their part when the
body registers, and its root carries the names the audio side resolves
(`prop::BodySounds`: each part's material collision set, each source's
sound). `body::impact_sound` runs the collision call on a part's own clock
and threshold (`Part::sound_ready_ms`, `Part::last_impact`) over
`part::base_impact_sound`: a part with no sound that passes the gate parks
its clock at the all-ones word; one that sounds stores twice the strength
and sets its clock with the generator's draw taken after the play.
`exa_audio` plays the impacts and drives the scrape loops. The original
drops a slot whose sound name the audio environment cannot find; Exarust
keeps the named slots and leaves an unshipped one silent.
Each character tick measures a living body's balance at its chest against
its ankles and asks the level for the ground under each foot
(`contact::Level::height_below`, the original's Height Query), which give
its support S (`src/body_balance.rs`, `body::Ragdoll::measure_balance`);
without support it also asks for the ground under its pelvis
(`body::Balance::ground`, read by the get-up's support and the fall
check). Its feet are planted as the original's are (`src/body_feet.rs`):
each leg's foot contact (`body::Leg`: `state`, a `body::FootState` of the
original's bits, `anchor`, `grip`, `ramp`) is planted where its foot meets
the floor, stood on its stance spot getting up or unsupported, cleared
lying, and steps to its stance spot by the footwork's stance score, one
foot at a time (a free foot within 5 cm stays). The score zeroes the
feet's height difference before its span, projection and side terms; its
reference term is 1 − the horizontal distance to the feet's segment × the
single-precision 1/30, except under the squared epsilon where it stays 1
(a NaN reference takes the root); the width and side terms divide by their
spans unguarded, as the original's do. A recovery step starts
when the chest is past 1.5 · R · E⁴ · S (45 cm for a settled human),
compared squared as the original does, and not during a shock stagger;
the balance keeps the support point and the feet's centre
(`Balance::support_point`, `feet_centre`) and an imbalance accumulator
that grows past the trigger and fades 0.01 a tick (nothing reads it). Its
stance origin and anchors use the footwork's own reach, 80·√scale·(1 −
0.2·encumbrance)·(1 − 0.3·feral) cm, and a walking body's ground is its
two feet's grounds blended by the gait's weights.
While it runs and the chest is not past the trigger, the thorax pins
lead by the horizontal error from the pin to the posed target plus the
step offset: 0.7, or 0.4 while Idle, times (direction + 0.5) times
(2 − 25·s) times 1.2 times the square root of body scale times the
stance level. A still 40 cm sideways error moves the target 33.6 cm.
The lower leg's and foot's friction follow its
state; in each iteration a standing
foot is gripped at its anchor (the PD grip, a walking one the velocity
servo, a lifted one only pushed up; the balance routine's feet centre puts
every leg's grip back at full each tick, so the iterations' rise from an
anchored foot's zero only runs within a tick) and its toe held over the floor
(`body::Ragdoll::clear_feet`, `body::Ragdoll::ankle`,
`body::Balance::level`/`extension`/`landings`; the overlay's
`body::Ragdoll::drawn_foot`). The controller's carry moves the anchors
with the body.
`body::Ragdoll::exertion` sums each part's farthest particle over the last
tick (cm), the summed limb motion `exa_combat`'s posture stage reads.
`body::Fallen` marks a body off its feet while its posture stage takes an
off-balance branch; `body::Drive::posture_wait` is the Rebalance's hold on
the posture switch; `body::Ragdoll::turned` turns a body's frame with a
new heading without carrying the body through the turn.
A collapsing body (`body::Bearing::collapsing`, from its condition word)
has its frame follow its pelvis exactly, its lift multiplied by its tone
squared, its arms at 0.2 and its feet probed standing; its support pins'
gain (`body::Drive::pin_gain`, 0.081 living) does not recover, and the
pins read that gain each iteration. A body
asked to go limp keeps its motion unless the request gives a velocity
(`body::GoLimp::velocity`); `body::daze_energy` is a blow's daze.
Velocity retention is the original's default environment: banded by speed
for limp parts, the fast value for living limbs; standing in combat the
limb factor is u rather than u² (or 0.97·u² in double for a body over 1.3
scale outside an attack), and a dormant body takes 0.95 of the fast value
in double first. A limp body sleeps by the original's countdown
(`body::Ragdoll::countdown`: 256, two off each tick under 20 cm/s, held at
30–32 between 20 and 40 cm/s, back to 256 above) and is woken by a blow,
by going limp or by a contact with an awake body (a sleeper woken by a
contact takes that tick's post-step, so one woken to a countdown under 2
sleeps again at once); a living body never
sleeps, though a dormant statue or golem counts down one a tick and
sleeps 255 ticks after going dormant. A body a level placed from its saved
pose is built limp and registered at rest
(`body::Ragdoll::register_placed`, the `body::PlacedCorpse` request): every
velocity zero and the countdown at 0, so it runs one tick and that tick's
post-step puts it to sleep, with deep sleep from the next; it never wakes
on its own.
Each part also keeps its state word and the part's own impact and wound
bookkeeping (`part`): the class bits this crate reads (`part::PartState`:
the strike window and chain, a held item, an arm, a muscled limb, a sole,
level contacts, the living limb family), a strike marking the whole chain
under its root (`part::set_striking`, `body::Ragdoll::set_striking`), a
held weapon's window opening and shutting only its chain bit beside the
attach's permanent weapon bit and mark (`grip::weapon_words`,
`part::set_strike_chain`; shields and torches are held parts, and the
pair stage's hittable test takes the mark too), and
a limp part keeping its muscle mark and getting its own contact mobility
back (`part::ragdoll_state`, `part::ragdoll_mobility`,
`body::Ragdoll::go_limp`). The impact rules are the original's slot 38's:
the recent-impact counter (+2 a call to 40, one off a tick,
`part::impact_added`/`impact_decayed`), a limb's and a held weapon's sound
volume, pitch and cooldown (`part::limb_impact_sound`,
`part::weapon_impact_sound`: the impact a third unless knocked down then
halved, the volume 0.05 of it at most 1, the pitch 0.6 of the volume plus
0.5), the hard-landing camera shake under 1800 cm, and a weapon hit that
interrupts a swing (a 600 ms recoil when the striker's chain is its
primary, or a stab's or overhead's recorded `round(5√impact)` with the
strike chain cleared; `part::hard_impact`), and the immediate counter a
clash may start (`part::immediate_counter`: the basic attack for motions
5–7, the combo for 8–10). A striking part's contact is classified as the
damage routine's kind 1, 2 or 3 (`part::classify`: a muscled limb, a
weapon, a shield, with its pending-hit, arm, once-a-swing, parrying and
countering guards; a parrying struck character always takes the clash),
on the striking side only (`part::faces`: local y not below 0, NaN
passing, unless unarmed or chained). A wound adds nothing over time: the part slot that
once was read as a bleeding tally is the burn's dose, which only the
particle body-contact pass feeds (`exa_combat::burn::Burn::add_dose`).
A dazed or unsteady body's targets give way to the pose it holds by the
original's weight (`body::Drive::hold_weight`), which the character tick
hands to the sampler; the race's clip rates (`body::RaceMuscles`) set its
clip clock.
An ordinary rigid body's collision sound is the original's slot 38 base
rule (`part::base_impact_sound`, read by a striking item's non-striker
contact and, through `body::impact_sound`, by every prop): above 0.5 and either past its clock or
over twice the last strength, the volume `1.5·min(0.01·S, 1)` (times
`8·mobility + 0.2` under 0.1), the pitch `(0.02·mobility + 0.9)·(0.6·v +
0.7)` (or `7·mobility + 0.3` under 0.1) and the clock `now + 220 −
round(170·v)` plus a draw below 100 ms, made only when it rings.

A held item's body (`grip`) is joined to the hand that holds it by the
grip table's link records, in the original's order and numbers: the second
hand's and the arm's links first (built disabled, the second hand's points
× the character's size, the arm's hand-side point raw, the second hand's y
by W24), then a two-hander's stab link, then the main links (points × the
body scale, the hand-side point the item's node matrix into the hand,
slack from the table, the second link's smoothing 0.01, every other
runtime link the init's 0.001). A shield gets its three pivots (the back
point to its hand's forearm, in the forearm's frame; both face points to
the left hand whichever hand holds it, that frame less the left hand's
rest translation, with no turn) and a soft world link; a torch, attached
outside the table, gets its own two pivots at (0, −20, 0) and (10, 0, 0) cm
and its soft world link at the item's origin. The grip's
modes (`apply_mode`, `mode_from_clip`, `set_mode`) switch the second-hand,
arm and extra links by the original's two callbacks (the two-hander's turns
mode 2 into 1 for a lit burning blade), the polearm's main
link slides at 15·scale cm, `relax` takes the post-change slack off the
links enabled once, and `flip_step` turns the flip point about the item's y
by `f·π` while a flip is active and the character is not attacking away
from its middle; either end clears the active flag, turns the item's
flipped state and marks its record for a rebuild (the middle and end tests
are double compares, as the original's). A flip command is refused while
the character switches and for a section with no flipped-side kind byte
(`GripLinks::begin_flip`), as the original's command does. Neither the
collapse nor death touches a held item's welds: death's strength-1 write
is to each lower leg's limit link (`Ragdoll::go_limp`) and a gobbler
collapse's 10 cm slack to each femur's (`Ragdoll::slacken_femur_limits`),
both on the leg parts the original keeps for the feet. Emptying the hands
restores each hand's link-mobility and contact-mobility words
(`empty_hand_words`: 0.175 × and 1 × its base); the equip refresh then
leaves an armed character's hands at their base and gives an unarmed one's
half, a quarter with heavy claws (`refreshed_hand_mobility`,
`unarmed_hand_mobility`), and an attack start in the heavy-claws state
quarters them (`attack_hand_mobility`). The body word is the hand part's
contact mobility (`Ragdoll::contact_base`, `Ragdoll::set_contact_share`),
written by `exa_character::body_words`. `GripLink` is plain data
with the law documented: a correction is shared by the item's and the
hand's particle weights and mobilities; a world link moves only the item.
`grip_solve` runs the records as a cross-body solver stage: a held item is
its own `Ragdoll` (one part per stored rigid node), pulled every iteration to the hand, second
hand and arm by their mobility shares with the body's own limp link law (a
slack link's smoothed length kept from tick to tick) whether the holder
lives or not (the original's held item never takes the living-part bit, so
its links always run through the plain iteration), a world weld (no pivot
flag) by the plain link law (`body::plain_law`, shared with `WorldLink`:
`strength·d`, a `10·strength²` cm move at most), and kept off its
holder's body and the other items of the same character (the original's
all-ones group mask, as `Ragdoll::held_by`). Its generic world weld is the
stiff origin link, plus the soft (parry) weld when the holder may parry,
0.3 of the node box's top above the grip (at the grip for a two-hander of
the exact class 0x30). The character's equip and the torch
light a held item's body and its `HeldGrip`. The grip builder keeps where
the item sits under its holding hand (`grip::HeldRest`); after a move of
its holder (a `grip::ReplaceHeld` on the holder's body: the player's
ground placement, a Long Door) `grip_solve::replace_held` puts every item
back there at rest, turned half a turn if it was flipped since, wakes it,
and gives an armed weapon's enabled second-hand link its new span as slack
and held length (logged at info, "held re-place: …").

A link without the runtime pivot flag is a **limit link**
(`body::MODEL_LINK_PIVOT`, marked through `Ragdoll::set_link_flags`): the
living iteration compares the link's slack with the gap's square
(`|d|²·100 > slack` in metres, as the original's centimetres), moves the
part alone by the strength times the gap, and cuts the move to 10 cm (2 cm
for a `MODEL_LINK_HORIZONTAL` link, which ignores the vertical y gap and
runs only in the tick's first two passes). A NaN gap is not "inside the
slack"; its non-finite move is dropped. The original's skeleton
registration sets the pivot flag on every link of a parented part (the
shipped skeletons store the word as 0), so a character body has no limit
links and the game does not call `set_link_flags`; the law is kept tested
for the root world links that clear the flag. Limit links are left out of
the anchors, and a limp body skips them (the original's limp solver has a
separate limit law of its own, not ported).

The mesh voxeliser (`voxel`) keeps the original's occupancy map of a
rigid-body mesh (`VoxelMap`: the 0.02 resolution, unit scale and caller's
tag; the box with a one-cell border, one word per cell, Z fastest; two fill
bytes per cell; the extracted spheres; the array release that frees the
cells and fill bytes but keeps the spheres) and the two routines that
sample a mesh into a fine grid. The load-time rules sit beside them: a
floor-set child gets generated points when bit `0x10000` or bit `0x8` of
its second flag word is set, and `generated_surface_points` runs the
surface generator in the original's centimetres at its runtime parameter
20 (a 4 cm step) and returns points in the mesh's space, answering its
sphere queries through `MeshQuery`, a triangle tree that gives the plain
closest-point query's answers. `volume` is the
dynamics volume: the
shape builders' fit over that map, in their order (`volume::fit`). The
grid is the mesh's stored box: the longest half extent gets
`round(half · resolution)` cells (ties to even), made even, at least 6 (8
with the builders' quality flag) and at most 96; the other axes round
their proportion up, made even when asked. Cells within half a
longest-axis cell of the surface are marked, then empty cells with at
least five marked neighbours whose offsets sum to at most 3 per axis and
that touch the surface again (both radii ×√2/2 and ×0.75√2 in double
precision with the voxeliser's wide-query flag); a cell is solid when the
rays along both directions of all three axes hit a face from behind (the
−axis ray to the mesh box widened by 0.01 cm, the +axis ray 0.2 cm past
it). The neighbour scans walk all 26 neighbours (the loops test their
index against 25 after the body). The corner masks cover the border cells
too.
The volume is the filled cells times the cell volume and the centre
the original's half-offset expression. The extraction takes candidates
whose eight corners are filled, 3 or more cells in, with the radius to the
nearest cell short of eight corners; covers each candidate's `r − ¼`
disc (the squared offsets added as `z² + (y² + x²)`), sorts them with the
original's quicksort and drops, smallest first, any whose cells another
covers too (never the strictly largest), taking a dropped one's coverage
back over the window its own radius gave; covers the survivors' `r + ½`
discs and makes them spheres of `r/(1 + 0.01 r)`
cells at their corner point; and then corner spheres one cell across for
surface cells with uncovered corners, their neighbours' shared corners
taken away, pulled toward the empty side by 0.02 of half a cell per unit
(a mesh with no candidate takes the corner spheres without coverage).
`volume::sort_by_key` is the original's quicksort: its pivot halfway
between the current low and the call's fixed high, a left scan that stops
at a value not below the pivot, the left side recursing while the right loops.
The mesh queries (`volume::Mesh`, `volume::Face`) are the original's
triangle tests, with its approximate reciprocals: a sphere touches when
the plane is within reach and its nearest point (the original's region
test) is strictly inside the squared radius less 1.2e-7; a ray hits only
faces whose normal points along it. A bounding-volume tree over the faces
only skips faces that cannot answer. Of the 1503 shipped `Objlib` body
nodes, 1060 get their stored sphere lists bit for bit (the stored lists
come from two versions of the fit, one without the `1 + 0.01 r` shrink,
and most were made with the wide-query flag). `VolumeSummary` weighs a
body: mobility 10 000 over the volume, and four point masses 4 over the
barycentric weights (the original's triple products) of the centre on the
rest tetrahedron. The original has no buoyancy or drag, so
none is modelled. `floor_points` is the floor heightfield,
run on a grid cell's merged floor in the cell's space: 10 cm columns, a ray
down each with the highest upward face winning, a mark in the 10 cm cell
under it, and a point at the centre of every 20 cm cell holding four marks.
`surface_points` is the general surface generator: cells a fifth of its
parameter apart (4 cm for the original's 20, the step and the near radius
taken in double precision) are classified on or near by a sphere query,
near cells beside the surface are cleared, and a greedy 5×5×5 cover leaves
one point per 20 cm cube (`SurfaceCover::EveryCell` is the original's
other builder path, one point per cell); `shared_surface_points` is the
master-to-instances copy, and `mesh_touches` a stand-in for the node's
sphere query. `exa_world::gi_volume` runs the floor generator on each grid
cell's floor in the original's centimetres.

The three shape builders (`src/shape.rs`) fit a body to a mesh node the way
the original does: they run the volume fit, sort the extracted spheres
around the node's box centre or origin, take the node's box (a limb's cut
at its own origin so the box reaches the joint), clamp the extents (the two
smaller at least a fifth of the largest, in double precision, then six
centimetres), order the axes by extent smallest first (a mirrored body
overrides the order from its own numbers), lay the regular tetrahedron over
the box and scale and place all four points (the middle √2 sign fixes the
handedness for every axis order; a mirrored body on the +x side flips x and
y), and weigh the body: mobility 10 000 over the volume, its reciprocal as the
part's initial mass word, point masses 4 over the barycentric weights of
the volume's centre, and gravity shares 4 over each mass scaled to sum to
four. The resolution comes from the node
box's smallest half extent; the quality flag gives one voxel per
centimetre and eight cells at least; the wide-query flag reaches the grid;
a factory weapon takes its resolution multiplier, even-axis word, depth,
free-corner count, map tag and mirror byte from its definition's fit
record instead (every factory weapon shares one record: 1.1, even axes, 6
cells, no free corners, tag 2, not mirrored). Nothing calls the builders
yet: the character generator and the item factory still build their
bodies the old way.

The dynamics world unit (`src/dynamics.rs`) holds the rest of the world's
side of the solver as data and rules, checked against the original's
listing: the step driver (the first simulation whose step's end is reached
runs; more than four steps past it, its base moves so that step ends now;
otherwise it waits for the soonest end, at most 100 ms) and the closed
simulations' sweep, the clock's time scale, the default, inert and
mechanism environments, a simulation's two 600 cm grids (static nodes and
bodies: a sector's cell from its node's x and z times the float 1/600
rounded to even plus the half; a body's floored from its sector's; the
nine offsets starting at the cell itself; the flat-table shift, cell j
taking what j − offset held, with the original's strip clears), the
grid rebuild (cells emptied, the sectors' cells cached again), the node and
body tree helpers (wake down the node tree, settle with its own countdown,
freeze at a matrix, disable/enable and hold/release down the body tree,
the grab layer, held pairs, the marks), the particle matrix (the
unnormalised affine fit of the rest points), the sectors' purges (static
nodes swapped out, bodies compacted and renumbered, objects shifted, the
pair rule as the original compares it), the update's flag order (dead,
reset, scene left, shared scene, sub-scene, pending registration) before
the clock advances, the placement overlap query (bound spheres apart,
same root, layers keeping them apart or disabled skip a body; the lower
sector index first; the child bodies after) and the watch list. Nothing in
the running game drives these records yet: Bevy's fixed step stands in for
the driver and the body solver keeps its own sleep and pair state.

Props and items with a rigid body are bodies in the same solver
(`src/body_rigid.rs`, `body::Ragdoll::rigid`): built as the original
registers them, asleep (countdown 0) where the level placed them, with
their stored mobility over their material's divisor, gravity shared by
the point masses so it acts at the centre of mass (`Part::gravity`, 1 on
limbs), the stored spheres against the level (friction 0.75), articulated
children joined by their stored links and an origin pivot. The body also
keeps the original's other placements: `body::Ragdoll::place_at` puts its
particles at a node's matrix times the rest points, at rest (an item put
back into the world), `translate` moves every particle set (current,
previous and the tick's start), the drawn transforms, level contacts and
foot anchors with the body as a pure displacement (no velocity, no
strain), `push_weighted` is the original's weighted push (each particle by
the offset times its weight) that adds the offset's length to the strain
and restores the edges past 0.1 cm, and `settle_for` runs the original's
temporary state where gravity acts with no inertia (a re-placed
character). The integration keeps the velocity by the motion band's
retention (0.426, 0.25, 0.05 a second; nothing else damps a body), and the
sleep countdown is the original's: 256 on fast motion, held at 30–32 on
slow motion, two off a tick at rest. A frozen body (`Ragdoll::freeze`, the
original's state bit 0x40) skips all of that: its step puts each part back
on its previous particles and takes one tick off the countdown (floored at
1). Nothing in Exarust freezes a body yet (the original does it for a
walkable-platform node, which no shipped body has, and for an item the
cursor holds). Each root keeps the original's link-iteration count
(`Ragdoll::link_iterations`: 2 for a plain body plus one per articulated
child, 20 for a character, 10 once it dies in play; each living body tick
sets it to 5 while dormant, else moves it one a tick up toward
round((1 − 0.25·feral)·1660/83) or down to it at once,
`timers::link_iteration_step`); as in the original,
the solve runs 20 passes over every root regardless and the count only
marks `count ≤ pass` (`iterations_reached`), which nothing reads.
`iterate_links` is the original's whole-count iteration, with no caller.

A limp body's links also carry the original's other kinds. A soft weld
takes a quarter of its strength times the gap less 0.4 of its two points'
motion since the tick began, shared by 0.1/(m + m') of the two parts' link
mobilities as they stand (`Part::link_mobility`); a link with an
iteration limit runs only up to its pass; a disabled link never runs. A
rigid body's link mobility is its registered mobility; when the group masks
of an articulated tree are built (once for every body with children, its
children's passes first) each child's becomes the mean of its own and its
parent's (`pairs::blend_link_mobilities`), so a body d levels down is
blended d times; the link shares were taken before, so only the soft welds
and a later `Ragdoll::recompute_link_shares` (every enabled body link from
the mobilities as they stand) read the blended values. A limb's link
mobility is 0.175 of its inverse mass, set afresh by the character's init
after the masks, so a character is never blended. A breakable grab
(`body::GrabPull`, `Ragdoll::pull_grab`, opened by
`exa_combat::bite::grab_of`) is worked in centimetres: from a gap of at
least the general epsilon squared, a disabled struck body breaks it, a
re-anchoring grab moves its struck point onto the weapon point once,
otherwise its strain falls by (q/|d|)·2 + 1 when q = (d in the weapon's
frame)·axis + 1 is above 0 (by 1 otherwise, a NaN q too), an ordered
strain below 0 breaks it (a NaN strain pulls), and above the epsilon each
side takes its share (`body::grab_shares`, m/(m + m') and −m'/(m + m') of
the link mobilities) of strength/(|d|·0.001 + 1) in double. After a break
`Ragdoll::release_grabs` drops every breakable grab to that part, in
order, and lowers the grab layer bit on it, its parent and its children
(`Ragdoll::lower_grab_layer`) and on the weapon. Bites are not live yet.
A root body hangs from points in a
scene node by world links (`body::WorldLink`, made by `Ragdoll::rigid` from
the part's stored links, so the hung props use them). They are worked in
centimetres in the original's order: the hand's horizontal drag (twice the
strength, capped by min(3·√mobility, 5) cm, less the approach); the slider
(its parameter clamped to the travel, the correction applied twice); the
damped spring (a tenth of the strength less the approach and 0.4 of the
motion); a plane link removes 80 % of the gap along its axis first; and the
plain pin or slack link (the gap times the strength, with no slack at most
10·strength cm and beyond that its strength squared; inside a slack
nothing, forgetting the length). Each pulls only its own part through its
weights, unclamped. After the iterations a part whose tetrahedron is
inverted is rebuilt as the original does: the edges are restored, its
points go back to where the tick began shifted by how far its box centre
has come, the body matrix is built from those points and the rest basis,
each axis that leans against the cross of the other two (dot under 0.1) is
re-formed as that cross, and the particles are put back on the rest
tetrahedron under the matrix; the velocity is left alone. `prop` registers
them from what the level loader gives (`prop::PendingProp`, `prop::BodyNode`)
once transforms propagate (a held item whose holder has no body yet waits for
it, so it is built at its grip under the hand the body placed; the grip builder
likewise waits while the holder key is the placeholder, and logs at debug a
grip tied, "grip: … tied to its holder", and let go; a held item's first
link is the attach's origin pivot, its origin to its place in the hand's (a
shield's forearm's) frame, as the original's register-under-parent adds it,
so the grip's two table points do not leave it a free turn; the
`exa_physics::grip_trace` log target at debug writes every held item's grip
after each tick: mode, each link's state, slack and span, and the item
against its hand); the static build keeps their meshes as
stand-ins moved with them (`prop::StandIns`; `contact::Level::movable`) for
the character capsules, which bodies skip: a prop body meets the level, and
characters' parts, corpses and other props through the pairs, by its
mobility and the group masks of its tree (`Part::rigid`'s pairing,
`pairs::mask_groups`); a body below −50 m is removed. Only awake bodies are redrawn between ticks,
at Bevy's fixed-step overstep fraction; the original's node-clock fraction
(`body::clock_fraction`: 1 − (past − offset)/period held at −2, or
(ahead + offset)/period + 1 within a tick, else offset/period + 2) and a
body's tick lag added to it (`body::body_draw_fraction`, 0 at rest) are
tested functions with no caller.
The hand's grab (`body::Grab`, `Ragdoll::grab`) is the original's horizontal
drag link: a limp part's point pulled across the ground toward a world
anchor in the passes it runs in, twice the strength times the gap counted at
most 3·√mobility (≤ 5) cm, less the motion already heading there. `Ragdoll::turn_about` turns a held body's parts about
a world point and places them there, as the item cursor's turn writes the
held frame.
`Physics::fixed_pick` and `prop::StandIns::owner_of` name the prop a ray
meets.

A root body whose stored links hang from the world keeps them: `Ragdoll::rigid`
turns each into a `WorldLink` anchored in the node's parent frame (the
parent point plus the node's translation, through the parent's world
matrix), with the origin pivot added unless the first link is a slider, and
marks the part as skipping level detection. The links are solved unshared
with the body's links every iteration, by the original's world-link law: a
plain pin (strength, capped at 10·strength cm beyond 10·strength), a slack
link, a slider (its fraction of the travel projected on its axis, the
correction applied twice), a horizontal drag (capped at min(3·√mobility, 5)
cm less the motion already toward the anchor), a damped spring and a plane.

The level's mechanisms are in `door`, `lever`, `plate`, `relay`, `signal`
and `mechanism`. Their rules follow the original's instruction listings,
including which thresholds and products are taken in double precision,
and are unit-tested; a hinged door's leaf runs them live (see below). Lengths
are in the original's centimetres, as the body links store them.

A door (`src/door.rs`) reads its record's flags, key and four sound
letters. Its links decide its kind: a slider (flag 4) travels at the
fraction of the first link whose body point is 30–160 cm up. A hinge's pins
(slack ≤ 30 cm, farther than 1 cm) give its axis, straight up by default;
its last longer link is the stop, radius r and half swing 2·asin(slack /
2r) with the engine's fast arcsine, and any stop on the positive side makes
the swing sign negative. The frame's zero is the stop's anchor, so closed
is the half swing and the range is −2·half swing times the sign. The step
runs the signalled wait (mode 3, 120 ticks) before anything else, then the
creak (stop at squared 0.015; volume 0.075·√speed, pitch 0.022·speed +
0.7), the stop knock past the link's slack, the unlock timer (1000 ms,
strictly past), the angle, the motor (a plain step |error/range|·0.001 +
0.0003, a motor's 0.0003, a fast one's 0.005; toggles arrive within their
step, an open within 0.1 rad) and the shut/open state. Passing closed shuts
and slams, sets the pull count to 20 and bars a latching leaf. A shut leaf
rises by 1 to 20 and a held, unbarred one short of closed loses 4, opening
below 0. A single leaf leaves once it is more than 0.05 rad short (a double
compare), a paired one when its partner is not near. A frozen slider keeps
its parked travel. Open shoves a hinge 3 cm, jumps a plain slider to 0.6
for 80 ticks, drives a motorised one for 100 or 300 ticks. Toggle chooses
by (angle − closed)·−0.7 or 1 − travel against 0.7. The stored word is
written only by commands: the mode in its low byte, the countdown in its
top byte.

A lever (`src/lever.rs`) reads its operative record's targets, flags and
auto-return time (×100 ms), and finds a linked light under its node when
its kind is 6; registration finds a light node at or under the arm node's
parent (`PendingLever::light`) and hands it to `Lever::configure`. Its last
pin (slack ≤ 1 cm) gives the axle and its longest
link the arm; the frame's zero is the arm's anchor and the detents are at
−α and +α, α = 2·asin(slack / 2r). The step drives the arm 0.02 rad toward
the hand (held), the off detent (forced), its home (returning) or the
nearest detent (half to even), holds within 0.02, and on arriving more than
0.1 from the last report signals the rest flag's value, arms a return or
stops one. A report of 0 fades a linked light in and 1 fades it out. The
return's own tick counts the remaining time into the object word's high
half and, at the deadline, aims home while `leverb_tck01` fades from 0.1 by
0.01 a tick.

A push surface (`src/plate.rs`) reads its two targets and flags (kind 2 or
bit 8 inverts, bit 2 one-shot, bit 0x10 second target), takes mobility 0.3
or 0.03, and steps every travel up by 0.02 pressed or down by 0.01 free,
the mean back into every travel. The grind speed is 0.9·|Δmean| + 0.1·old
(volume (50s)², pitch 20s + 0.8, the body kept awake). Above 0.7 it presses,
clicks and signals 1 (or −1); below 0.5 it releases and signals 0 unless
inverted. Its state is the object word's low byte as stored.

`mechanism` holds the operative records and the dispatcher: a door record
wins (value −1 toggles by latch, otherwise inversion then 0 close-and-lock
or 1 open, the node's word taking bit 0 for a shut leaf); class 6 starts a
deferred fire or light job, class 7 a relay of kind 0, 1, 2, 3 or 5
(`relay` combines, broadcasts, forwards both, combines and forwards, or
counts a sequence), class 5 forces a push surface. At load an operative
switches its node's effects off when flagged and still runs its kind's
setup: a light job whose value is odd starts an electric object, and a fire
record lights a fire of the size and lift in its first target word unless
the daylight is above 0.12, when its light is put out. The mechanism
environment is the original's: retention 0.001 per second in every band,
no gravity, thresholds 40 and 20 cm/s. `signal` is the single-order summary
and `door`'s free rule helpers (`creak`, `pull_closed`, `slide_step`,
`motor_step`, `toggle`, `signal_open`, …) remain for callers that hold one
leaf's summary.

Wiring: a placed object keeps its door and operative records
(`exa_world::mechanisms::MechanismRecords`). A hinged door's leaf registers
hung (its pin and swing stop as world links, the origin pivot) with a
`body::PendingDoor`, and `body::DoorBody` (`src/body_door.rs`) runs the
original's door set-up on it (mobility 0.05; the door frame from the hinge
axis and the stop's tied point in the parent node's frame; the handle 150
cm from the hinge at the node box's centre height; the placement's stored
word; a sibling leaf as the partner, both with pair layer bits `0x3000`)
and, each awake tick after the solver, the door's step: the handle's
motion smoothed by half as the creak energy, the stop's measured length as
the knock travel, the stop's heading in the door frame as the angle, and
the drive angle moving the stop point onto the stop's radius by its
weights. The hand's door link (slack 1 cm, strength 0.003, the first three
passes) is `DoorBody::take_hold`, `pull_to` and `let_go`. The door's
sounds go out through `mechanism_sound`; the last-pusher stamp is not
reproduced. Its navigation
passage report is not made: the original's pathfinder ignores it (it
returns itself; the fallback's slot is empty), so a door never changes the
grid's cells. Hung props (lamps, chains, wall rings) also register, with
their world links.

A lever's arm registers hung with a `body::PendingLever`, and
`body::LeverBody` (`src/body_lever.rs`) runs the original's lever set-up
(mobility 1; the mechanism environment, `Ragdoll::enter_mechanism_environment`:
no gravity, 0.001 of a part's velocity kept a second in every band; the
swing from the links, the last pin farther than 1 cm the axle and the last
longer link the arm; the lever frame in the parent node's frame, its up
row the axle turned by the node's rotation and negated, its forward row the
arm's tied point less the node's position with its part along up taken
off, the first detent that point's heading less α; a saved return re-armed
from the word's bits 16 and up) and, each awake tick after the solver, the
lever's step: the arm point's motion as the creak velocity, its heading in
the frame as the angle, the drive moving the arm point onto the arm's
radius at the step's angle. A detent's report writes a
`mechanism::MechanismSignal` per target; an armed return counts down every
tick, asleep or not, and wakes the lever when it fires. The wheel drag is
`LeverBody::take_hold`, `turn_to` (two planes through the node, across the
axle and across the arm; the clamp `body::lever_body::swing_clamp`) and
`let_go`. A push surface registers with a `body::PendingPlate`, and
`body::PlateBody` (`src/body_plate.rs`) sets its mobility (0.3 or 0.03) and
the mechanism environment and runs `plate_step` on its slider links'
fractions (smoothing 0.1 each tick); `PlateBody::push` is the
interaction's and a class 5 signal's press. A sliding door's leaf
(`body::PendingSlide`, `body::SlideBody`, `src/body_slide.rs`) runs the
door's sliding step on its travel link's fraction, writing a command's
fraction into the link; it registers only when asked
(`exa_world::bodies::MechanismBodies::sliding`, `--sliding-doors`): the
shipped leaves creep up to 2 cm in two seconds at rest along their level
rails and the slider law rewrites a command's fraction every pass, so a
plain open only shoves the leaf (see the module notes). Each body's tick
sends its sounds as requests (`mechanism_sound::MechanismSounded`): the
door's creak voice (`door?_hng`), knock, slam (`slm`) and unlatch (`opn`),
the lever's creak (`leverb_mov`) and return tick loop (`leverb_tck01`), the
plate's grind (`Stnb_mov`) and click (`Stnb_fit`), with the original's
points and draw order (`door::Sound::levels`); `exa_audio` plays them.

The debug tools' measures are in `body::inspect` (`src/body_inspect.rs`):
each joint's turn from its rest turn (stored when the body is built,
`Ragdoll::with_rest_turns`) split into a swing of the bone axis and a twist
about it, its distance from the clip's turn, each link's gap against its
slack, the range a joint's slack links use, its pivot's gap, and where
parts, links and anchors are drawn. `body::DealtShoves` keeps the blows the
last tick dealt. Nothing calls them unless a tool is on
([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)).

An effect pushes a part at one of its spheres with `Ragdoll::push_sphere`:
each particle moves by the push times its weight of the sphere's centre,
unclamped, and past the push strain the part's edges are restored, as the
original's push with its strain flag does (the force barricade's shove;
[`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md)). `Balance::set_counter`
sets the instability counter (the barricade sets it to 100).
`Ragdoll::push_part_weighted` pushes a part at given particle weights (a
force blast's contact, weighed at the chunk), counting the push's length
toward the strain the same way.

## Scope

- The mesh voxeliser and its voxel map (`voxel`): the floor heightfield and
  general surface generators, their fine cell grids, rounding and ray and
  sphere queries, and the map of a rigid-body mesh.
- The dynamics volume and the shape builders' fit (`volume`): the cell
  counts, the near-surface and six-ray solid fills, the corner masks, the
  candidate and corner sphere extraction with its coverage pruning and
  fallback, and the volume, centre and point masses that weigh a body.
- The three rigid-body shape builders (`shape`): the sphere sort, the box
  cut and growth, the extent clamps, the axis order, the tetrahedron with
  its mirroring, the point masses and the setup's gravity shares, and the
  quality, wide-query and factory-weapon fit flags.
- The dynamics world unit (`dynamics`): the fixed-step driver and its
  backlog drop, the clock's time scale, the environments and their bands,
  the two 600 cm grids and their shift and rebuild, the node and body tree
  helpers, the particle matrix, the sectors' purges and the update's flag
  order, the placement overlap query and the watch list (as records, not
  yet driven by the game).
- A root body's world links (`body::WorldLink`): a pin or slack link, a
  slider with its travel, a horizontal drag, a damped spring and a plane,
  anchored in the node's parent frame, with the origin pivot and no level
  detection for a hung root.
- The cloth's start seeds its fields, installs an identity matrix and derives its step from the particle count; its frame swap rotates three vertex buffers.
- Activation sets the cloth's flag words, phase and 0.4 scale; the transform visits all three particle buffers plus the object's origin and extent.
- The rebuild copies the origin, rejects a mismatched override and transforms every particle point into a fresh submitted buffer.
- The sync skips its transform and pose on the meta bit 0x10.
- The push pass after the body stage (`cloth_body::apply_pushes`): each buffered push moves its vertex out after stopping or shortening its travel across it, and holds it on its plane through the remaining constraint passes.
- The destructor frees four pointers, the root origin, three managed fields and the list before the base and final calls.
- The body collision calls back for accepted hits and pushes resolved particles to the body's radius surface (the per-particle mode; the shipped cloth's triangle contact is `cloth_body`).
- The clone creates a new cloth with four copied fields.
- The hand's grab (`Cloth::grab`/`release_grab`): one vertex is put on the hand's target after each tick's links, zeroing its velocity there, and wakes the cloth; letting go leaves it to its links.
- The cloth helpers the step does not call yet: the bounds midpoint the particles' component extremes (vector min and max semantics) and floor the half extents at 1e-4 cm; the swept-particle query over an index range, with the closest-approach slack that goes negative outside the tick's motion; the blood painter, which draws the engine's generator and lowers the vertex alpha with a ties-to-even conversion; and the owner-move blend, which writes `(d + (target - d) k) weight + offset + base` into a third buffer.
- Fixed-timestep deterministic simulation set, independent of frame rate.
- The original's shared intersection routines (`intersect`): sweep set-up, point/segment/sphere/box distances and tests, the swept-sphere slab tests, segment/oriented-box separation, point-set bounds, and the convex-volume box tests, refresh, polygon clipping, plane cut with cap and directional shadow volume, in the original's arithmetic.
- Rigid bodies (props, doors, items), collision against level geometry and convex hulls.
- Articulated character bodies driven by joint motors toward animation targets (active ragdoll / 'motile' dynamics).
- Cloth and padded-armour layers (original engine has cloth classes per material: basic, fine, padded, silk, velvet).
- Decision recorded in docs/plan: rapier3d for level colliders and the character controller; the
  body solver and its level contacts are in-house, as the original's (`docs/formats/RSQ.md`).

The Verlet step adds the kept motion to the position and then the gravity
displacement, as two rounded sums (the original computes the displacement
into its own buffer and adds it), and the edge restore takes an edge's
length with the squares summed x, then z, then y, its correction
`(len − rest)/(len + len)`. A body that falls asleep empties its motion and
smoothed buffers, as the original's post-step does.

The body kernel adds links at run time as the original does
(`Ragdoll::add_link`): a point on each part becomes weights over that
part's rest particles, a body parent takes the two parts' mobility shares
and the body and code flags, a slack at or under the general epsilon (in
centimetres) or a NaN stores zero, the record's smoothing starts at zero
and the length the routine measures is overwritten with zero. A link to a
scene node is `Ragdoll::add_world_link`, a plain world link with the code
flag and a world anchor. A clone copies its rest particles; the original's
clone slot keeps the part states and axes, shares its template's
collision-sphere array (copying it only before a write) and copies no link
that code made at run time or that can break (its flags' `0x401000` mask,
which drops every origin pivot, since the copy re-runs its own setup). That
slot is `Ragdoll::runtime_clone`, which remaps the indices of the links it
keeps so a leg's limit link follows its link; no run-time caller exists, so
the tests and tooling use the whole-state `Clone`, which shares the sphere
array the same way. The body also keeps the particles where the last
post-step left them (the original's third particle set, rolled each
post-step, on the first tick of sleep and in the muscles-off snap), so
`Part::two_step_point` gives a point's displacement since then (two ticks
mid-step, one between ticks). Between ticks the original keeps one set for
where a body was a tick ago, which its Verlet step's velocity and the
two-step point both read, and every particle writer outside the step and
post-step (the placements, the freeze, the held items' re-placing) keeps
the previous set equal to it; so `Ragdoll::set_velocity` writes both of
Exarust's stand-ins for that set; `Ragdoll::root_moved`, which the locomotion
stage and the lift servo read, is the two-step point of the root's own
origin, as the original's character tick reads it; and its tree-wide take of sleep ticks (`Ragdoll::take_ticks`,
holding the count at one) that the frozen step and a dormant living
body use. `Ragdoll::set_disabled` sets the original's state bit 0x800 (an
item on the cursor): the pair stage and the level detection skip the body
and a grab on it breaks.

The body kernel also keeps the original's class and state words
(`Part::class_word`/`state_word`): `Ragdoll::set_class_word` writes the
class word to every part and `word | 0x30` to each first child,
`raise_grab_layer` adds the bite-grab's 0x40 to a part, its parent and its
children, and `raise_state_flag` walks the whole tree. Death's class
writer is `pairs::set_leg_layers` instead (layer 0 on each lower leg, 0x30
on its foot), and `Ragdoll::clear_death_strikes` (run by the collapse's
start and by death) clears the strike chain and the world mask word only on
those feet. A gobbler's collapse gives each femur's limit link 10 cm of
slack (`Ragdoll::slacken_femur_limits`). The revive of a corpse
(`Ragdoll::revive`) marks every limb below the pelvis muscled with its
contact base, gives the feet the thin-plate bit and the 0x30 layer (the
lower legs keep death's 0), turns the muscles on and knocks the body out;
it has no running caller yet (Raise Dead and the undead's rise are not
wired). `Ragdoll::contact_base` and `set_contact_share` read and write a
limb's contact mobility from its base (inverse mass × 2/3). `push_at` applies a
push at a world point with the original's weighting (`push_fraction`, the
`4·dot` test against the push's squared length). The stagger helper
(`stagger`, `recoil`) gives a blow's stagger speed, decay, flags and
countdown;
`femur_posture_turn` is the thigh posture's alternating 5° (the state
word's splay bit, which no shipped code writes) or (1 − height)·−1.5°
(the behaviour's in-combat flag, on its feet and not walking), applied by the sampler to the clip's
thigh turns; and `MOTILE_ENVIRONMENT` is the class init's unused
0.2556/0.15/0.03 environment. `full_tone`/`clear_foot_state` are the
original's resets.

`mesh_tree::ray_mesh` is a node mesh's ray query (first front face, through the
tree or every triangle of a small mesh), used by the hand's pick of bodies with no
stand-in collider; `mesh_tree::sphere_contacts` is its sphere query (each front
face the sphere reaches hands its closest point to a callback that may move the
sphere). `mesh_query` holds the mesh's per-leaf effect tests the kd-tree
installer sets beside the pair stage's: the two-sided sphere test (a
force blast's, wave's and torrent's part and level contacts), the front
sphere test and the swept test (the bolt's sampled walk). `mesh_tree` puts a body node's CPU triangles behind the tree as the
original does: a node mesh of more than 10 and fewer than 100 000 triangles
gets one (`takes_tree`, the scene loader's queue rule), a shape builder's
fitted mesh (a limb's or a weapon's) one at any count (`shape_tree`). Trees
are built in the original's centimetres over the part's frame, shared per
triangle list, and queried by `KdTree::sphere_leaves` (the sphere's box
walk: the second child alone when the sphere is past the plane, both, the
second deferred, when it reaches it, last in first out) and
`KdTree::sweep_live` (the swept walk with the record, `SweptRecord`,
refreshed by a hit's callback and read live by the descent), the ray walk
for picks and the nearest swept column for the Height Query
(`body_drop`). `sector_lists` keeps the sector's simulation lists in the
running game: `SectorBodies`, every body part in the 600 cm world cell of
its mesh box's world centre, rebuilt after each body step, and the static
grid's records with the mesh each was built from
(`StaticColliders::mesh_of`, `in_world_cell`). `MeshLevel` is the Height
Query's mesh mode (a grid scene's): the level's answer and those of the body
meshes listed in the nine cells around the query (`height_meshes`: not a
body out of the world, not a living limb, a dead body's limbs kept), the
highest winning; the player's ground placement uses it.

`kd_tree` is the original's triangle kd-tree (`TkdTree`) and its terrain
block subclass (`TTErrainkdTree`) as a pure model. The node is the
original's 8 bytes (a leaf flag plus triangle count, or the axis plus the
child byte offset, and an f32 plane or an integer start into a separate
leaf-index array). The build walks a work stack: the root is bounded by the
mesh box padded by 0.01, each cell's triangles are clipped to it
(Sutherland-Hodgman) before their bounds are taken, the split plane is the
cheapest of the 15 steps at 1/16 of the axis by the surface-area estimate
(or the middle of the longest axis above 512 triangles), and a child becomes
a leaf at 8 triangles or a 10 cm extent. Queries are the box-clipped
nearest front-face ray (`ray`), the both-face segment with the far end
included (`segment`), the boolean ray that projects onto the dominant plane
(`ray_any`), and the swept-sphere traversals (`sweep`, `sweep_nearest`),
which take the mesh-side leaf test as a closure because the original
keeps it outside the tree. `TerrainKdTree` is the 30 x 30 cell block tree:
leaves list two triangles per cell, the range is packed in the root word,
a block's translation moves only the split planes, and an attach caches the
mesh's box as its centre less and plus its half extent (a clone owns copies
of the arrays where the original shares them; the planes are only ever moved
after the copy). Assumptions (the
stored triangle normal's orientation and the standalone root box) are in
the module docs. The game still takes the level's collision from rapier's
trimesh BVH; the body meshes' trees (`mesh_tree`) and the terrain
manager's block trees are its game paths.

`controller` and `sim_object` are the original's two abstract bases: the
physics controller (kind word, attached node, flag reset that clears bits 0
and 3, no-op step/post-step/wake/translate/interpolate slots, a false slot,
a float pass-through slot and a render fraction of 0.0) and the non-rigid simulated object (an empty
after-collisions stub plus the static/body/object contact hooks). `dynamics`
gains the environment's per-tick conversion (`configure_step`: retention
raised to 1/83, gravity over 83², the squared per-tick speed bands; a
retention that is not above zero, a NaN included, stores zero), the
simulation constructor's constants (12 ms step, 11-step backlog), the
per-frame interpolation fraction with its [−2, 2] clamp, the scene's
sector creation (a new sector with no static nodes yet hands its node's
children back for immediate registration; a shared scene reuses the first
sector for every node of the batch, keeps its scene word and node, and
queues the children as pending) and the sector's
pending-batch append.

Fixed-tick cost: `body::first_by_key` finds the holders of held items by key (the first part's entity) in one walk that ends once every key is found, with the answer a `find` per item gives; the pair stage tests a sleeping part's cell (`Part::cell`, the pose's cell) before building its pose. `EXA_SWEEP_STATS=1` logs, every 83 ticks, how many characters the controller swept and the colliders and triangles within their capsules' boxes (the sewer's player follow meets about 4000 triangles a tick); `EXA_FIXED_EXECUTOR=single` runs the fixed step's systems one at a time.
