# Skeleton and motions (`.rsq`)

Status: **verified** (REA on the reader; `exa_formats::rsq` consumes all
729 932 bytes of `humanoid.rsq`; the walk cycle plays correctly in Exarust).
Only `Resource/humanoid.rsq` ships.

The file is a bare chunk stream (no sanity word):

```
chunk 0x3D000000  scene-node list: 18 bones (plain nodes 0x3D01, see RFC.md)
chunk 0x00003DAB  motion set
```

## Bones

Standard scene nodes. The first two characters of the name are the bone
index (`01lvis` = 01 pelvis, `08umbar` = 08 lumbar); each node carries a
physical-body chunk (see below). Hierarchy:

```
01 pelvis ─ 02 femur ─ 03 lower leg ─ 04 foot
          ─ 05 femur ─ 06 lower leg ─ 07 foot
          ─ 08 lumbar ─ 09 thorax ─ 10 cervical
                                  ─ 11 shoulder ─ 12 humerus ─ 13 forearm ─ 17 hand
                                  ─ 14 shoulder ─ 15 humerus ─ 16 forearm ─ 18 hand
```

Character models (`humanbase.rfc`, …) are hierarchies of body-part meshes
named with the same indices (`01Pelvis`, `09Chest`, `17RHand`).

## Physical body (`0x3DD0B100`)

Each bone node carries a body chunk. The node header is 112 bytes. Its
nested sub-chunks start at offset `0x5C` inside the chunk:

| chunk | content |
|---|---|
| `0xDDB0` | links to the parent part: u32 n, then n × {u32 flags, vec3 point on the part, vec3 point on the parent, f32 slack, f32 stiffness, f32 smoothing, vec3 axis if flags & 4} |
| `0xDDBC` | foot contact points (each lower leg: one, at (±0.16, −48.97, 2.4) cm) |
| `0xDDBB` | thorax pins (two shoulder points at (±10.4, 9.65, 1.25) cm) |

All values are in cm. The point on the part is in the part's own frame.
**The parent's point is that same point turned into the parent's axes and
measured from the part's origin**: for all 36 links of `humanoid.rsq` its
length equals the part's point's to 0.01 cm. On the parent it lies at the
stored point plus the part's origin in the parent, the character model's
local translation of the part (`humanbase.rfc`), with no rotation added
(REA on the original's link conversion; corpus check). Each bone has 2
links; each hand has 3. The original's 0.9.5 reader keeps flags, both
points, slack and smoothing; it replaces the stiffness with 1 and reads no
slider axis (`exa_formats::rsq` skips 12 bytes for one; no shipped link
sets flag 4).

How the original simulates a body (`exa_physics::body`):

- Every part is a rigid tetrahedron of 4 particles, kept rigid by its 6
  edge lengths. The particles start from a regular template laid over the
  part mesh's box (each extent at least 6 cm) and scaled by 1.25. The part's
  frame follows the particles through the inverse rest matrix.
- Verlet integration at 83 Hz with gravity −9.80665 m/s². Every body,
  characters living or dead included, takes the simulation's default
  environment (REA on every writer of a body's environment). A limp part
  keeps 0.426 / 0.25 / 0.05 of its velocity per second (0.98977 / 0.98344
  / 0.96455 per tick, r^(1/83)) when the farthest of its four particles
  moved over 40/83 cm, over 20/83 cm, or less in the last tick (strictly
  over). A living limb keeps the fast value × u² whatever its speed, with
  u = (1 − balance)(1 − r) + r and r the race's limb damping ratio (1 for
  humans, so 0.98977). The original also builds a 0.6× environment
  (0.2556 / 0.15 / 0.03) that no body ever uses. Exarust leaves out the
  living ×0.95 the original applies in a state of dormant statues and
  golems, and caps particles at 25 m/s, a guard the original lacks.
- Joints (`Ragdoll::new`). Every part with a parent part takes its stored
  links and then one more, made when the body is registered: a **pivot**
  from its origin (the joint centre) to its origin's place in the parent,
  with no slack. So a femur has 3 links and a hand 4. The pivot is the ball
  joint. A second zero-slack link on the lateral axis makes the knee a
  hinge. The other links bound each joint's range: a point at r cm from the
  origin may drift by its slack, about 2·asin(slack/2r) of swing (femur
  14.9 cm, shin 12.7 cm, …). There are no angle limits. The original
  scales the points and slacks by the body's scale; Exarust does not yet,
  which matters for bodies of another size.
- A correction is shared by the parts' mobilities, m/(m + m_parent), with
  m = 0.175 × the bone's value (pelvis 0.45, thighs 0.7, shins 1, feet 2.8,
  lumbar 0.45, thorax 0.5, head 1, shoulders 1, upper arms 1.2, forearms
  1.5, hands 2.5; larger moves more). It is spread over each part's
  particles by the point's weights, unclamped, so the two parts' momenta
  change by equal and opposite amounts.
