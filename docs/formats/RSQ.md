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
| `0xDDBC` | foot contact points (each lower leg: one, at (±0.16, −48.97, 2.4) cm; the original stands the leg on its foot part's origin instead, below) |
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
- Joint ranges. Nothing but the links bounds a joint: no bone carries a
  bend weight or an angular stiffness, and the solver's only test is a
  link's length against its slack (REA on both link iterations). The
  ranges of `humanoid.rsq`, as turns from the clip's zero turn:
  - Elbow: a hinge about the forearm's y axis, through its 2.61 cm link's
    point. Its 12 cm link lets it bend from straight to 120° and never
    past straight: 10° past straight stretches that link 1.8 cm beyond its
    slack, 45° past 7 cm, and a living link takes the excess back in full
    every pass (`exa_game` test of the shipped links).
  - Knee: a hinge about the lower leg's x axis, through its zero-slack
    link's point, bent from 10° to 130°; it never quite straightens (same
    test).
  - Computed by scanning every turn (research notes, not tested): the
    shoulder a cone of 45–51° with ±41–46° of twist, and with the
    collarbone the upper arm rises at most 35–40° above the horizontal;
    the wrist 74° toward the palm, 27° back, about ±30° sideways and ±65°
    of twist; the elbow's side bend and twist about ±15°.
  - The clips are authored inside these ranges: most links' worst key is
    1.00–1.05 × the slack, and only a few keys of Fall Forward and Break
    Tangle reach 1.4–3.3 ×. The original clamps angles only in the
    animated pose (the leg IK keeps the knee's inner angle within
    30°–180°, the head look within ±72°·0.8, the foot's ground pitch),
    never an arm's.
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
    link is a link muscle (below), plus its excess beyond the slack at full
    weight; the sum is cut to 80 cm.
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
    over 10 cm of slack (the leg limit): 0.2 when a living body is built,
    then following its gait (below), and 1 again once it goes limp, as the
    original's death sets it. The femur's first such link is its leg limit.
    Each link also keeps the original's
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
  - A sleeping body is not tested, and a level contact never wakes one.
- Sleep (`Ragdoll::countdown`; REA on the plain step, the post-step, the
  sector step and every wake helper). Each body tree keeps one countdown,
  in ticks. Before each tick's integration a limp body takes the band of
  its fastest part, by the last tick's motion (the bands of its velocity
  retention): over 40 cm/s the countdown goes back to 256; between 20 and
  40 cm/s it is lifted to 32 from below, falls by one above 32 and by two
  at 32, so it is held at 30–32; under 20 cm/s it falls by two; never
  below 1. At 1 the post-step puts the body to sleep. A body at rest so
  sleeps 128 ticks (1.54 s) after its last fast tick, one that crept at
  20–40 cm/s about 16 ticks after it slows, and one that keeps creeping
  never. Its first tick asleep rolls its particle history without moving
  it, so its velocity is gone; from then on (deep sleep) nothing is done.
  A sleeper is not integrated, solved, tested against the level or
  rubbed; its stored level contacts are kept, unused.
  - A living body never sleeps: the original wakes its tree to 256 every
    tick, standing, fallen or getting up (only dormant statues and golems
    count down; Exarust has none). Death leaves the countdown as it is
    (256). Registration sets it to 0, so a placed corpse sleeps after its
    first tick, and the living builder wakes its body to 256. Exarust
    builds every body at 256, since its corpses still collapse from a
    standing pose.
  - Wakes: a contact between bodies raises the lower countdown to the
    higher one and drops the sleeper's level contacts; the explicit wakes
    (knock-out, revive, statues, grabs) raise it and keep them. Level
    contacts never wake a body or keep it awake. In Exarust a blow (so
    every hit) wakes a body as a living body's touch (256) does, and
    going limp wakes it to 256; bodies do not touch each other yet.
