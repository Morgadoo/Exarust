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
  - Friction uses the part's own coefficient μ (limbs 2.5, feet 0.5 as a
    body is built; a living body's lower legs and feet take theirs from
    their foot contact's state every tick, [Feet](#feet)). With the limit
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
    every hit) wakes a body as a living body's touch (256) does, going
    limp wakes it to 256, and a contact between bodies equalises their
    countdowns as the original's does (below).
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
    zeroes the shares of the legs and pelvis on bodies of scale over 1.3
    (not yet in Exarust), and multiplies f by t² while collapsing (the
    word's bit 0x800: [The collapse](#the-collapse)).
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
    Each foot's ground is its own; the controller's floor does not raise it.
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
      quarter of the thorax's (0.125) times the character's head factor
      (1 − 0.8 × its feral level: a zombie's 0.025); a thigh 0.2 × 1.25 ×
      the body scale standing and 0.2 moving; the shoulders, arms and hands
      1.25 × the body scale × the arm stiffness (the scale is the
      character's, 0.95 to 1.05 by its height: RFC.md, Stats derived at
      spawn).
    - Arm stiffness (both arms): 0.5 off balance (balance under 0.6,
      falling, lying, getting up); 0.2 at tone under 0.85; in the walk and
      run cycles the character's walking arms (0.2, or 0.2 − 0.15 × its
      feral level, at least 0.05: a zombie's arms hang at 0.05); otherwise
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
    0.03·K, K = 19200/83² = 2.787) and the head (0.01·K × the character's
    head gain, 1 − its feral level: none for a zombie) toward their
    animated orientation; thighs,
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
    of 0.05. It recovers 0.005 per tick up to the character's most energy
    (1, or 1 − 0.1 × its feral level: a zombie's 0.9). A body starts at 1,
    so a feral one is cut to its most on its first tick, as the original's
    body tick clamps it. The daze (energy more than 0.05 under the most:
    a zombie under 0.85), the pose hold, the swing clock and the attack
    gate all measure the energy against this most.
  - A recovery step starts when the chest leaves the feet (1.5 · R · E⁴ · S;
    a settled human, balance bonus 1, at 45 cm), leads the target by the
    chest's velocity, and the footwork's stance score moves one foot. The
    episode ends by its timeouts and landings. The thorax pins then lead
    by the horizontal error from the pin to the posed target plus the
    step offset, times 0.7 (0.4 while Idle plays), (direction + 0.5),
    (2 − 25·s), 1.2, the square root of body scale and the stance level.
    Direction is (h²/60 + 0.25)·0.36·forwardness. The support pins pull
    the thorax and the pelvis (below); its feet
    are planted ([Feet](#feet)).
  - Several of these read the character's race record in `races.rdb`: the
    stance factor, the support-pin factor, the balance bonus, the arms'
    relaxation, the limb damping ratio and the gait table
    ([OTHER.md](OTHER.md), Races). Exarust decodes them
    (`exa_formats::races::RaceBody`, with the clip-rate base, size, scale
    and mass divisor the character's spawn reads); the muscles use the
    stance factor and the arm relaxation (Skeleton 0.75 and 0.5), the
    balance the balance bonus (1 in every record). What the body takes
    from its own character (most energy, head gain and factor, walking
    arms, scale, the pins' factor the support pins scale by) is derived
    from its record when it spawns (`exa_physics::body::BodyStats`;
    [RFC.md](RFC.md), Stats derived at spawn).

How a tick runs. The original steps every body of the world in one 83 Hz
tick, in this order (REA and `exe.asm` of its sector step and body tick):

1. Sim objects against the bodies near them.
2. Every character's body tick, from the last registered character to the
   first, which is the player, so the player comes last. Each character's
   body tick runs in stages, all of them before the next character's:
   upkeep (wounds, red regeneration, impacts, burning; a dead character
   stops here), readiness (stepping, energy, the daze block and its
   capture), anchor (chest velocities, the standing anchor), behaviour
   (the player's steering or the AI, once per tick), posture (falls and
   get-ups), attack (the swing, focus, health), movement (the movement
   decision, the turn, the locomotion, the clip clock), sampler (each
   part's animated transform in character space, multiplied by the
   character's frame into the world), carry (pins, balance, feet, lift,
   leg IK).
3. The integration of every awake body.
4. 20 iterations, each the pair contact pass, then every body's links
   (with its stored level contacts and edges).
5. Pair and level contacts dropped; sim objects stepped; the bodies tested
   against each other, the sim objects against the bodies and the level,
   then the bodies against the level.
6. 8 friction passes, each the pairs' friction, then every body's against
   the level.
7. The post-step (for a character: its pending hit and its damage first).

Exarust runs the same order in its fixed step: `BodyPreStep`, then
`CharacterTicks`, `BodySystems` and `BodyPostStep`. The character
controllers move first. Then each living body's character tick runs the
`CharacterTick` schedule's nine stages (`CharacterStage`), player last,
one body's whole tick before the next: posture says what the body does
(the condition, the collapse's dying tick and the falls are still decided
over every body just before the character ticks, in that order);
movement measures the walk's lowest ankle while it moves, leaves its
controller's step out of the carry and measures its balance and the
ground under its feet (below; the original measures them in its carry
stage, after the sampler); the sampler takes the capture of a clip started
since its last tick, advances its clip clock by the original's rate and
samples the clip ([Motion playback](#motion-playback)); the carry puts the
clip pose on the body root's world transform and its character frame
(below), sets its carry from the lift and has the controller follow the
frame. Then the solver (`BodySystems`) deals the queued blows and runs
steps 3–7 over every awake body; its stages (`SolverStage`) are where
pairs, sim objects and links between bodies join it. The same ticks give
the same particles at any frame rate. Blows sent between ticks wait in a
queue for the next one. Parts are drawn between where the tick before last
and the last tick left them.

Contacts between bodies (`exa_physics::body::Pairs`, the solver's pair
stage; REA and `exe.asm` of the original's sector step, its contact
generator, per-hit push, pair manifold, pair pass and pair friction, its
group-mask builder and layer test, constants read from the PE). In the
original every part is a body; two parts of one character are two bodies
of one tree. Characters, corpses and props meet alike: a prop's contact
mobility is its mobility, its group masks come from its tree, its layer is
0 (so a cup rests on a table that is a body, and a character pushes a
prop). Rules in cm and ticks:

- **When.** Body–body detection runs after the 20 iterations and before
  the level's, so the level has the last word. The pairs found are
  enforced in the next tick's 20 iterations, each pass before any body's
  links, then dropped; they rub in this tick's 8 friction passes, before
  the level does.
- **Who meets whom.** Every awake part meets the parts in the 3 × 3 cells
  (600 cm) around it that it has not met yet this tick, awake or asleep,
  so a sleeper is touched but two sleepers never meet. The spheres around
  each part's spheres must overlap. A character's feet never meet other
  parts (the original puts a foot out of body–body contacts when it is
  registered and every tick its leg finds the ground; nothing takes it
  back).
- **Group masks** (two parts of one tree). From the root, with a bit
  counter from 0 (wrapping at 32), a part with children takes the next
  bit, its parent's bit and the two bits its parent was given; a leaf takes
  its parent's bit; every child then also takes its parent's two given
  bits. Two parts skip each other when their masks share a bit, so a part
  never meets anything within three generations of it. A character's two
  feet share the union of their masks. For a human (13 bits): legs,
  pelvis, spine and head never meet each other; the lower body meets the
  humeri, forearms and hands; the lumbar the forearms and hands; the
  thorax and head the hands; a shoulder, humerus or forearm only the other
  hand; the two humeri and the two forearms skip each other; the hands meet.
- **Layer masks** (parts of two trees). Bits pair up from bit 4: channel k
  is membership bit 2k and exclusive bit 2k + 1 (the members are
  `0x55555550`). Two parts skip each other when both are members of a
  channel and either is exclusive there. A living character's lower legs
  are `0x300`, so two characters' lower legs pass through each other; at
  the collapse's start or on death they become 0 and meet everything.
  Feet are `0x30`.
- **Sides.** A part's order key is round(100 / mobility) (half to even,
  16 bits). The part with the smaller key, the more mobile one, brings its
  spheres; the other is touched (on a tie the awake one brings them). In
  the original the touched side is its mesh; Exarust tests sphere against
  sphere until the parts' spheres are fitted to their meshes. A limb's
  mobility is its part factor (pelvis 0.45 … hand 2.5) × n / ((20·size³ +
  V/10⁴)·d) with n parts, V the body's volume in cm³ and d the race's mass
  divisor: about 2/3 for a human (Exarust uses 2/3 until the parts'
  volumes are measured), so a pelvis is 0.3 (key 333) and a hand 1.67 (key
  60).
- **Each sphere** of the sphere side, where it is in the other part's
  frame, or swept from where the last tick left both parts when it moved
  over 0.1 cm² relative to the other part, against each sphere of the
  other. A hit gives the touched point and the push that takes the sphere
  off (pushes under 10⁻⁵ cm² are ignored; over 160 cm they are reported,
  not clamped). The contact's weights on the sphere side are those of its
  sphere's deepest point (or of the sphere's centre when that point lies
  far outside the tetrahedron, a weight past 1); on the other side those of
  the touched point in its rest frame.
- **Immediate push.** Each side's weights, negatives dropped, normalised
  to sum 1, give φ = 1 / (largest weight): 1 when the contact sits on one
  particle, 4 at the centroid; the weights are then divided by the largest.
  With f = 2φ_A / (φ_A + φ_B), the sphere side moves by its mobility share
  m_A/(m_A + m_B) × (2 − f) of the push and the other side back by its
  share × f, so the side whose contact sits on fewer particles moves more
  and the two parts' momentum is kept. Each push counts toward the part's
  0.1 cm edge restore. Later spheres see where the push left both parts.
- **Points.** A pair keeps at most four points (the weights on each side
  and the push's direction), the four spread widest in the sphere side's
  weights, as a part's level contacts are kept.
- **Pair pass** (each of the next tick's iterations). Each point still more
  than 0.01 cm deep along its normal is taken back to 0.01 cm, minus a
  friction term from the sphere side's travel relative to the other this
  tick across the normal: with L = (0.01 + 0.99·n_y²)·μ_A·μ_B (cm² a
  tick²), a travel whose square is at most L loses only 0.01·L of itself
  and a larger one is cancelled: the reverse of every other friction rule
  of the engine, copied as observed (limbs side by side: L = 0.0625 cm²,
  0.25 cm a tick). Each side moves by its share over its non-negative
  weights, and both restore their edges. A pair whose points need nothing
  in a pass is dropped for the rest of the tick.
- **Pair friction** (each of the 8 passes). Each point's slip across its
  normal relative to the other side this tick is cancelled under L and
  kept, less 0.01·L of itself, over it (the plain rule), each side moving
  toward the other's slip by its share over its raw weights; pairs with
  μ_A·μ_B under 1.19·10⁻⁷ do not rub.
- **Wake.** After a pair's hits, the body with the lower sleep countdown
  takes the other's (a sleeper also drops its level contacts), so a living
  body (256) that touches a corpse wakes it for at least 1.5 s.
- Not yet in Exarust: spheres against the touched side's mesh and the
  parts' spheres fitted to their meshes (C20); held weapons, shields and
  torches as parts, the hit events between a striking weapon and what it
  struck, and impacts with their sounds and damage (C21, C22); the placed
  state's one-sided pushes; the overlap-only queries of the AI and of
  placement; Break Tangle and the two-handed grip bits.

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
  against one; a miss is −10000 cm). The probe is 5 cm (× scale) along
  the lower leg's forward axis from the ankle (an anchored foot's moved
  toward its anchor by a share that grows 0.05 a tick to 0.5), 70 cm above
  it, 2000 cm down; while the body is down (lying or getting up) it starts
  at the ankle. Its sphere is 10 cm while the body walks (its condition 1,
  the walk or the run playing), else 5 cm plus 10 cm × that share. Living
  bodies are not part of the level (Exarust's level is its static
  colliders, so dead bodies and loose props are not either). A walking
  body's probe also sways forward by the gait phase and widens to 20 cm,
  and its ground falls by a share of the gait; Exarust leaves these for the
  gait (C19).
- Ground higher than half the leg (42.6 cm) under the pelvis is out of reach
  (the foot is lifted over it), except getting up. A foot is grounded when
  its ankle is within 20 cm (30 cm moving) of its ground plus its ground
  offset (the sole height standing; moving, also its ankle's lift in the
  walk, set by the lift each tick, so Exarust reads last tick's). With a
  grounded foot the body has support (regaining it sets the
  landing weight to 5); without, it counts the ticks it is airborne, and a
  standing body loses its balance. S = grounded × balance. Without
  support the body also asks for the ground under its pelvis (the
  character's ground): a 20 cm sphere from 20 cm above the pelvis, 2000 cm
  down, −10000 cm on a miss (`Balance::ground`; with support the original
  takes it from the feet, which nothing in Exarust reads yet). A body
  getting up with no foot grounded and its pelvis within 30 cm of that
  ground has support at half its balance; the fall check reads it too
  ([Falling to death](#falling-to-death)).
- Exarust's own, while a capsule leads the body: its controller's capsule
  (28 cm radius) may stand where the feet do not (perched on a stone the
  feet hang beside, or spawned over the floor), so a standing body whose
  controller stands on the ground keeps its balance while no foot is
  grounded (S is still 0, so it drops onto its feet).
- Measuring costs about 15 µs a body a tick in exanima01 (two queries; the
  query tries a 2 m column before the whole depth, with an exact drop onto
  each face, edge and corner).

The **character frame** (`exa_physics::body`, `body_frame.rs`; REA on the
original's anchor routine and its callers in the body tick, the get-up
start and the locomotion branch, its constants read from the executable).
The animated pose is put on a frame that turns with the body. Its origin,
the character's place, is where the pelvis origin is less the pose's root
offset (the clip root turned into the world), across the ground, at the
pelvis's height. Each character tick, before the targets are set, it
follows the pelvis:

- while the body moves, onto the pelvis origin itself, every tick (the
  original also while stepping);
- without support, or collapsing, to the pelvis less the root offset,
  every tick;
- standing on its feet, only while its support is under 0.95 (the
  original compares with a double, so a support of exactly 0.95f is under
  it), and then once that place is more than 5 cm away; at 0.95 or more it
  stays put. Its height is brought to the pelvis's every tick either way.
  A steady body's stance spot is so fixed in the world, and an unsteady
  one's follows its pelvis;
- down (off balance with a balance under 0.3), within 5 cm, the distance
  measured in all three axes from where it was last placed;
- getting up, within 10 cm while the get-up clip is in its first quarter
  (its progress, (clock − blend) / last key, under 0.25), then not at all
  until the body stands. The clip blends from the lying body to its first
  key ([Motion playback](#motion-playback)), and the frame re-anchors with
  the root offset of that blend.

It is placed at once the first time. Not reproduced yet: the jolt that
moves the stance toward the feet (C15).
Only the targets' rotations drive the muscles; their places matter to the
support pins (below) and the feet's stance spots ([Feet](#feet)).

Exarust's own, while a character controller (a capsule) still moves the
body across the ground instead of the original's locomotion (C18):

- The body is carried with its root's (its character controller's) motion
  since the last tick across the ground and about the vertical only, its
  frame with it: its height is the lift's. Where the last tick left each
  part, its level contacts, its feet's anchors and last tick's targets are
  carried with it.
  The controller's moves to follow the frame are left out (the body made
  them already).
- The controller follows the frame. Each fixed step moves its feet a
  tenth of the way to where the frame's origin puts the body root, at most
  2 cm, through the controller (stopped by walls, stepping up as a walk
  does), before its own walk. The controller, the player's position (the
  camera follows the drawn pelvis), so stays with a shoved or falling body,
  and glides after a get-up's re-anchoring instead of jumping: that can be
  a metre, since Exarust's
  lying clip (Unconcious 1, until C11) keeps its pelvis 57 cm off its
  root, and the controller stands at the frame's origin while the body
  lies.
- The support pins, in each solver iteration: the two thorax pins and the
  pelvis pin (its origin) are pulled horizontally toward their points on
  the frame. With support the target stays near the feet centre, the mean
  of the ankles: a thorax target within (60 − 60·crouch)·0.6·scale cm, or
  (60 − 30·crouch)·0.6·scale cm while a foot is stepping, and the pelvis
  within 60·scale cm. Crouch stays 0 until a crouch exists, so a standing
  thorax reach is 36·scale cm. One application, in centimetres: raw is
  0.2·e·|e|/60 when |e| is under 60 cm and 12 cm along the error otherwise,
  less the point's horizontal displacement this tick, capped at 8 cm,
  times S²·pin-gain·P·the spawned pin factor, and times 0.3 while getting
  up. While a recovery step runs and the chest is not past the trigger,
  each thorax target is first shifted by the horizontal error from the
  pin's world point to the posed target plus the step offset. The shift
  is that error times 0.7 (0.4 while Idle plays), times (direction + 0.5),
  times (2 − 25·s), times 1.2, times the square root of body scale and
  times the stance level. Direction is (h²/60 + 0.25)·0.36·forwardness,
  with h in centimetres. A still 40 cm sideways error moves the target
  33.6 cm. P is √E·T·E⁴. Pin-gain starts at 0.08 and
  recovers 0.001 a tick while it is under that and the body is not
  collapsing. With no support
  the pull is zero and the target only shifts 5·scale cm forward. While
  the body moves the frame already follows the pelvis, so the pins do not
  hold it on one world spot. The capsule does not pull the pelvis
  horizontally; the planted feet and these pins do.
- Stairs: the controller-floor coupling is gone. The lift reads each
  foot's own ground, so the controller's floor does not raise the pelvis
  on level ground, perched, fallen or on the stairs. A 0, 2 or 5 cm float
  keeps the original's target, and a controller 12 cm up adds nothing. A
  synthetic body walking four 18 cm steps, stopping on the third, keeps
  its balance at or above 0.9 and ends with its feet on the landing,
  carried or not, and the two pelvis heights agree. The standing leg bend
  is in (`exa_physics`, `body_bend.rs`): the knee is the law of cosines on
  the two bones, 30° when the cosine is above 0.866 and 180° when it is
  below −1. The correction from the clip advances at most 0.02 rad a tick
  (0.2 while a recovery step is on) plus its stored rate, never past the
  target, and the rate becomes half that sum, within ±0.1 rad. The
  muscles follow it on the next tick. An unanchored bend target is the
  anchor lowered by twice the lift term stored before clamping: 2.25 cm
  when a standing body is at its target height and not stepping. While
  stepping, the thigh turns 75% of the angle from its rest direction to
  the hip-to-target direction in the character frame; otherwise that turn
  loses 10% a tick. An anchored stepping foot lifts by
  −(1 − crouch)·f·(1.5 − stance), with s = min(0.4·horizontal, 30 cm) and
  f from r = D²/(3s)² (a human at 40 cm, crouch 0, stance 0.5:
  −11.5185 cm). A free foot's lift keeps 0.8 of itself a tick. Not in
  yet: the walking gait's plants, the probe ahead, the swing flags and
  the locomoting bend (C19). A fall on exanima01's stair room is that
  missing probe; the controller floor is not restored for it.
- A duellist's blows (energy down to 0.62) sag the struck player's pelvis
  by under 1 cm and leave its balance at 1: the lift and the legs carry the
  dazed upper body.
- Clips keep their root's travel, but for the walk, the run and the
  backpedal (and Exarust's interim `Combat Walk`), whose travel along z is
  left out since the body's own motion makes it ([Motion
  playback](#motion-playback)). A get-up's travel moves its targets, and
  the support pins pull at 0.3 of the standing pull while the body is
  getting up.
- Falls, staggers and get-ups are the original's ([Falls and
  get-ups](#falls-and-get-ups), below). A falling or lying body is no
  longer carried or held by its controller (Exarust's own, until the body
  stands on its feet) and keeps its tone, as in the original: once it lies
  its support is 0, so only its link muscles act, inside the body.
- Recovery steps are in: the trigger, the stance score and timers, the
  episode, and the thorax lead's direction term (C15). The standing leg
  bend is in (C13): the law of cosines, the one-tick lag, the thigh turn
  and the step lift. Gait walking is not (C19). The feet are planted
  and step to their stance spots ([Feet](#feet)).
- The duellist's strike range is Exarust's own (1.15 m): link-muscle arms
  reach 15–20 cm less far than the clip's pose at the original's swing
  speed.

### Feet

The feet of a living body (`exa_physics::body`, `body_feet.rs`; REA on
the original's limb link iteration and its servo and grip helpers, its
feet routine, its balance routine's footwork, its standing sampler and the
callers of its contact clearer, constants read from the executable). Each
leg's foot contact (its ankle) has a state, an anchor where it is held, a
grip and its lower leg's grip ramp. Every rule is in the original's cm and
ticks.

- **States** (the original's bits): 2 touching, 4 anchored (moving to a
  new anchor: a step), 8 anchor reached, 0x10 lifted, 0x20 no ground within
  reach, 0x40/0x80 the ramp held at full / rising slowly, 0x100 a stalled
  step. Condition 1 is walking (the walk or the run playing), not the
  normal standing condition 0.
- **Planting** (the sampler, before the feet are measured): a body that is
  not walking plants each foot that does not touch where it is (touching,
  its anchor its ankle). Exarust plants it only once its foot meets the
  level. The standing leg bend folds the leg onto its bend target; it
  does not change this plant. A walking body's feet do not touch (Exarust's own until its
  gait plants them, M16/C19: the original lifts and plants them by the
  walk's phase).
- **Footwork** (standing, not moving): lying down (down, dormant, dead) no
  foot touches; getting up until nine tenths of its clip, or without
  support (last tick's), each foot touches on its stance spot, where the
  clip puts the ankle on the frame (not touching in a get-up's first
  fifth). Standing with support a foot that reached its anchor lands (a
  support foot again), one whose smoothed approach fell under 1e-5 lands
  where it is (stalled), and at most one foot steps to its stance spot
  (anchored, its anchor re-taken every tick at its height), pulled there by
  the drive. Which foot steps is the footwork's stance score: stay, or
  either foot, the highest, ties keeping the earlier. A free foot within
  5 cm of its spot (10 cm after a stall, outside a recovery step) scores
  as staying. A get-up's
  start, the walk's stop and a queued clip switch clear every state (the
  last two are implied in Exarust: a walking body's feet do not touch, and
  it has no queued switch).
- **Friction** each tick, on the lower leg and copied to its foot: 0 not
  touching, 2.0 touching, 0 anchored (stick below 1 cm a tick on a floor).
- **Anchor height**: the smoothed ground (it rises at once and falls by the
  walk's share, at once otherwise) plus the sole height, at most 100 cm
  over the ankle; with no ground within reach the character's ground (the
  lowest foot's with support, their mean walking, the pelvis's query
  without) and flags 0x30. The toe's floor is the ground (the original's
  sole query meets only level faces the feet skip; Exarust has none),
  never over the lower leg's origin.
- **The drive**, in each of the 20 iterations after the lower leg's links,
  for a body over half its tone with support and a contact more than 10 cm
  under the pelvis (the point refreshed every iteration): a touching foot
  (not lifted) with gap d to its anchor: anchored, within 5 cm (× scale;
  in 3-D when the anchor is above, else across the ground) it has reached
  it (flag 8, a landing, the anchor halfway to it across the ground), else
  its approach follows how fast it closes (0.95/0.05); its grip is 0 and
  the ramp rises 0.01 an iteration (to 0.25). Not anchored, the grip rises
  0.001 an iteration to 1 (1000 iterations, 0.6 s) and the ramp 0.001
  (0.25 at once while held, 0.0001 while rising slowly); while moving the
  anchor follows the ankle by 1 − 4 × the ramp. Then walking, a velocity
  servo (never down): a point closing q = moved·d/|d|² of its gap a tick
  is pushed d·(0.2·scale − q)·(0.3·run + 0.2)·ramp unless q ≥ 0.05;
  otherwise a PD grip: d·(0.1p + 0.004g(1 − p)) less the point's motion
  across the ground × (p + 0.03g(1 − p)), p the grip (kept up to 1 cm over
  the anchor, fading to nothing at 9 cm, the anchor not pulled down to) and
  g = scale × the support pins' factor (a human's 1.25) × the leg
  extension. A full grip pulls a tenth of the gap and stops the point
  across the ground. A foot not touching or lifted is only pushed up:
  the servo on the gap's upward part, gain S·(0.3·run + 0.2)·grip at rate
  0.1·scale walking, grip·S at 0.4·scale otherwise.
- **The sole**, in each iteration after the foot's links, on its feet and
  with support: the toe (the sole's point 20 cm × scale ahead of the ankle
  along the foot) under its floor is pushed straight up by the gap × S, at
  most 0.5 cm while the foot stands (touching, not anchored, not lifted),
  else 0.1 cm. The original also turns the sole toward its planted
  orientation (not ported; the pose drive turns the feet).
- **Level and extension**: with support the stance's level follows 1 − v²
  by a tenth a tick, v the share of the feet's ground difference in their
  squared distance (1 on level ground, 0.6 for feet 11 cm apart on a 15 cm
  step); the legs' extension is the level × (pelvis over the highest
  anchor ÷ the pose's pelvis over its mean ankle)², at most 1.
- Exarust's own, while its controller carries a standing body: the carry
  moves the feet's anchors with the body, so a standing turn carries the
  planted feet round (C18 removes the carry).
- Measured: an idle human's feet touch (0x2) on anchors 9.7 cm up, their
  ankles 0.1 cm off them (0.4 cm after a walk's closing steps; 0.004 cm of
  drift over 200 ticks); a
  synthetic body's quarter turn in place steps its feet six times while
  the foot planted at full grip stays within 0.64 cm (10 cm of slide
  without the drive); a 2 m/s shove sways its chest 4 cm while its feet
  move 0.1 cm. After a walk stops the feet step onto their stance spots in
  about 45 ticks.

### Falls and get-ups

What a living body's posture stage starts from last tick's balance, as the
original's posture block does (`exa_game::balance`; REA and `exe.asm` on
the posture block, the get-up start, the heading toward a fall and the
clip start, every constant read from the executable). The condition and
the branch come first ([Condition](#condition)). Thresholds marked
(double) compare the float in double precision, so the floats 0.8, 0.4,
0.3 and 0.1 pass them.

- **Off balance** (under 0.6, double) and not lying still, the clip rate is
  1.25 × the base (0.75 frames a tick), the arms' stiffness 0.5, no
  locomotion. With the lean u (unit, support point to chest), the frame's
  forward axis and the chest's slow velocity:
  - upright (u_y over 0.8, double): the Defence stance (blend 40) with a
    slow speed² of at most 0.3 cm²/tick² (double), else Rebalance (blend
    20), which holds the posture switch off for 41 ticks;
  - leaning further: a **stagger** when |u · forward| is over 0.4
    (double), the slow speed² under 8, the balance over 0.1 (double) and
    the chest under 80 cm off its support point: Get up leaning ahead
    (u · forward over 0), else Get Up 2, with a blend of 50 and the whole
    capture; when the clip changes it starts at the time of its key before
    the fourth (Get up, frame 55) or the third (Get Up 2, frame 40). Its
    first sample aims at the first key while its capture is meant for that
    later key, so it drops the blend there and then (clock 5.75 or −9.25
    at full tone, its start's tick running at the off-balance 0.75) and
    holds the first key exactly until its clock reaches it: 16 ticks for
    Get up, 27 for Get Up 2, at 0.9 a tick. The condition becomes getting
    up;
  - otherwise a **fall**: Fall Forward when u · forward is over 0, else
    Fall Backward, no blend, both looping held poses.

  Each is started only when its clip does not play already, and eased
  fast (1 + 0.11f − 1/(1 + 8f)) every tick. Under 0.3 (double) the
  condition is down meanwhile.
- **The daze in a fall.** A dazed body still falling (balance 0.3–0.6, its
  condition not helpless) gets the Defence stance from the daze every
  tick, and the posture stage starts its fall, stagger or Rebalance again
  after it: the clip restarts every tick (its clock 0.6 or 0.75, a new
  plain capture each time) until the condition turns helpless (down under
  0.3, or getting up), as the original's does.
- **Lying still** (under 0.3, double, its summed limb motion under 25 cm a
  tick): down, and the **get-up start**. With no get-up playing it waits
  until a foot gives support or it has gone 100 ticks (1.2 s) without.
  It picks Get up when its pelvis's front axis points down (its height at
  most 0, 0.2 while Get up plays, −0.2 while Get Up 2 plays), else Get Up
  2. Past the first quarter of the clip it picks (unchanged) nothing
  happens; else the condition is getting up, the heading (body yaw and
  stance facing) turns to the direction from the pelvis to the chest
  across the ground (the other way for Get Up 2) when its cosine to the
  heading is under 0.95 (double: more than 18.2° off), the frame
  re-anchors to the pelvis within 10 cm, and a changed pick starts with a
  blend of 20 from the whole physical pose; the feet's contact states are
  cleared. (A held body, state flag 0x40, does not get up; a dormant one,
  0x10, loses its tone and energy instead.)
- **Getting up**, the clip clock runs tone × (1.5 − 0.5 × the armour
  encumbrance) × base × 0.6 a tick while the tone is over 0.5 (0.9
  unencumbered at full tone, 0.7125 in the 0.625 plate set), but tone ×
  1.25 × base × 0.6 (0.75) on the tick the posture stage starts the
  get-up, from the floor or a stagger: the original sets the rate before
  the start, by the condition the stage began with; the movement
  stage starts the get-up again while it is under 40 % (double), so a body
  that rolls over changes clip with the same blend and capture, and one
  still in its first quarter re-anchors; the balance rises 0.05 a tick
  (above); the energy gains 0.01 × (1 − progress) more a tick (double, by
  last tick's progress); the clip ends into the stance with no blend and
  the condition normal.
- **Unsteady on its feet** (balance 0.6–0.8, double, at full tone, playing
  a plain clip: not an attack, the combat idle, the walk or run, the kick,
  a weapon switch or a power): the Defence stance (no blend) with the
  chest's fast speed² at most 0.3 (double), else Rebalance (blend 20, 27
  ticks of wait), eased fast. Weak (tone under 0.85): the Defence stance,
  no blend. At 0.8 or more the posture switch starts the stance (30
  frames, linear) once the Rebalance's wait, which every posture stage
  lowers by one, is under 1.
- **Toward a fall**: under balance 0.8 (double) and not getting up, when
  the pelvis's front axis has turned more than 45° from its clip's
  (cosine at most 0.7, double), the heading turns to the direction from
  the chest to the pelvis (front axis height over 0.4, double), from the
  pelvis to the chest (under −0.4) or along the front axis, across the
  ground, when it is more than 37° off (cosine at most 0.8, double).
- Exarust's own: a fall, or lying still, takes the body off its controller
  (no carry, no hold); a stagger, a get-up or standing again puts it back.
  While its posture stage takes an off-balance branch (falling, lying,
  getting up) the body is marked off its feet: it is not steered, engages
  no attack and its stance does not switch (the original turns its
  locomotion off there).
- In game (light runs, `--fixed-frames play arena`): `--shove 30` and
  `--shove 60` daze the player to 0.13; the daze's Defence stance, then at
  balance 0.61 Rebalance (unsteady), restarted every tick while it falls,
  down at 0.26 (31 ticks after the blow), Fall Backward at 0.19 (34
  ticks), Get Up 2 109 ticks later (its feet out of reach since the fall),
  Idle 155 ticks after that (its clock 0.75 on the get-up's first tick,
  then 0.9), standing at balance 1 with the pelvis 101.6 cm up;
  `--shove 90` the same 43 ticks later. One `--shove 90` run with stray
  walk input before the blow staggered at balance 0.25: Get Up 2 jumped
  in (its clock −9.1 then, when the first tick still ran at 0.9; −9.25
  now), held its first key 27 ticks while the pelvis sank to 23 cm, and
  ended into Idle 144 ticks later.

### Condition

The original keeps one condition word on every character, and most of its
systems test it rather than their own flags (`exa_combat::condition`; REA
on the body tick's posture block, the swing routine, the get-up, the
collapse, death and the masks its readers test, constants read from the
executable):

| value | meaning | written when |
|---|---|---|
| `0` | normal | standing in a stance or any other clip; the daze; a get-up's clip ends; a walk stops |
| `1` | walking | the walk or the run plays |
| `2` | attacking | an attack (actions 0x05–0x0C) plays: its swing writes it every tick |
| `8` | combat idle | `Cmbt - Idle` plays, or the character parries |
| `0x20` | casting | a power's clip (0x22–0x28) plays |
| `0x100` | weak | its tone is under 0.85 |
| `0x200` | down | off balance under 0.3; a knock-out |
| `0x300` | getting up | a get-up starts (a stagger plays one partway) |
| `0x600` | dormant | an AI lies as a corpse until it sees a target |
| `0x800` | collapsing | a lethal wound, until death |
| `0xA00` | dead | death; a corpse placed by its level |
| `0xC0` | shocked | a golem stunned by electricity, until its timer runs out |
| `0x8000`, `0x10000`, `0x10004` | kicking; switching weapons before and after the swap at 60 % of the clip | their clips play |

The readers test masks: `0xAC0` (down, getting up, dormant, collapsing,
dead, shocked: no daze stance, no turning, no attack engaged), `0xA00` (the
same but shocked: the exploration cursor's cross, the blow sag zeroed),
`0x200` (down, getting up, dormant, dead), `0x800` (collapsing or dead),
`0x10203` (no queued motion starts). State flags sit beside the word
(dormant `0x10`, waking `0x20`, a scripted hold `0x40`, the shock's stagger
`0x100`, a potion `0x800000`).

The body tick writes the word once, in this order: the daze block makes a
dazed character normal unless the word holds `0xAC0` or is 2; then the
posture block, skipped for the collapsing (their dying tick runs instead)
and the dead, and for the shocked until their timer runs out:
1. balance under 0.6 (the float compared in double, so 0.6f stands) or
   getting up: getting up stays; under 0.3 the word becomes down, whether
   the body lies still (its summed limb motion, every limb's farthest
   particle over the step, under 25 cm: the get-up decision follows) or
   still moves (a fall, a stagger); else it stays;
2. an attack playing: 2 (the attack routine);
3. a tone under 0.85: weak (with the Defence stance, blend 0);
4. by the clip: the kick 0x8000; a parry or the combat idle 8; a power
   0x20; the walk or the run 1; a weapon switch 0x10000, 0x10004 past 60 %
   (compared in double, so 0.6f is past);
   anything else 0 (with the posture switch to the stance).

Exarust (`exa_combat::condition`, in each living character's posture
stage, after the daze's stance; every `Fighter` keeps the word):
- the decision, as above, from last tick's balance, summed limb motion
  and tone and the clip playing; it also names the branch the original
  takes next (falls, lying still, getting up, attack, weak, standing,
  collapse), kept on the fighter (`PostureBranch`): the posture stage then
  runs the dying tick ([The collapse](#the-collapse)), the falls,
  staggers and get-ups ([Falls and get-ups](#falls-and-get-ups)) and, in
  the falls branch, the fall check ([Falling to
  death](#falling-to-death)). A body that dies there (its collapse ends)
  runs none of the stages after it, as the original's leave the body
  tick;
- the get-up's start and the stagger write getting up, and the character
  tick's sampler stage normal when the get-up's clip ends into the stance
  (the same tick);
  a killing blow starts the collapse (below; dead at once for a body
  without muscles) and a level's corpse is placed dead; a walk's stop
  needs no writer, since the player's steering starts the stance before
  the posture stage;
- the health bookkeeping, in the attack stage after the posture stage,
  holds yellow regeneration while the word is 2 or Cmbt - Idle plays, and
  skips it for the dead ([OTHER.md](OTHER.md), Regeneration);
- not yet: the kick, weapon switch and powers, knock-outs, dormancy,
  shocks (nothing lowers tone yet), the collision layer the original
  restores on leaving a state of 0x100 or more; Exarust's backpedal and
  combat walk stand in for the walk played backwards and the footwork, so
  they give 0.
- `--shove 60 play arena`: 0 in Idle, in the daze's Defence stance and in
  Rebalance while it falls, `0x200` from balance 0.26 on, `0x300` at the
  get-up's start, 0 when `Get Up 2` ends into Idle.

### The collapse

A lethal wound never kills at once (`exa_combat::collapse`; REA on the
collapse start, the dying tick, the body tick's collapsing branch and
death, constants read from the executable; six live retail collapses):
- **Start**, once (not while the word holds 0x800): the waking flag is
  cleared, the word becomes `0x800`, the stance and the clip Fall Backward
  (a looping held pose, its hands up at the head) with a blend of
  round((1.2 − E) × 300) frames, in double precision, E the energy after
  the blow's daze: 60 at full energy, 345 at the floor of 0.05; eased
  smoothly. A clip start does nothing when its clip already plays. The
  lower legs leave the living lower legs' layer (0; Contacts between
  bodies, above). (Not yet: the held items dropped (L13), a grab released,
  the death voice.)
- **Each tick**, in place of the posture block: the frame re-anchors on
  the pelvis with no reach; the arms' stiffness is 0.2; the dying tick
  moves the heading's x and z each by (random − 0.5) × 0.5 (the original's
  generator) and renormalises it (the stance facing follows), starts
  Rebalance (20 frames, linear: the arms flung wide) once the balance is
  under 0.8 in double precision, unless the character's size is 1.2 or
  more, takes 0.003 off the tone and 0.0003 off the support pins' gain (in
  double; the gain is set to 0.01 only if it goes under 0), and when not
  stepping starts a stagger step toward chest + unit(fast chest velocity)
  × (tone + 0.5)² × reach × max(balance, 0.4) × √scale (not yet: Exarust
  has no steps, C15). T = tone × stance factor. The clip clock, balance,
  feet and lift run on; the lift is multiplied by t² as well as
  2(t − 0.5), so it is gone at half tone; the pose drive is t²·T·…, the
  link muscles √E·T.
- **Death** at a tone of 0.25 or less (0.5 × the image's 0.5): 250 ticks,
  3.0 s from full tone. Muscles off, tone 0.01, the word `0xA00`; a plain
  ragdoll from that tick, its motion kept (Exarust gave it a made-up
  velocity before).
- Exarust starts it in its hit reaction, with the energy the blow's
  queued daze leaves, takes the body off its controller (no carry) while
  its balance and feet are still measured standing,
  drops its own fall and any attack, and runs the dying tick in the
  posture stage; a further blow shoves and bleeds a collapsing body but
  leaves its collapse. Its living support pins' gain starts at 0.08f and
  recovers 0.001 a tick under the double 0.08, so it settles at 0.081.
  The support pins read it each iteration.
- `--opponent play arena`, the duellist's seventh blow: E 0.668, Fall
  Backward with 160 frames of blend; the body sinks with its chest over
  its feet (balance 1) until its feet lose the ground 130 ticks later
  (1.57 s; the live collapses: 0.58–1.44 s, always at balance 0), when
  Rebalance starts; dead on the 250th tick, limp, partly on its back with
  the left arm out. Without the stagger steps it sinks where it stands
  instead of stumbling back.
- The player killed by the duellist's first blow (`--health 0.05,0.05`,
  `--pause-after-collapse`, eight runs): E 0.61–0.63, 171–178 frames of
  blend. At 0.4 s a slight crouch, the arms low; at 1 s and 1.2 s a squat,
  the knees at 86°, the chest upright over the feet; the feet lose the
  ground and Rebalance starts 133–158 ticks (1.6–1.9 s) after the blow, at
  tone 0.53–0.60; the body sits, then tips onto its back, the pelvis down
  at about 2.5 s. Dead on the 250th tick; settled it lies on its back and
  side, the upper arms turned 68° and 72° at the shoulders, one knee up
  (thigh 108°), the other leg out (20°), as the original's corpses lie.
  The live collapses at like energies threw their arms wide sooner: 0.88 s
  (E 0.645) and 1.42 s (E 0.613); the reference frames' player (E 0.36, a
  heavier daze) was in a deep crouch at 0.4 s and on its back by 1.2 s
  (Rebalance at 0.73 s). The lag is most likely the missing stagger
  steps' (C15).

### Falling to death

The original checks a fall on every living body that takes the posture
block's falls branch (off its balance and still moving, whatever it plays
there: the Defence stance or Rebalance upright, a stagger, a fall clip),
at the branch's end (`exa_combat::fall`, `exa_game::fall`; REA on the
check, its callers, the pit sound, the voices, the session's death hook,
the kill plane, constants read from the executable):
- **Steady** while its chest's slow smoothed vertical velocity (the
  balance's, lerp 0.1) is −5 cm a tick or slower (a NaN too): the fall
  starts over.
- **Falling**: faster, the first tick it screams (not if dead): the heavy
  groan of its voice (the voice set's `_GroanH`; the original's voice kind
  2) at gain 0.2 + 0.15 × a draw and pitch 0.95 + 0.1 × the draw before it
  (double precision), times the voice's volume and pitch.
- **Lost**, in the same tick or a later one, when its pelvis is 2500 cm or
  more over the ground under it (asked without support, above) and over
  every foot's ground: since a query reaches 2000 cm and a miss is
  −10000 cm, that means no ground within reach under the pelvis or any
  foot. Its time is stamped on the game clock (12 ms a tick); from then on
  only the clock counts, compared strictly, and only while the body stays
  in the falls branch.
- The **player** hears the pit (`pitdeath01`, gain 0.1, on the interface)
  more than 1000 ms after (the 84th tick), when its screen starts to fade
  to black ([UI.md](../UI.md), Death fade); more than 4000 ms after (the
  334th tick) it dies: dead, yellow and red 0, out of the world. Anyone
  else dies so more than 3000 ms after (the 251st tick), without a sound.
- **Out of the world**: the character, its body and everything it carries
  are gone (the original takes its node out of the scene and frees its
  items). The original's death here leaves its muscles flag on, but the
  body leaves with it, so no corpse keeps the living link rules.
- The **kill plane**: a body integrated by the plain step (for a character,
  only a dead one) whose root's first particle goes below −5000 cm is taken,
  once: a character loses its health and, unless it is the player, leaves
  the world the same way. A body lost to its fall is never taken again.
- Not in Exarust: the original also nudges its camera's wall tilt 0.0075
  rad a tick (overwritten at once by the camera), ends a possession or the
  status effects at the pit, holds a voice 500 ms after the last, and
  checks a body that became dead within its tick (which never happens).
  Exarust has no voice sets yet: the scream is a random `vhm01`–`vhm09`
  heavy groan, as the hit sounds pick a random grunt.
- `--at 0,200,0 play arena` (light): the player loses its balance on its
  first ticks over nothing, screams and is lost on tick 89 (its pelvis
  1.9 m lower, sinking 6.1 cm a tick), hears the pit on tick 173 and is
  gone on tick 423. (Exarust's fallen body sinks up to about 14 cm a tick,
  where the original's off-balance retention allows about 8.6.)

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
| `target{1..18}_{x,y,z,qx,qy,qz,qw}` | each part's animated world transform this tick, on the character frame: place and rotation |
| `foot_{r,l}_flags`, `foot_{r,l}_anchor_{x,y,z}` | the foot contacts in leg order (`r` the lower leg 03, `l` 06): the state in hex ([Feet](#feet)) and the anchor |
| `motion`, `clock` | the action's index in the motion set, and the clip clock |
| `countdown`, `sleep` | Exarust's own, after the probe's: the body's sleep countdown and state (`awake`, `asleep`, `deep`) |
| `frame_x`, `frame_z`, `root_x`, `root_z` | Exarust's own: the character frame's origin, and the body root (where the character controller stands), across the ground |
| `blend`, `hold` | Exarust's own: the blend (frames) the clip still has from its start's capture (0 once its first key is reached), and how far the sample was pulled toward the pose the body holds |
| `ankle_{r,l}_{x,y,z}` | Exarust's own: each foot contact's ankle where the tick left it, so a foot's slide from its anchor shows |
| `condition` | the character's condition word in hex (`0x300`), as the retail watcher logs it ([Condition](#condition)); empty for a body without one |
| `swing_rate`, `strength`, `attack_flags` | the fighter's swing (empty for a body that does not fight): the frames its swing routine last set for an attack's clip, the swing strength P, and its attack state word in hex ([Swing](#motion-playback)), the retail probe's rate, strength and state columns |

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
    i32 mode             bit 0: the clip loops (the original's clip start
                         reads only it); 3 in most clips
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

### The moveset tree

A moveset's parent is the **nearest moveset before it** whose name is
exactly its parent name; one without a parent name, or with none of that
name before it (its own name included), is a root. Children keep the
file's order. So a name used twice resolves to the nearer copy: the
creature variants hang under their own creature (REA: the motion
library's tree pass after loading, comparing the 16 stored name bytes;
every name in the installed file is zero after its terminator, so comparing
the names as strings is the same). The installed tree (`exa-tool rsq`
numbers it):

| # | moveset | parent | | # | moveset | parent |
|---|---|---|---|---|---|---|
| 0 | Default | — | | 16 | 1H & Shield | 15 |
| 1 | One Handed | 0 | | 17 | Polearm | 14 |
| 2 | 1H & Shield | 1 | | 18 | Golem B | 14 |
| 3, 4 | Shield C1, Shield C2 | 2 | | 19 | Polearm | 18 |
| 5–8 | 1H & Torch, 2W Light, Torch, 1H Heavy | 1 | | 20 | Herath | 0 |
| 9 | Two Handed | 0 | | 21, 22, 25 | Gobbler, Fael, Harvester | 20 |
| 10 | 2H Light | 9 | | 23 | Demon A01 | 22 |
| 11, 12 | 2H Flame, Greatsword | 10 | | 24 | One Handed | 23 |
| 13 | Polearm | 0 | | 26, 27, 31 | Herath Ogre, Ogre, UnarmClaw | 0 |
| 14 | Golem | 0 | | 28, 30 | One Handed, Ogre old | 27 |
| 15 | One Handed | 14 | | 29 | 1H & Shield | 28 |

### A character's clip table

The original builds each character's table (one clip per action) from a
**base moveset**, the race's own (`races.rdb`: `Default` for the human,
skeleton, ancient and golem suit; `Ogre`, `Herath Ogre`, `Fael`,
`Herath`, `Gobbler`, `Harvester`, `Demon A01`, `Golem`), and a **config
word** from the held items ([PLAYER.md](../PLAYER.md), Moveset from held
items), after every equip change (REA, the motion constructor):

1. The base is the first moveset of that name, ASCII case ignored. Its
   ancestors are overlaid from the root down: every clip a moveset defines
   (keys > 0) replaces the table's, whole, with no per-bone mask.
2. The config's low 16 bits name a weapon moveset:

   | code | moveset | | code | moveset |
   |---|---|---|---|---|
   | `0x1` | One Handed | | `0x10` | Two Handed |
   | `0x3` | 1H & Torch | | `0x20` | 2H Light |
   | `0x4` | 2W Light | | `0x30` | Greatsword |
   | `0x100` | 1H & Shield | | `0x40` | Polearm |
   | `0x200` | Shield C1 | | | |
   | `0x300` | Shield C2 | | | |

   It is searched under **each ancestor** of the base, from the root down
   to the base: the ancestor's children in order first, then each child's
   descendants the same way. Each one found is overlaid with its own
   ancestors below the ancestor it was found under, from the top down.
3. The overlays "Torch" (mask `0x10010000`) and "Flaming"
   (`0x10020000`), in that order, each when the config shares any bit with
   its mask: the two masks share `0x10000000`, so either asks for both. Each
   is searched among the children of the last weapon moveset found, of its
   parent, and so on up to (not including) the ancestor it was found under,
   then the same for the earlier ones; the first found is overlaid alone.
   No "Flaming" moveset ships.

A base no moveset is named after gives no table (the original logs "Base
motions not found" and goes on with a pointer that is no table). Tables
are cached by base and config. The constructor keeps at most four
movesets in the base's chain (the base and three ancestors), four weapon
hits and four path steps; the deepest base chain in the data is four
(Demon A01, Shield C1).

What the installed tree gives (`exa-tool clips <base> [config]`, pinned by
a corpus test over the 9 race bases and 26 configs):
- A human's table is the chosen moveset's own chain (One Handed over
  Default, Greatsword over 2H Light over Two Handed over Default, …); the
  Torch overlay lies over a shield moveset or a flaming weapon too (Walk,
  Cmbt - Idle, Attack Mid and Idle from #7).
- An armed creature takes the human weapon moveset, then its own over it:
  a golem with a sword draws on #0, #1, #14 and #15; an ogre on #0, #1,
  #27 and #28; a golem with a shield on #0, #2, #15 and #16.
- Any Herath creature with a one-handed weapon finds #24 below Fael and
  Demon A01, which come with it: a Fael, Herath, Gobbler or Harvester with
  a sword walks as the demon.

Exarust before this used the first moveset of a name and walked parents
by first name match: the creature variants were unreachable. 162 of the
234 race-and-config tables differ from that lookup, for a human only the
nine with the Torch overlay over a shield or a flaming weapon.

### Keys

The per-bone word of a key is
no mask either: it is 0, or 1 on the lower legs (bones 3 and 6, a foot
flag), in every key but some keys of 16 clips, where it holds float bit
patterns: Default Backpedal; "Torch" Walk (the last four of its eight
keys); 1H & Torch Walk and Cmbt - Idle; five 1H Heavy attacks; 2H Light
and Two Handed Cmbt - Idle, Two Handed Walk; the Gobbler's and the ogres'
walks and runs (corpus: 3 342 ones on the lower legs and 818 other words
in the installed `humanoid.rsq`). "Torch" (parent One Handed) defines Walk,
Cmbt - Idle, Attack Mid and Idle; its arm on the body model, in
[PLAYER.md](../PLAYER.md) (The carried torch), is pinned by a test against
the installed files.

About 1 200 keys in total serve every humanoid and creature: the original
drives physically simulated bodies toward these poses rather than playing
them directly.

## Motion playback

Evidence: REA (Ghidra 12.1.4) decompiles and `exe.asm` of the original's
character init, clip start, captures, living sampler and body-tick clock
block, constants read from `Exanima.exe`; `humanoid.rsq` and `races.rdb`
from the installation. Exarust: `exa_anim` (`Animator`, its `clock`),
`exa_game::limp` (the rates, the physical turns), `exa_combat::swing`.

**Clock.** A clip's clock counts frames; each body tick adds a rate. The
base is the race's clip rate (`races.rdb` `0x5C`: 1.0, the ghouls 1.5; the
character init first stores (80/83)/scale, then overwrites it). Per tick:

| state | frames a tick | a human, full tone |
|---|---|---|
| walking or running in the walk, run or backpedal clip | the gait's clip rate ÷ 83 (walk → jog by √run, or the backpedal's) × (0.8 × speed + 1), speed in cm a tick | 0.76 walking (1.69 cm), 0.89 running, 0.74 backpedalling |
| getting up | tone × (1.5 − 0.5 × encumbrance) × base × 0.6, only above half tone | 0.9 |
| a get-up's first tick (its posture stage starts it) | tone × 1.25 × base × 0.6, only above half tone | 0.75 |
| an attack (actions 5–12) | the swing's rate (below) | ≈ 1.0–1.3 |
| off balance (falling, lying) | 0.6 × 1.25 × base | 0.75 |
| otherwise (stances, idle) | 0.6 × base | 0.6 (49.8 frames a second) |

The clip ends when its clock less its blend reaches the last key's time. A
looping clip (mode bit 0, or one Exarust loops) then wraps by the last
key's time, and its frames before the first key blend from the last key
back to the first. A clip played once starts its follow-up with a blend
of twice its weapon's recovery for an attack, 60 frames from action 0x21
on, 20 from 0x1F, none below, eased smoothly; without a follow-up it
holds its last key. Progress is (clock − blend) / the last key's time.

**Starts.** Starting a clip sets its clock to 0, its blend (frames) and its
ease (linear unless the caller sets one) and captures where the body is:
the root and the pelvis's and legs' (bones 1–7) last targets, and every
other bone's physical turn under its parent part; the capture becomes the
bones' targets. A get-up from the floor captures every part's physical
turn, the pelvis's on the character frame. The original takes the capture
in the start; Exarust in the body's next tick, from the same state.

**Sampling.** With t′ = clock − blend: before the first key the clip blends
from its capture to the first key by clock / (first key + blend), eased
(smooth f²(3 − 2f), or fast 1 + 0.11f − 1/(1 + 8f) after falls); without
a capture, from its first key (from its last when looping). Once t′ aims
past the first key the blend ends for good: the clock loses the blend.
A capture is meant for the key the start named, the first but after a
stagger's jump in (a few keys in, its clock set to the key before): a
sample aiming at any other key drops the blend at once, and a clip not
looping then holds its first key until its clock reaches it ([Falls and
get-ups](#falls-and-get-ups)).
Keys are slerped, the root lerped. While Walk plays and the run blend is
above zero, each bone is then slerped toward the Run sample at the same
keys by 0.7 times that blend; the root stays the walk's. A full run is
63° of the way from a walk bone to a run bone 90° away. Sprint, whose
weight is the blend itself, is not in yet. The root keeps the clip's travel, but
the walk's, run's and backpedal's z (Exarust also drops its interim
`Combat Walk`'s). A body that has not sampled a clip yet has no capture
and starts from the keys, with no blend.

**Pose hold.** Each bone is then pulled toward the pose the body holds by
w = 1 − (1 − S)²·(1 − s)·(E/Emax)⁴ while dazed (energy more than 0.05 under
its most), else (1 − S)·s, when w is over 1.19e-7 (S support, s blow sag,
which fades 0.01 a tick and is 0 down; nothing sets it yet). The held pose
is the pelvis's and legs' current targets and every other bone's physical
turn, taken each tick the body is dazed, has a sag, or its support is
under 1. A dazed body standing on its feet holds its physical pose in
full: the clip has no say until its energy recovers. The root is not held.
The original lerps the quaternions' components without a sign check;
Exarust takes the short way.

**Swing.** Every tick of an attack's clip (actions 5–12), with the
fighter's condition attacking (balanced, not getting up), the original's
swing routine runs after the posture decision and before the clock
advances (`exa_combat::swing`; evidence: REA and `exe.asm` of the swing
routine, the release, the daze and its stall, the expected swing time,
the executor and its wrappers, the side starters and the clip start,
constants read from `Exanima.exe`). In order:

1. The game clock is stamped as the fighter's last attack tick (the
   executor's cooldown counts from it).
2. The window region: past 0.3 of the clip (double precision), or within
   the first 0.25 of a chained clip. In it the swing is late past 0.7 and
   each striker's tracker takes its tip's velocity (cm a tick; Exarust's
   from the striker's per-frame tip velocity): capped at 50 up to 0.8 of
   the clip, after it at the slot's best, which then only shrinks; the
   slot moves a quarter of the way a tick, its squared speed at most the
   latest's and 1000. Outside the region the trackers clear.
3. The rate: speed × g × 1.125 × base, g = 1 − 0.833·m·(d + d²), d =
   |progress − 0.5| (at or before the first key 1 − 0.833·m); a stab
   ×1.2, below 0.4 also × (1.2·progress + 0.5); a feral record (level
   f > 0, 0..1) × ((1 − f)/2 + f·S(progress) + 1/2), S(x) = (3 − 2x)x²
   (1 − f/2 at or before the first key). Speed, m and the recovery are
   the primary striker's weapon record (W15, W18, W19,
   [OTHER.md](OTHER.md), Weapon record, held by the character's own
   encumbrance and size); with no armour encumbrance speed = (1 −
   weight)·0.15 + 1.05 (×1.2 unarmed), m = 1 − balance, recovery =
   round((weight + 1 − balance)·10) frames; fists 1.44, 0, 0. The
   original's ×0.64 under a byte no code writes is left out.
4. A chained clip past 0.25: its state word becomes the continued bit
   (0x20000) alone and every striker's window shuts.
5. The strike window: the overhead's shuts past 0.9 with weapon class 0,
   else past 0.95, or at once past 0.85 when the swing falls slower than
   3 cm a tick; the stab's past 0.8 unless its thrust still drives on
   along the aim at 2.5 cm a tick or more, then past 0.95. It opens past
   0.3 (stab 0.4, overhead 0.5) once a swing: with weapon class bit 0x80
   (fists, claws, two weapons) only the motion's hand (the overhead's
   first held item, both with claws; the main side's and the stab's
   first; the combos' second), which becomes the primary striker;
   otherwise every held item. Every clip start shuts it.
6. Remise: a fighter with the perk, its swing neither released,
   stalling, continued nor started without Remise (0x80000), chains the
   other side's attack past 0.8 of a side attack: one of that side's
   three clips without repeat, with round(6·W18) + 2 frames of blend
   (smooth), its primary kept and its strikers' windows carried.
7. The stall: the swing strength P loses the stall rate (floor 0); once
   the game clock is past the stall's end the stance starts (blend
   2·W19, smooth) and the state word clears. The rate × P.
8. Tiredness: under 0.8 of its most energy (double precision) the rate is
   0; under its most it is (E/Emax)² in place of the rate.

Off its balance (under 0.6, double precision) the body tick takes its
falls branch instead of the attack branch: no routine runs, and an
attack's clip runs 1.25 × base a tick (the clock skips the 0.6 for
attacks), also an attack started off balance, whose condition is not
attacking yet.

A swing stalls three ways. **Released**: when the attack request drops
during the attack (the player's button up; the AI's choice), unless it
recoiled, cleaved or was released already, P = 1 drains by max(1 −
0.05·|v|, 0.1)/W17 × 0.03 a tick (at least 0.01, |v| the first striker's
tracked speed), the stance due round(1000/(80 × stall)) ms later, at most
the expected rest of the swing; a released main-side attack also returns
to the stance from the behaviour once that time has passed (2·W19, 30
frames without a primary striker), condition normal. Feint may renew it
after round((2f + 1) × (round(100·W17) + 100)) ms, f = S(progress/0.7) up
to 0.7 (1 after, out of an attack, or in a continuation past its carried
window). **Struck**: a daze D (capped at 0.9) on an attacking fighter
with P not spent adds r = 4·D·f − (1 − D)·min(0.005·s, 1)·W17 (f = 0.25
over its most health, ×0.5 under a potion; s the strikers' larger tracked
speed squared) when r ≥ 0.001: the first stall is r, a further one adds
up to 1; the stance due round(80·P/stall) ms later (the original's 80),
at most the expected rest. (Recoil and the follow-through after a wound
come with C21/M26.) The expected rest is (1 − progress) × T_last / (80 ×
W15 × ḡ × 1.125 × base) × 1000 ms, ḡ the mean of g over what is left
((g + g_end)/2 past 0.5, else ((0.5 − p)(g + 1) + (g_end + 1)/2)/(1 −
p)/2, g_end = 1 − 0.62475·W18): 849 ms for a 110-frame punch from its
start. The clip start does not change the condition, so the behaviour
tick after a stall's stance releases once more, as the original's.

Attacks start eased smoothly: the main side with no blend, the other side
with round((20 − W19)·0.6) frames (W19 the striker it makes primary: 12
for the fists), the stab and the overhead with the executor's blend (10
for Feint's renewal, else none); every start sets P = 1, no stall and a
shut window, and a fresh side attack without Remise the state word
0x80000.

| state word bit | meaning |
|---|---|
| `0x1` | the window opened this swing |
| `0x2` | stalling |
| `0x4` | a landed blow cleaves on (C21) |
| `0x10` | released |
| `0x20` | recoiled (M26) |
| `0x80` | past 0.7 in the window region |
| `0x10000` | a chained swing, its window carried |
| `0x20000` | a chained swing past 0.25: no further chain |
| `0x80000` | started without Remise: no chain |

Outside attacks the behaviour keeps only the released, recoiled and
`0x40000` bits.

**Stance starts.** Evidence: `exe.asm` of the set-stance helper, the body
tick's posture and daze blocks and the AI's combat state, constants read
from `Exanima.exe`. Each starts the stance's clip as above:

| start | blend (frames) | ease |
|---|---|---|
| the set-stance helper (Tab, an AI entering combat), only when the old stance's clip plays | 30 | smooth |
| the posture switch (a balanced body in another looping clip) | 30 | linear (the start's own) |
| the AI's stance, on every pass while it is free | 40 without a primary striker, else 2·W19 of the primary | smooth, written every pass even when nothing starts |
| the daze, every tick the energy is more than 0.05 under its most (double precision), unless down or attacking | round((1.05 − E)·300), ties to even (129 at E = 0.62) | smooth, written every such tick |

The daze also makes the stance the Defence stance every dazed tick (down
or attacking too), and the player's stance rule gives Cmbt - Idle only
when the energy is within 0.05 of its most. So a dazed body's clip is
the guard, whose blend the pose hold (above) hides while it stands; once
its energy is back the stance turns again and the posture switch blends
from where the body is, so its targets do not jump back to the clip in
one tick.

Not reproduced yet: the locomotion clock's stride modulation, its start
key and the run overlay (C18); the blow sag's writer, the swing's
weapon-in-hand wait (weapons are bodies with M24), recoil and the
follow-through (C21, M26); the attack arm stiffness; the torso
twist in the captures (identity without it); the femur posture flag; the
foot contacts' flag in the capture; the armour encumbrance; the creature
classes' own clock bases.

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