- Each tick runs 20 iterations over the body tree: a living body children
  first, from its hands and feet in; a limp one parents first, depth first,
  from the pelvis out. In each, a part takes its links to its parent, then
  its level contacts from the last tick, then restores its edges.
  - Living: a pivot (slack ≤ 0.01 cm) closes d/(1 + 0.001|d|) of its gap d
    (cm), in full however far apart (past 80 cm it is reported); a slack
    link takes back its excess beyond the slack at full weight, cut to
    80 cm. The link muscles that also pull each slack link toward the
    animated angle are not reproduced yet.
  - Limp: a pivot (slack ≤ 1.19e-7 cm) closes strength/(1 + 0.001|d|²) of
    its gap. A slack link damps its length inside the slack through a
    smoothed length L: unless |d| is within 1.19e-7 cm of L, L ← s·L +
    (1 − s)·|d| and the points are taken back to the L they had, by
    strength·(|d| − L_old)/|d| of the gap d. Over the 20 passes L catches
    up, so a change of length is resisted in proportion to its speed and a
    joint at rest stays where it is, with no spring back. With every pass
    closing in full, (1 − s)/(2 − s) of a tick's stretch survives (0.20
    for s = 0.755, inferred); the points' weights spread each pass, so in
    Exarust's synthetic pelvis and femur a swinging hip keeps about 0.8 of
    its swing a tick and under 0.2 after ten. Beyond the slack the link
    takes back strength × its excess and L is the slack.
  - L starts at the link's distance in the pose the body is built in (the
    original registers it in its starting pose). Going limp sets it to the
    distance, at most the slack, as the original's death does (also its
    placement routine and one power), so a fresh ragdoll neither jolts nor
    springs back. The living iteration never uses L.
  - Each link keeps a strength: 1, except a lower leg's first link with
    over 10 cm of slack (the leg limit), 0.2 while the body lives and 1
    again once it goes limp, as the original's death sets it. The femur's
    first such link is its leg limit. Each link also keeps the original's
    smoothing (1 − (r² − 25)·0.001)², at most 0.99 and 0 within √0.1 cm, r
    its point's distance from the part's origin in cm: the shipped slack
    links take 0.640 (the humerus's point 15 cm out) to 0.914.
- After the iterations, every sphere collider is tested once against the
  level's triangles (`exa_physics::contact`):
  - A triangle pushes only from its front, the side its winding faces in
    its own mesh, and only a sphere whose centre is in front of its plane
    within the radius. The push takes the centre off the closest point on
    the triangle by the overlap, along the closest-point direction. It
    moves the part at once, spread so the heaviest particle takes all of
    it, and is not clamped (the original computes a 160 cm clamp and then
    discards it).
  - A sphere whose centre moved more than 0.1 cm² (squared, about 3.2 mm)
    since the end of the last tick is swept from there. An end that crossed
    a triangle's plane goes back to where the sphere first touched it. In
    one rare branch (the closest point beyond both ends of the sweep) the
    original scales the vector from the start, which can push by metres;
    Exarust pushes off the touched point instead.
  - Each push is kept as a contact: the touched point, the normal, and a
    plane 0.1 cm under the surface. A part keeps at most four, the four
    spread widest over its particles. They are enforced in the next tick's
    20 iterations (the point lifted onto its plane), then dropped.
  - Friction uses the part's own coefficient μ (limbs 2.5; Exarust gives
    feet 0.5 until foot planting is ported). With the limit
    T = 0.5·μ·(0.01 + 0.99·n_y²) in cm² per tick², a contact point's slide
    this tick whose square is under T is cancelled; a larger slide is cut
    by only 0.01·T of itself. Friction runs in 8 passes after detection and
    again in the next tick's iterations. Walls are nearly frictionless.
    There is no restitution.
  - A sleeping body is not tested, and no contact wakes a body.
- Dead characters (condition `0xA00`) are plain ragdolls without muscles.
- Living bodies have muscles:
  - Gravity is compensated by muscle tone.
  - A pose drive pulls each particle toward its rest vertex under the part's
    animated transform. The gain is 0.084 per tick (0.07 on the hands),
    scaled by tone² and by energy⁴.
  - Energy is the daze level. A blow takes up to 90 % of it, with a floor
    of 0.05. It recovers 0.005 per tick.
  - The original also runs a PD muscle per slack link toward the animated
    anchor, balance with recovery steps, foot planting, and hidden support
    forces on the pelvis and thorax pins.
  - Several of these read the character's race record in `races.rdb`: the
    stance factor (a standing human uses half its tone, and its joint
    muscles are damped with k = 40 / 0.5), the support-pin factor, the
    balance bonus, the arms' relaxation, the limb damping ratio and the
    gait table ([OTHER.md](OTHER.md), Races). Exarust decodes them
    (`exa_formats::races::RaceBody`) but does not use them yet.

