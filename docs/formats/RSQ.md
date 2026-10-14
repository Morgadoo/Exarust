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

All values are in cm. **Both points of a link are in the part's own rest
frame.** The parent's point reaches the parent through the part's rest
placement under it. In the rest pose the two points of every link lie
within the slack. Each bone has 2 links; each hand has 3. One link is the
pivot (knee: slack 0; hip: 3 cm). The other limits the joint's range
(femur 14.9 cm, shin 12.7 cm).

How the original simulates a body (`exa_physics::body`):

- Every part is a rigid tetrahedron of 4 particles, kept rigid by its 6
  edge lengths. The particles start from a regular template laid over the
  part mesh's box (each extent at least 6 cm) and scaled by 1.25. The part's
  frame follows the particles through the inverse rest matrix.
- Verlet integration at 83 Hz with gravity −9.80665 m/s². Velocity retention
  per second for characters is 0.2556 / 0.15 / 0.03, by speed (over 40, over
  20, and under 20 cm/s). Slow bodies are damped hard, which acts as sleep.
- Each tick runs 20 link iterations. A link is a hard radius: past its slack
  it is corrected fully, the correction split by the parts' inverse masses
  (relative masses: pelvis 0.45, thighs 0.7, shins 1, feet 2.8, lumbar
  0.45, thorax 0.5, head 1, shoulders 1, upper arms 1.2, forearms 1.5,
  hands 2.5).
- Each tick then runs 8 contact iterations of sphere colliders against the
  world. Friction cancels slip up to μ·(n_y²·0.99 + 0.01) times the contact
  correction; past that, the limit is taken off. There is no restitution.
- Dead characters (condition `0xA00`) are plain ragdolls without muscles.
- Living bodies have muscles:
  - Gravity is compensated by muscle tone.
  - A pose drive pulls each particle toward its rest vertex under the part's
    animated transform. The gain is 0.084 per tick (0.07 on the hands),
    scaled by tone² and by energy⁴.
  - Energy is the daze level. A blow takes up to 90 % of it, with a floor
    of 0.05. It recovers 0.005 per tick.
  - The original also runs a PD muscle per link toward the animated anchor,
    balance with recovery steps, foot planting, and hidden support forces
    on the pelvis and thorax pins.

Exarust's muscles (`exa_physics::body::Drive`) are a pose drive with
critical damping relative to the pose's own motion:

- The pelvis drive is 2× stronger and stands in for the support forces.
- The body is carried rigidly with its character controller's motion, so
  muscles only work against the pose and outside forces.
- Animation clips play in place: the root's horizontal travel over a clip
  is removed, since the controller moves the body.
- Balance follows the original's rule (`exa_game::balance`):
  - The centre of mass is projected 0.1 s ahead by its velocity and
    measured against the segment between the feet.
  - The threshold is (energy·0.75 + 0.25)·30 cm. The score is stability⁸.
    Exarust takes stability as 1 within half the threshold, falling to 0
    at 1.5 thresholds; the original's exact measure is unknown.
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
bone j. The inverse bind pose is relative to the part's rest pose. Apparel
takes each vertex's weights from the nearest body vertex of its bone. The
original runs a similar builder for apparel; it also hides covered body slots
(per-slot coverage bits of equipped items) and culls apparel triangles by the
cull bits. Neither is done yet.
