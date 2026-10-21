# `exa_physics`

Rigid-body and articulated-body simulation: the core of Exanima's feel (physically driven characters, momentum-based combat, cloth).

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started — the original's
body solver (`body`), its level contacts (`contact`), props and items as bodies (`prop`), a cloth step (`cloth`), the
character controller (`lib`), the dynamics world unit (`dynamics`), the rigid-body shape
builders (`shape`) and the shared intersection maths (`intersect`).

The original's `RF_Intersection` unit is reproduced in `intersect`, a pure-maths
module that follows the original's instructions (sums in its order, the
processor's minimum and maximum, sign-bit answers, NaN branches): sweep
set-up, point–segment and point–box distances, sphere–sphere and
sphere–box tests, capsule–sphere, the slab tests of a swept sphere against
a box and an oriented box, segment against oriented box (slab and
separating-axis), point-set bounds, and the convex volumes: rebasing,
plane refresh, box tests, polygon clipping into a portal pyramid, cutting
a volume by a plane with its cap, and the directional shadow's caster
volume. Its cube-face mask lives with its only user,
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
values). Nothing in the running game calls it yet.

A subtree marked `StaticGeometry` has its meshes copied into static
triangle-mesh colliders. `NoStaticCollider` on a mesh (or subtree) under it
keeps that mesh out: the original draws terrain vegetation but never
collides with it.