How a tick runs. The original steps every body of the world in one 83 Hz
tick, in this order:

1. Every character's body tick, from the last registered character to the
   first, which is the player, so the player comes last. The body tick
   runs the character's behaviour (the player's steering, once per tick),
   samples its clip into each part's animated transform in character
   space, and multiplies that by the character's frame into the world.
2. The integration of every awake body.
3. 20 iterations over all bodies.
4. Contacts dropped and the bodies tested against the level.
5. 8 friction passes over all bodies.
6. The post-step.

Exarust runs the same order in its fixed step. The character controllers
move first. Then each living body's character tick (`CharacterTicks`,
player last) advances its clip clock by one tick and samples the clip; its
targets are the body root's world transform times the clip pose. Then the
solver (`BodySystems`) deals the queued blows and runs steps 2–6 over every
awake body. The same ticks give the same particles at any frame rate. Blows
sent between ticks wait in a queue for the next one. Parts are drawn
between where the tick before last and the last tick left them. Exarust
keeps the clip clock at 60 frames a second until the original's rates are
pinned. Balance still runs once a frame, over the ticks that frame ran.

Exarust's muscles (`exa_physics::body::Drive`) are a pose drive with
critical damping relative to the pose's own motion:

- The pelvis drive is 2× stronger and stands in for the support forces.
- The body is carried rigidly with its root's (its character controller's)
  motion since the last tick, so muscles only work against the pose and
  outside forces. Where the last tick left each part, its level contacts
  and last tick's targets are carried with it.
- The pose's own motion over a tick is this tick's targets against last
  tick's. A fall restarts the targets, which are then measured from the
  body's own pelvis. A fallen body's muscles act inside the body: its
  limbs are damped relative to its rigid motion (bulk velocity and spin),
  and the net shift and turn of the whole body that the pulls and damping
  add up to are taken back out, so muscles change its pose but not where it
  goes.
- Animation clips play in place: the root's horizontal travel over a clip
  is removed, since the controller moves the body.
- Balance follows the original's rule (`exa_game::balance`):
  - The centre of mass is projected 0.1 s ahead by its velocity and
    measured against the segment between the feet.
  - The threshold is (energy·0.75 + 0.25)·30 cm. The score is stability⁸.
    Exarust takes stability as 1 within the threshold, falling to 0 at
    twice it; the original's exact measure is unknown. Attack lunges move
    the centre of mass up to 20 cm off the feet.
  - A counter grows by 10 − 10·score per tick. At 100 it sets balance to
    the score. Otherwise balance recovers by (5 − speed in cm/tick)·0.01
    per tick.
  - Below 0.6 the body falls. Its support goes, tone drops to 0.5, and it
    plays Fall Forward/Backward into Unconcious 1, held relative to its own
    pelvis.
- Getting up:
  - Once settled (2.5 s, energy ≥ 0.9), the body plays whichever of
    `Get up` / `Get Up 2` has a first pose lying the same way, judged by the
    pelvis's front axis facing up or down.
  - The controller is first turned and moved so that clip's first pose lies
    on the body.
  - While getting up, balance rises by 0.05 per tick.
- Link PD muscles, recovery steps and foot planting are not reproduced yet.

Per-tick trace. `exarust --trace <file.csv>` writes one row per traced body
after every tick. It traces the player's body, or every simulated body when
there is no player. The columns are those of the retail body probe, so a
capture of the original and an Exarust run compare row by row. Lengths are
in cm and the clock in clip frames. A column Exarust does not simulate yet
is left empty.

| columns | content |
|---|---|
| `tick`, `body` | tick number since the start; the body root's entity |
| `pelvis_p{0..3}_{x,y,z}` | the pelvis's four particles (world) |
| `pelvis_height` | the pelvis part's origin height |
| `root_clip_height` | the clip root's height over the body root (the pelvis's clip pose) |
| `frame_height` | the height of the frame the targets are placed in: the original's sits at the pelvis height, Exarust's body root at the feet |
| `target_pelvis_height`, `stance`, `tone_target` | the lift servo's target, the stance factor, the tone target (empty) |
| `tone`, `energy` | muscle tone and energy (daze) |
| `support` | support S, grounded × balance (empty) |
| `balance`, `balance_counter` | balance and its instability counter |
| `target{1..18}_{x,y,z,qx,qy,qz,qw}` | each part's animated world transform this tick: place and rotation |
| `foot_{r,l}_flags`, `foot_{r,l}_anchor_{x,y,z}` | foot contacts (empty) |
| `motion`, `clock` | the action's index in the motion set, and the clip clock |

