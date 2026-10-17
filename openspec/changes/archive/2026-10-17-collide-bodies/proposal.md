## Why

Exarust's body parts meet only the level. Two characters' limbs pass
through each other, a corpse can be walked through and nothing pushes it,
a limp body's arms sink into its own trunk, and props, once they are
bodies, fall through the tables they stand on. In the original every part
is a body, and its "pairs" keep the parts of two bodies, and two parts of
one body, out of each other: detection after the 20 iterations, an
immediate push, up to four points per pair solved in the next tick's
iterations before the links, and a friction of their own before the
level's. A contact also wakes a sleeper, which Exarust's sleep (S1) left as
a hook.

Phase 3 (Bodies), milestone M3 "standing man". This is change C17 of the
master plan (lane E, physics), on the solver stages of M0; it is needed by
the walking drive (C18), weapons as bodies and their hits (C21, M24),
impacts (C22) and props pushed by bodies (M32).

## What Changes

- A pair stage in the body solver for every body (characters, corpses,
  props once they are bodies): after the iterations every awake part
  meets the parts near it (the 3 × 3 cells of a 6 m grid), awake or
  asleep, that it has not met this tick.
- Two parts of one body skip each other by the original's group masks,
  built from the tree (a part within three generations of another, or
  sharing an ancestor that close, never meets it); a character's two feet
  share theirs.
- Parts of two bodies skip each other by the original's layer masks: two
  characters' living lower legs pass through each other (dead ones meet
  everything); a character's feet never meet other parts.
- The more mobile part (the smaller key round(100/mobility)) brings its
  spheres against the other's; the original's other side is its mesh, and
  Exarust tests sphere against sphere until the parts' spheres are fitted
  to their meshes (C20). Each sphere where it is, or swept from the last
  tick when it moved over 0.1 cm² relative to the other part.
- Each hit pushes both parts at once by their mobility shares and the φ
  factor (how concentrated the contact is on each one's particles), which
  keeps their momentum; a pair keeps up to four points, spread widest.
- The pair pass in each of the next tick's 20 iterations, before the
  links: a point over 0.01 cm deep is taken back to 0.01 cm with the
  original's reversed pair friction; a pair needing nothing is dropped for
  the rest of the tick. The pair friction in each of the 8 passes, before
  the level's.
- A contact equalises the two bodies' sleep countdowns, so a living body
  touching a corpse wakes it.
- A character limb's contact mobility is its part factor × 2/3 (the
  original's normaliser for a human of about 70 dm³, until the parts'
  volumes are measured).
- Debug helpers: `exarust --pair-trace <file.csv>` (every contact found
  per tick) and `exarust --corpse x,y,z` (a dead body lying down in play
  mode).

Evidence: REA and `exe.asm` of `Exanima.exe` 0.9.5: the sector step's
broad phase (group and layer tests, the order key from add-to-world), the
contact generator and its per-hit push, the pair manifold, the pair pass
and its two variants, the pair friction, the group-mask builder, the
layer setter of legs and feet, the part-mass setter, the wake helpers;
constants read from the PE (0.01, 0.99, −0.01, 0.1 cm², 10⁻⁵ cm², 2.0,
100.0, 1.19·10⁻⁷, 0x55555550, 20.0, the part factor table);
`context/re/body-pairs.md` §3–§11, `master-physics.md` §13. Unit tests
pinned to those numbers; light runs of the duel and of a corpse walked
into.

Out of scope:
- spheres against the touched part's mesh and spheres fitted to the part
  meshes (C20);
- held weapons, shields and torches as parts, the striker and hittable
  classes and the hit events (C21, M24); impacts, their sounds and damage
  (C22);
- the placed state (one-sided pushes of settling items) and the AI's and
  placement's overlap queries;
- Break Tangle, the two-handed grip bits, biting weapons' layers,
  Constant Displacement;
- the capsule between living characters (removed by C18).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: contacts between bodies, their masks, push, pair pass
  and friction; the tick order and the sleep wake gain the pairs; the pair
  trace.
- `world-viewer`: debug corpses; the prop shove's outcome now that props
  meet each other.
- `object-physics`: props meet bodies through the pairs (in place of
  meeting only the level); their stand-ins are the character capsules'
  only.

## Impact

- `exa_physics` (new `body_pairs.rs`: `Pairs`, `Pairing`, `Touch`; `Part`
  gets its `pairing`; `Ragdoll::new` builds the group masks and the legs'
  layers, `go_limp` drops the lower legs' layer; `SolverStage::detect` and
  `friction` also see the sleeping bodies, `solve_among`; `step_bodies`
  passes the sleepers without marking them changed; the level and pair
  manifolds share `contact::evicted`)
- On props as bodies (M32) and the collapse (L5), merged before it:
  `Ragdoll::rigid` gives each part its pairing (its mobility) and the
  group masks of its tree, bodies no longer meet the rigid bodies'
  stand-ins (the stand-in rule M32 kept until bodies collide is gone), and
  the collapse's start puts the lower legs in layer 0
  (`exa_combat::collapse`)
- `exa_game` (new `pair_trace`)
- `exarust` (`--pair-trace`, `--corpse`)
- `docs/formats/RSQ.md` (Physical body: contacts between bodies, sleep),
  `docs/DEBUGGING.md`, `docs/PLAN.md`, READMEs of `exa_physics`,
  `exa_game`, `exarust`
