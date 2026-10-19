# `exa_physics`

Rigid-body and articulated-body simulation: the core of Exanima's feel (physically driven characters, momentum-based combat, cloth).

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started — the original's
body solver (`body`), its level contacts (`contact`), props and items as bodies (`prop`), a cloth step (`cloth`) and the
character controller (`lib`).

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
its feet (`src/body_lift.rs`). The pose is put on the original's character
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

The debug tools' measures are in `body::inspect` (`src/body_inspect.rs`):
each joint's turn from its rest turn (stored when the body is built,
`Ragdoll::with_rest_turns`) split into a swing of the bone axis and a twist
about it, its distance from the clip's turn, each link's gap against its
slack, the range a joint's slack links use, its pivot's gap, and where
parts, links and anchors are drawn. `body::DealtShoves` keeps the blows the
last tick dealt. Nothing calls them unless a tool is on
([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)).

## Scope

- The cloth's start seeds its fields, installs an identity matrix and derives its step from the particle count; its frame swap rotates three vertex buffers.
- Activation sets the cloth's flag words, phase and 0.4 scale; the transform visits all three particle buffers plus the object's origin and extent.
- The rebuild copies the origin, rejects a mismatched override and transforms every particle point into a fresh submitted buffer.
- The sync skips its transform and pose on the meta bit 0x10.
- The pressure pass normalizes the normals, projects each edge, couples the result and zeroes the normals.
- The destructor frees four pointers, the root origin, three managed fields and the list before the base and final calls.
- The body collision calls back for accepted hits and pushes resolved particles to the body's radius surface.
- Fixed-timestep deterministic simulation set, independent of frame rate.
- Rigid bodies (props, doors, items), collision against level geometry and convex hulls.
- Articulated character bodies driven by joint motors toward animation targets (active ragdoll / 'motile' dynamics).
- Cloth and padded-armour layers (original engine has cloth classes per material: basic, fine, padded, silk, velvet).
- Decision recorded in docs/plan: rapier3d for level colliders and the character controller; the
  body solver and its level contacts are in-house, as the original's (`docs/formats/RSQ.md`).