Each fixed tick (83 Hz) moves the character controllers, then runs the
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
spheres against the other's, with an immediate push by mobility shares
and φ; up to four points per pair are solved in the next tick's
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
stages and budgets. The pair stage buckets parts by their mesh box's
world centre and first asks that the awake part's sphere touch the
other's mesh sphere; a small sector's all-pairs walk
(`body::Pairs::all_pairs`) adds the original's two point-to-mesh-box
tests. Not wired: the static-collider grid (rapier keeps the level's
broad phase), the overflow guard's reset, the overlap and placement
queries, and an automatic choice between the grid and all pairs.
Blows are queued for the next tick, and parts are drawn between their last
two ticks by interpolating their four particles and building the part's
matrix from them, as the original does (`body::BodyPlugin`). Every part is
pinned at its origin to its parent, and the skeleton's links bound each
joint's range (`body::Ragdoll::new`); a limp body's links also damp each
joint inside its range through their smoothed lengths, reset when the
body goes limp (`body::Ragdoll::go_limp`). In a living body every slack
link is a link muscle that turns its joint toward the clip's angle, with
the gains the character tick sets from what the character does
(`body::Bearing`), its race (`body::RaceMuscles`) and its own character's
stats from its record (`body::BodyStats`: most energy, the limbs' scale,
the head's gain and factor and the walking arms of the undead)
(`src/body_muscle.rs`);
a pose drive turns the pelvis, spine, head and feet toward the clip's
orientation about each part's own origin, scaled by the support S²
(`body::Drive`); and the carry holds the body up in place of gravity, with
a lift servo holding its pelvis at a target height over the ground under
its feet (`src/body_lift.rs`). A character moves through a locomotion request its
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
of its run blend while it stands (`stand`; `src/body_gait.rs`). The
routine also keeps the original's exertion value and the head tilt the
torso twist reads from it, the held items' grip mode, the dead jump
branch's clock decay, and the start's notice: another clip playing clears
the held items' motion flag. A body with muscles walks by its carry: the
movement stage adds the drive × S² × the gait's push column × the feet's
lag to `Drive::carry` (`Ragdoll::lift`; the lag cuts a foot that trails its
animated spot by (throw/60·scale)²), and its capsule only follows its
frame (`capsule_velocity`, `CapsuleGaits`; `src/body_locomotion.rs`). The
move branch sets the held items' static-only flag (their level queries
never sweep) and leaving the walk plants the rearmost foot at its stance
spot under the frame (`Ragdoll::plant_trailing_foot`). A stepping foot that
is anchored blends its leg limit to 0.2, and a powered golem suit halves
the standing thigh factor and quarters the limit's target
(`src/body_muscle.rs`). A behaviour's look target (`Locomotion::look_target`
and `look_until`) turns the head while its deadline is in the past. The
pose is put on the original's character
frame, whose origin follows the pelvis by its rules: on the pelvis while
moving, frozen while standing steady, behind dead-bands of 5 and 10 cm
otherwise (`src/body_frame.rs`, `body::Drive::anchor`). The controller
carries a standing body only across the ground and turns it, and follows
the frame (`CharacterBody::follow`, a tenth of the way a step,
at most 2 cm), leaving its follow out of the carry. In each solver
iteration the support pins (`src/body_pins.rs`) pull the two thorax pins
and the pelvis pin horizontally toward the frame's targets, clamped near
the feet centre. Each foot's ground is its own: the controller's floor
does not raise the lift (`src/body_lift.rs`). A standing leg bends onto
its target by the law of cosines, at most 0.02 rad a tick (0.2 while a
recovery step is on), and the muscles follow that bend on the next tick
(`src/body_bend.rs`). While stepping the thigh turns 75% of the
hip-to-target angle and an anchored foot takes the step lift. Gait
plants are not in yet.
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
foot at a time (a free foot within 5 cm stays). A recovery step starts
when the chest is past 1.5 · R · E⁴ · S (45 cm for a settled human).
While it runs and the chest is not past the trigger, the thorax pins
lead by the horizontal error from the pin to the posed target plus the
step offset: 0.7, or 0.4 while Idle, times (direction + 0.5) times
(2 − 25·s) times 1.2 times the square root of body scale times the
stance level. A still 40 cm sideways error moves the target 33.6 cm.
The lower leg's and foot's friction follow its
state; in each iteration a standing
foot is gripped at its anchor (the PD grip, a walking one the velocity
servo, a lifted one only pushed up) and its toe held over the floor
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
for limp parts, the fast value for living limbs. A limp body sleeps by the
original's countdown (`body::Ragdoll::countdown`: 256, two off each tick
under 20 cm/s, held at 30–32 between 20 and 40 cm/s, back to 256 above)
and is woken by a blow, by going limp or by a contact with an awake body;
a living body never sleeps.
Each part also keeps its state word and the part's own impact and wound
bookkeeping (`part`): the class bits this crate reads (`part::PartState`:
the strike window and chain, a held item, an arm, a muscled limb, a sole,
level contacts, the living limb family), a strike marking the whole chain
under its root (`part::set_striking`, `body::Ragdoll::set_striking`), and
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
countering guards), and a boned limb's wound adds its size to the
character's bleeding tally (`part::WoundTally`).
A dazed or unsteady body's targets give way to the pose it holds by the
original's weight (`body::Drive::hold_weight`), which the character tick
hands to the sampler; the race's clip rates (`body::RaceMuscles`) set its
clip clock.
A held item's body (`grip`) is joined to the hand that holds it by the
grip table's link records, in the original's order and numbers: the second
hand's and the arm's links first (built disabled, points × the character's
size, the second hand's y by W24), then a two-hander's stab link, then the
main links (points × the body scale, the hand-side point the item's node
matrix into the hand, slack from the table, the second link's smoothing
0.01). A shield gets its three pivots (back and both face points, two to
the other hand) and a soft world link. The grip's modes (`apply_mode`,
`mode_from_clip`, `set_mode`) switch the second-hand, arm and extra links
by the original's two callbacks, the polearm's main link slides at 15·scale
cm, `relax` takes the post-change slack off the links enabled once, and
`flip_step` turns the flip point about the item's y by `f·π`. `GripLink`
is plain data with the law documented: a correction is shared by the
item's and the hand's particle weights and mobilities; a world link moves
only the item. The running game still draws held items as bone-attached
models; the records are ready for the body solver to use.

The mesh voxeliser (`voxel`) keeps the original's occupancy map of a
rigid-body mesh (`VoxelMap`: the 0.02 resolution, unit scale and caller's
tag; the box with a one-cell border, one word per cell, Z fastest; two fill
bytes per cell; the extracted spheres; the array release that frees the
cells and fill bytes but keeps the spheres) and the two routines that
sample a mesh into a fine grid. `volume` is the dynamics volume: the
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
it). The volume is the filled cells times the cell volume and the centre
the original's half-offset expression. The extraction takes candidates
whose eight corners are filled, 3 or more cells in, with the radius to the
nearest cell short of eight corners; covers each candidate's `r − ¼`
disc, sorts them with the original's quicksort and drops, smallest first,
any whose cells another covers too (never the strictly largest); covers
the survivors' `r + ½` discs and makes them spheres of `r/(1 + 0.01 r)`
cells at their corner point; and then corner spheres one cell across for
surface cells with uncovered corners, their neighbours' shared corners
taken away, pulled toward the empty side by 0.02 of half a cell per unit
(a mesh with no candidate takes the corner spheres without coverage).
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
y), and weigh the body: mobility 10 000 over the volume, point masses 4
over the barycentric weights of the volume's centre, and gravity shares 4
over each mass scaled to sum to four. The resolution comes from the node
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
slow motion, two off a tick at rest.

