# `exa_physics`

Rigid-body and articulated-body simulation: the core of Exanima's feel (physically driven characters, momentum-based combat, cloth).

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started — the original's
body solver (`body`), its level contacts (`contact`), props and items as bodies (`prop`), a cloth step (`cloth`), the
character controller (`lib`) and the shared intersection maths (`intersect`).

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
equalises the two bodies' sleep countdowns (`body::equalise_sleep`).
Blows are queued for the next tick, and parts are drawn between their last
two ticks (`body::BodyPlugin`). Every part is
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
`turn_rate`/`turn`: the body turn by the original's table; `align`: the
alignment of the pelvis with the heading; `body::locomotion::Locomotor` keeps the body's side), which
its capsule carries out until the body walks by its carry
(`capsule_velocity`, `CapsuleGaits`; `src/body_locomotion.rs`). The pose is put on the original's character
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
A dazed or unsteady body's targets give way to the pose it holds by the
original's weight (`body::Drive::hold_weight`), which the character tick
hands to the sampler; the race's clip rates (`body::RaceMuscles`) set its
clip clock.

The mesh voxeliser (`voxel`) keeps the original's occupancy map of a
rigid-body mesh (`VoxelMap`: the 0.02 resolution, unit scale and caller's
tag; the box with a one-cell border, one word per cell, Z fastest; two fill
bytes per cell; the extracted spheres; the array release that frees the
cells and fill bytes but keeps the spheres) and the two routines that
sample a mesh into a fine grid. `floor_points` is the floor heightfield,
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

Props and items with a rigid body are bodies in the same solver
(`src/body_rigid.rs`, `body::Ragdoll::rigid`): built as the original
registers them, asleep (countdown 0) where the level placed them, with
their stored mobility over their material's divisor, gravity shared by
the point masses so it acts at the centre of mass (`Part::gravity`, 1 on
limbs), the stored spheres against the level (friction 0.75), articulated
children joined by their stored links and an origin pivot. `prop` registers
them from what the level loader gives (`prop::PendingProp`, `prop::BodyNode`)
once transforms propagate; the static build keeps their meshes as
stand-ins moved with them (`prop::StandIns`; `contact::Level::movable`) for
the character capsules, which bodies skip: a prop body meets the level, and
characters' parts, corpses and other props through the pairs, by its
mobility and the group masks of its tree (`Part::rigid`'s pairing,
`pairs::mask_groups`); a body below −50 m is removed. Only awake bodies are redrawn between ticks.

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
`Placement::Mechanism`): their bodies hang from the world by links and the
solver's world-link law is not implemented, so nothing produces the
heading, handle speed, travel fraction or arm angle these steps read.

The debug tools' measures are in `body::inspect` (`src/body_inspect.rs`):
each joint's turn from its rest turn (stored when the body is built,
`Ragdoll::with_rest_turns`) split into a swing of the bone axis and a twist
about it, its distance from the clip's turn, each link's gap against its
slack, the range a joint's slack links use, its pivot's gap, and where
parts, links and anchors are drawn. `body::DealtShoves` keeps the blows the
last tick dealt. Nothing calls them unless a tool is on
([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)).

## Scope

- The mesh voxeliser and its voxel map (`voxel`): the floor heightfield and
  general surface generators, their fine cell grids, rounding and ray and
  sphere queries, and the map of a rigid-body mesh.
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