- Dead characters (condition `0xA00`) are plain ragdolls without muscles.
- Living bodies have muscles:
  - **The carry** holds the body up in place of gravity (`exa_physics`,
    `body_lift.rs`; REA on the limb step, the body tick, the muscle setup,
    the feet routine and its lift servo, with their constants read from
    the executable). Each tick the character tick sets the carry to
    gravity (−0.142 cm/tick²), then, with support only (a grounded foot),
    the **lift** takes g·f·S off it, in cm:
    1. h = target pelvis height − the pelvis origin's height;
    2. standing, f = 0.075(h + 15), at most 1.5; a negative f lifts
       nothing (crouched deeper than 0.9 it may go down to −10). Moving,
       f = (0.4·run + 0.2)(h + 5), nothing when negative, at most 2;
    3. f −= (1 − |f|/(|f| + 2))·E²·v, v the pelvis origin's vertical
       displacement over the last tick;
    4. over a tone t of a half f ×= 2(t − 0.5) and the carry is
       g(1 − f·S); at a half or less it stays gravity.

    After its inertia each particle of a part over half its tone moves by
    g(1 − a) + carry·a, any other by g: a = t × the part's support share
    for the pelvis and thighs (1), lower legs (0.5) and feet (0.25), and
    t × E for the upper body. A standing body floats on its legs at f = 1,
    its pelvis 1.7 cm over its target; well under it, f = 1.5 lifts the
    pelvis and thighs at half g. A dazed body's upper body takes less of
    the lift, so it sags; a body without support falls at g. The original
    zeroes the shares of the legs and pelvis on bodies of scale over 1.3,
    and multiplies f by t² while collapsing; neither applies yet.
  - The **target pelvis height** comes from the feet, each tick, in cm.
    Each leg's is its ground + crouch·60 + (1 − crouch)·(the clip root's
    height over the leg's animated ankle + its ground offset). The ground
    offset is the sole height standing; moving, the ankle's height over the
    lowest ankle of the moveset's walk in its first pose, plus the sole
    (the original keeps that lowest ankle for walking, jogging and
    sprinting blends; Exarust's moving bodies use the walk's). The target
    is the lowest foot's, blended by half toward the other's while the
    other's contact height over its ground plus the lowest foot's contact
    height is under 40 cm, by 0.05 a cm less up to 60 (the original's test
    as read: on a floor high in the world nothing is blended), and not when
    the other foot has no ground. Standing, it is capped at that ground +
    sole + leg length and raised 5 cm; moving, neither. With no ground under
    any foot it is −1000 m: no lift. An idle human (the clip root 91.4 cm
    over the ankle, sole 9.68 cm) is capped at ground + 99.83 cm and stands
    with its pelvis at ground + 101.5 cm. Not reproduced: the stepping
    (h + 10, 70 cm crouched) and resting (h + 20) variants, the walking
    blend weights and swing flags, and the gait-smoothed ground (C19).
  - **Link muscles** turn every joint (`exa_physics`, `body_muscle.rs`). Each
    slack link (slack over 0.01 cm) pulls its point on the part toward its
    anchor: where the clip's joint angle puts that point, given where the
    parent actually is. The anchor is the point under the part's animated
    turn relative to its parent's, placed at the part's origin (the model's
    translation), kept as weights on the parent and read on its current
    particles. In each of the 20 iterations, in cm:
    1. the anchor is drawn toward the point by the part's relaxation λ;
       e = anchor − point;
    2. e += (how the gap changed since the tick began) × D·k/(0.5|e|² +
       1.4), × 0.6 on the pelvis and legs (bones under 8). D = (E − 0.5)²,
       0 below E = 0.5; k = 50 while the body moves, else 40 / the race's
       stance factor (80 for a human). Idle that is a gain of 14.3 (8.6 on
       the legs), walking 8.9. Under 1.19e-7 cm² the link does nothing;
    3. saturated e/(1 + c|e|), c = 0.1 per cm;
    4. times σ = link strength × part factor × √E·T, plus the excess
       beyond the slack at full weight, cut to 80 cm, shared by the parts'
       mobilities.
    The damping acts on the joint's own motion, not the clip's, so a joint
    lags a fast clip: an elbow swung 40° in 10 ticks is 18° behind when
    the swing ends and within 3° twenty ticks later.
  - **Gains**, set every tick by the character tick:
    - Stance factor s: the race's standing (human 0.5); while moving it
      rises 0.01 a tick toward 0.7 + 0.3·run and keeps what it reached.
      T = tone·s, so an idle human pulls at √E·0.5.
    - Part factors: 0.5; a foot half its lower leg's (0.25); the head a
      quarter of the thorax's (0.125); a thigh 0.2 × 1.25 standing and 0.2
      moving; the shoulders, arms and hands 1.25 × the arm stiffness.
    - Arm stiffness (both arms): 0.5 off balance (balance under 0.6,
      falling, lying, getting up); 0.2 at tone under 0.85 or in the walk and run cycles; otherwise
      x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), 0.4 for a human; at
      least 0.4 × the turn left (rad), at most 0.4. An idle arm pulls at
      σ = 0.25, walking 0.175, off balance 0.31. The original's attack,
      parry, recoil and power states are not reproduced: attacks use the
      relaxed value.
    - A leg limit link's strength moves a tenth of the way each tick to
      0.5·S² standing or 0.4 moving (0.2 → 0.5 standing).
    - Relaxation λ of a part with link muscles: toward 0.6 − 32|Δa|² (cm²,
      the most one of its anchors moved since the last tick) × the race's
      arm relaxation, capped (an arm part at 0.6 − (x − 0.1)·0.8, others at
      1); it rises 0.015 a tick, falls to the target at once and stays when
      the target is negative. A held pose is followed softly (an idle arm's
      λ settles at 0.36). A supported lower leg and its thigh instead take
      clamp(0.6 − 0.001·| |hip − foot contact|² − (2r)² |, 0, 0.2), r the
      contact's (the ankle's, 41.95 cm) distance from the knee, rising
      0.01: 0.2 for a straight standing human (hip 85 cm from the ankle).
    - Support S: grounded × balance, measured each tick (Balance and
      support, below). A body without two legs has 1 while it stands and 0
      once fallen.
  - A **pose drive** turns the pelvis, lumbar, thorax and feet (gain
    0.03·K, K = 19200/83² = 2.787) and the head (0.01·K; the original
    lowers an aged character's) toward their animated orientation; thighs,
    shins, arms and hands have none (`exa_physics::body`, `drive_parts`;
    REA on the limb step, its gain setup and the body tick). After inertia,
    each particle's target is its rest vertex under the part's animated
    rotation (the body root's turn times the clip's), placed at the part's
    origin as its particles now put it: there is no position or height
    target, and a part moved as a whole is not pushed. In cm:
    1. o = target − particle; under 1.19e-7 cm² nothing;
    2. c = o·v/(s|o|²), v the particle's displacement over the last tick
       (before this tick's inertia), s the stance factor; at c ≥ 1 (the
       particle already closes at s|o| a tick) nothing;
    3. push G·o·(1 − c), its vertical cut to 0.1: a P term and a
       one-sided damper along o that never brakes past the target, and
       pushes harder (c < 0) a particle moving away;
    4. G = gain·t²·T·E⁴·(0.5T + 0.5)·S², t the tone, T = tone·s, E the
       energy, S the support; only while the body has support. An alert
       human: idle 0.0314 (head 0.0105), walking 0.0497, running 0.0836.
       A dazed body (E = 0.5) keeps a sixteenth.
    The original's v is the particle's world velocity, the body's travel
    included, so a walking body (|c| ≫ 1) is not pushed along its motion
    and pushed harder against it. While Exarust's capsule carries a body
    across the ground (until C18), that carry is not velocity: v is the
    velocity in the carried frame (its vertical is the world's). The
    original also multiplies a lean into the spine's rotation and keeps
    only the vertical push for the unconscious; neither is reproduced.
  - Energy is the daze level. A blow takes up to 90 % of it, with a floor
    of 0.05. It recovers 0.005 per tick up to the most energy (1 for a
    human).
  - The original also runs recovery steps, foot planting and support
    forces on the pelvis and thorax pins.
  - Several of these read the character's race record in `races.rdb`: the
    stance factor, the support-pin factor, the balance bonus, the arms'
    relaxation, the limb damping ratio and the gait table
    ([OTHER.md](OTHER.md), Races). Exarust decodes them
    (`exa_formats::races::RaceBody`); the muscles use the stance factor and
    the arm relaxation (Skeleton 0.75 and 0.5), the balance the balance
    bonus (1 in every record).

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
player last) measures its balance and the ground under its feet (below),
advances its clip clock by one tick and samples the clip; its targets are
the body root's world transform times the clip pose, and its carry is set
from the lift. Then the
solver (`BodySystems`) deals the queued blows and runs steps 2–6 over every
awake body. The same ticks give the same particles at any frame rate. Blows
sent between ticks wait in a queue for the next one. Parts are drawn
between where the tick before last and the last tick left them. Exarust
keeps the clip clock at 60 frames a second until the original's rates are
pinned.

Balance and support (`exa_physics::body::Balance`, `Ragdoll::measure_balance`;
REA on the original's balance and feet routines, its Height Query, its body
tick and muscle setup). Every rule is in the original's cm and ticks:

- The **chest** is the balance point: the mean of the two thorax pins
  (±10.4, 9.65, 1.25 cm in the thorax) on the body's particles (the root
  part's origin without pins). Its motion is smoothed twice a tick, fast
  (0.4) and slow (0.1), from where it was the last two ticks: a motion
  shows a tick late.
- **Each leg stands on its ankle.** The original's muscle setup replaces
  the stored foot contact with the origin of the leg's foot part, the
  model's translation (±1.24, −41.87, −2.32 cm in the lower leg), before
  anything reads it. A leg is 43.20 + 41.95 = 85.15 cm long (knee plus
  ankle). The sole height is the depth of the foot's box under its origin
  less 0.5 cm (9.68 cm on Exarust's foot mesh box).
- The **support point** is the closest point to the chest on the segment
  between the two ankles (the parameter taken horizontally, the point
  keeping the ankles' heights), moved halfway to the ankles' centre while
  standing. The chest is first led: standing after a landing by the slow
  velocity × a weight that starts at 5 and fades 0.05 a tick; moving with
  support by 3 × the fast velocity; back by 10 cm at a full crouch
  (Exarust has no crouch yet).
- The **score** is q = u_y⁸ standing, u_y⁴ moving, plus 0.05 × the race's
  balance bonus, at most 1, u being the unit offset from the support point
  to the chest. A human (bonus 1) is steady (q > 0.99) within 15.6 cm of
  lean with the chest 125 cm up and scores under 0.6 past 50.2 cm (52.2 cm
  with the chest 130 cm up; 44 cm scores 0.698).
- A slow vertical chest speed v over 3 cm/tick sets the counter to 100 and,
  for a body grounded last tick, cuts q by (1 − 0.333(v − 3))², to 0 above
  6.
- The **instability counter** grows by 10 − 10q, rounded half to even (the
  processor's rounding), while q ≤ 0.99: nothing until q < 0.95. Otherwise
  it falls by 5 but never under 80 standing still (from 0 it jumps to 80 on
  the first steady tick), else by 10 to 0. At 100 balance is q and the
  counter stays at 100; below it balance recovers by (5 − v)·0.01 a tick at
  a slow chest speed v under 5 cm/tick (0.05 at rest). So once a body has
  stood, a second wobble topples it fast: a lean scoring 0.55 takes a fresh
  body 25 ticks, one that has stood 5. Getting up, balance rises 0.05 a
  tick and the counter falls by 5 while above 2.
- Then the **feet**. Each leg asks the level for the ground under it with a
  **Height Query**: a sphere whose lowest point starts at a point is swept
  straight down, and the answer is the height of its lowest point where it
  first meets the front of an upward face (where it starts, if it starts
  against one; a miss is −10000 cm). The probe is 5 cm along the lower
  leg's forward axis from the ankle, 70 cm above it, a 10 cm sphere,
  2000 cm down; while the body is down (lying or getting up) it starts at
  the ankle and the sphere is 5 cm. Living bodies are not part of the level
  (Exarust's level is its static colliders, so dead bodies and loose props
  are not either). The original's normal condition also sways the probe
  forward by the gait phase and widens it to 20 cm, and in combat shrinks
  it to 5 cm; Exarust leaves these for the gait (C19).
- Ground higher than half the leg (42.6 cm) under the pelvis is out of reach
  (the foot is lifted over it), except getting up. A foot is grounded when
  its ankle is within 20 cm (30 cm moving) of its ground plus its ground
  offset (the sole height standing; moving, also its ankle's lift in the
  walk, set by the lift each tick, so Exarust reads last tick's). With a
  grounded foot the body has support (regaining it sets the
  landing weight to 5); without, it counts the ticks it is airborne, and a
  standing body loses its balance. S = grounded × balance. A body getting up
  with no foot grounded and its pelvis within 30 cm of the ground under it
  has support at half its balance.
- Exarust's own, while a capsule leads the body: its controller's capsule
  (28 cm radius) may stand where the feet do not (perched on a stone the
  feet hang beside, or spawned over the floor), so a standing body whose
  controller stands on the ground keeps its balance while no foot is
  grounded (S is still 0, so it drops onto its feet).
- Measuring costs about 15 µs a body a tick in exanima01 (two queries; the
  query tries a 2 m column before the whole depth, with an exact drop onto
  each face, edge and corner).

Exarust's own, while a character controller (a capsule) still moves the
body across the ground instead of the original's locomotion, frame anchor
and support pins (C9, C14, C18):

- The body is carried with its root's (its character controller's) motion
  since the last tick across the ground and about the vertical only: its
  height is the lift's. Where the last tick left each part, its level
  contacts and last tick's targets are carried with it.
- The controller holds a standing body's pelvis over where it carries the
  pose's pelvis: its origin is pulled across the ground by 0.168 a tick ×
  tone² × E⁴, critically damped (2√g) against its horizontal velocity in
  the carried frame, the same push on all four of its particles, so it
  never turns the pelvis.
- Stairs: the capsule steps up before the feet do, and nothing yet lifts a
  walking foot onto a step (the original's swing flags, ground probe ahead
  and leg IK, C13 and C19), so a walking body catches its feet on the
  risers: in exanima01 it climbed one or two steps and fell forward.
- A duellist's blows (energy down to 0.62) sag the struck player's pelvis
  by under 1 cm and leave its balance at 1: the lift and the legs carry the
  dazed upper body.
- Animation clips play in place: the root's horizontal travel over a clip
  is removed, since the controller moves the body.
- Falling is still Exarust's (`exa_game::balance`; the original's
  Rebalance, Defence stance, staggers and down state come with C11). In the
  fixed step, before the character ticks, a standing body whose balance was
  below 0.6 last tick falls: it is no longer carried or held, its tone
  stays (as in the original), and it plays Fall Forward when its lean
  (support point to chest) points ahead of it, else Fall Backward, into
  Unconcious 1. Once it lies its support is 0, so only its link muscles
  act, inside the body: it falls as fast as a limp body.
- Getting up:
  - Once settled (2.5 s, energy ≥ 0.9), the body plays whichever of
    `Get up` / `Get Up 2` has a first pose lying the same way, judged by the
    pelvis's front axis facing up or down.
  - The controller is first turned and moved so that clip's first pose lies
    on the body, and settles on the ground there (it is marked airborne, so
    the next tick sweeps it down).
  - While its get-up clip plays the body's balance rises (above).
- Recovery steps and foot planting are not reproduced yet. After a get-up
  the feet stay where the clip left them (they bear weight now), so the
  body may stand leaning over them (balance 0.97 after `--shove 60`).
- The duellist's strike range is Exarust's own (1.2 m): link-muscle arms
  reach 10–15 cm less far than the clip's pose.

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
| `target_pelvis_height`, `stance`, `tone_target` | the lift's target pelvis height (−100000 with no ground under the feet; empty for a body whose balance is not measured), the stance factor s, the tone target T = tone·s |
| `tone`, `energy` | muscle tone and energy (daze) |
| `support` | support S = grounded × balance |
| `balance`, `balance_counter` | balance and its instability counter |
| `target{1..18}_{x,y,z,qx,qy,qz,qw}` | each part's animated world transform this tick: place and rotation |
| `foot_{r,l}_flags`, `foot_{r,l}_anchor_{x,y,z}` | foot contacts (empty) |
| `motion`, `clock` | the action's index in the motion set, and the clip clock |
| `countdown`, `sleep` | Exarust's own, after the probe's: the body's sleep countdown and state (`awake`, `asleep`, `deep`) |

The joint trace (`--joint-trace`), one row per body part, and the body
overlay are described in [`docs/DEBUGGING.md`](../DEBUGGING.md).

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

Head Pain (action `0x2D`, both hands on the head, elbows bent near 120°)
is played by a story event, the player recalling a memory, never as a
reaction to a wound (REA: its only writer is the game-script handler that
also shows `XU_Memory` and plays `XAU_Memory`; player input is blocked
while it plays).

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