A limp body's links also carry the original's other kinds. A soft weld
takes a quarter of its strength times the gap less 0.4 of its two points'
motion since the tick began, shared by 0.1/(m + m'); a link with an
iteration limit runs only up to its pass; a disabled link never runs. (A
breakable grab is `body::pull_grab`'s.) A root body hangs from points in a
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
once transforms propagate; the static build keeps their meshes as
stand-ins moved with them (`prop::StandIns`; `contact::Level::movable`) for
the character capsules, which bodies skip: a prop body meets the level, and
characters' parts, corpses and other props through the pairs, by its
mobility and the group masks of its tree (`Part::rigid`'s pairing,
`pairs::mask_groups`); a body below −50 m is removed. Only awake bodies are redrawn between ticks.
The hand's grab (`body::Grab`, `Ragdoll::grab`) is the original's horizontal
drag link: a limp part's point pulled across the ground toward a world
anchor in the passes it runs in, twice the strength times the gap counted at
most 3·√mobility (≤ 5) cm, less the motion already heading there.
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
and are unit-tested, but no live body feeds them yet (see below). Lengths
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
its kind is 6. Its last pin (slack ≤ 1 cm) gives the axle and its longest
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
(`exa_world::mechanisms::MechanismRecords`), but the level loader leaves
doors, levers and plates static (`exa_world::bodies`,
`Placement::Mechanism`): their bodies would need their steps to drive them,
so nothing produces the heading, handle speed, travel fraction or arm angle
these steps read. Hung props (lamps, chains, wall rings) do register, with
their world links.

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
- The pressure pass normalizes the normals, projects each edge, couples the result and zeroes the normals.
- The destructor frees four pointers, the root origin, three managed fields and the list before the base and final calls.
- The body collision calls back for accepted hits and pushes resolved particles to the body's radius surface.
- The clone creates a new cloth with four copied fields.
- Fixed-timestep deterministic simulation set, independent of frame rate.
- The original's shared intersection routines (`intersect`): sweep set-up, point/segment/sphere/box distances and tests, the swept-sphere slab tests, segment/oriented-box separation, point-set bounds, and the convex-volume box tests, refresh, polygon clipping, plane cut with cap and directional shadow volume, in the original's arithmetic.
- Rigid bodies (props, doors, items), collision against level geometry and convex hulls.
- Articulated character bodies driven by joint motors toward animation targets (active ragdoll / 'motile' dynamics).
- Cloth and padded-armour layers (original engine has cloth classes per material: basic, fine, padded, silk, velvet).
- Decision recorded in docs/plan: rapier3d for level colliders and the character controller; the
  body solver and its level contacts are in-house, as the original's (`docs/formats/RSQ.md`).

The body kernel also keeps the original's class and state words
(`Part::class_word`/`state_word`): `Ragdoll::set_class_word` writes the
class word to every part and `word | 0x30` to each first child,
`raise_grab_layer` adds the bite-grab's 0x40 to a part, its parent and its
children, and `raise_state_flag` walks the whole tree. `push_at` applies a
push at a world point with the original's weighting (`push_fraction`, the
`4·dot` test against the push's squared length). The stagger helper
(`stagger`, `recoil`) gives a blow's stagger speed, decay, flags and
countdown; `step_phase` advances the footstep phase from the pelvis travel;
`femur_posture_turn` is the thigh posture's alternating 5° (splayed) or
(1 − height)·−1.5° (combat) turn, applied by the sampler to the clip's
thigh turns in the combat conditions; and `MOTILE_ENVIRONMENT` is the class init's unused
0.2556/0.15/0.03 environment. `full_tone`/`clear_foot_state` are the
original's resets.