## Motion set (`3DAB`)

```
u32 bones (18), u32 actions (48), u32 movesets (32)
action × actions:   char name[16], u32, u32
moveset × movesets:
  char name[16]          "One Handed"
  char parent[16]        "Default": the moveset that supplies missing actions
  per action:
    i32 keys             ≤ 0: not defined in this moveset
    i32 mode             (3 in most clips)
    f32 time[keys]       frames (10, 20, 30, …)
    f32 root[keys][3]    pelvis position (cm)
    pose[keys][bones]:   f32 rotation[4] (local x, y, z, w), u32 flags
```

Actions: Walk, Run, Backpedal, Defence stance, Cmbt - Idle, Attack
Mid/Down/Up, Attack Combo 1–3, Stab, Overhead Swing, Kick, Get up ×3, Fall
Forward/Backward/High, Jump, Idle, Idle 2, Parry Left/Right/Up/Left Dn/Rgt
Dn, Stand 2, Scratch 1, Break Tangle, Unconcious 1/2, Sitting, Switch
Weapons, Kneel, Rebalance, Force Blast/Barrier/Raise/Weapon/Torrent, Energy
Shock, Disp Form, Cower, Combat Walk, Power, Head Pain.

Movesets (parent): Default; One Handed (Default); 1H & Shield, 1H & Torch,
2W Light, Torch, 1H Heavy (One Handed); Shield C1/C2 (1H & Shield); Two
Handed (Default); 2H Light, Greatsword (Two Handed); 2H Flame (2H Light);
Polearm (Default); Golem, Golem B; Herath, Gobbler, Fael, Demon A01,
Harvester, Herath Ogre, Ogre, Ogre old, UnarmClaw; and creature variants of
One Handed / 1H & Shield / Polearm.

About 1 200 keys in total serve every humanoid and creature: the original
drives physically simulated bodies toward these poses rather than playing
them directly.

## Skinning

The original computes skin weights for the body at load time; none are
stored. Body parts (`humanbase.rfc`) are meshes in their bone's space. Each
carries invisible seam triangles (material `_null`) whose per-triangle flags
(sub-chunk `0x3DC3` after the triangle ranges: a `u32` per triangle) hold a
seam class in the high byte:

| bits | falloff radius | multiplier on distance² (cm²) |
|---|---|---|
| `0xC0000000` | 20 cm | 0.0025 |
| `0x30000000` | 15 cm | 0.00444 |
| `0x0C000000` | 10 cm | 0.01 |
| `0x03000000` | 5 cm | 0.04 |

Visible skin triangles have flag 1. Bits `0x200000`/`0x400000` are apparel
cull bits. The word before a mesh's streams (newer files) holds skinning-mode
flags; `0x100000` marks apparel.

Rules (`exa_assets::skin`):

1. Connected regions of seam triangles (class `0xC0`) form groups. Their
   boundary edges (shared with non-seam triangles) form the loop that meets
   the neighbouring part. The group centre is the centre of the loop's box.
2. Joints: the parent at the part's origin, each child at its translation.
   Each joint takes the nearest free group, nearest pairs first.
3. Vertices of a seam region are split between the own bone and the group's
   neighbour: 0.5 on class `0xC0`, 0.25 on `0x30`, 0.125 on `0x0C`.
   Shared loops therefore sit half and half, and adjacent parts coincide.
4. Other vertices: for each group, m = min over loop edges of
   distance²·multiplier. Keep up to 3 groups with m < 0.999. Each gets
   w = 1 − √m, damped with s = Σw, q = Σw², k = s/q (s ≤ 1) or 1/q,
   w = min(w, w²k). Each neighbour receives 0.5·w; the own bone keeps the rest.

Checked against the shipped body: in `12RUArm`, the elbow and shoulder loops
get 0.5/0.5, and at x = −20.3 cm the weights are 0.563 own, 0.403 forearm and
0.033 shoulder.

Exarust skins each part as a Bevy `SkinnedMesh` whose joint j is the part of
bone j. The inverse bind pose is relative to the part's rest pose. A skinned
part gets its mesh only when it is bound, since Bevy cannot draw a mesh with
skin attributes and no skin. Apparel
takes each vertex's weights from the nearest body vertex of its bone. The
original runs a similar builder for apparel; it also hides covered body slots
(per-slot coverage bits of equipped items) and culls apparel triangles by the
cull bits. Neither is done yet.
