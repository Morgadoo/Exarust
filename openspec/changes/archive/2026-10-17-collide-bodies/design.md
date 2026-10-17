## Context

The original's sector step runs body–body detection after the 20
iterations: every awake body (a part, in Exarust's terms) against the
bodies in the 3 × 3 cells around it that it has not met this tick, with a
group test inside a tree and a layer test between trees, then contact
generation, which pushes at once and keeps a record of up to four points.
The records are the pair pass of the next tick's iterations (before every
root's links) and the pair friction of this tick's 8 passes (before the
world friction). The research notes, with addresses, are git-ignored:
`context/re/body-pairs.md` (§3 broad phase, §4 group masks, §5 layer
masks, §6–§8 generation, push and manifold, §9 pair pass, §10 pair
friction, §11 wake and events), `master-physics.md` §13, the master
plan's E2 and its M0 note (the solver stages this change plugs into).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body,
contacts between bodies).

## Goals / Non-Goals

**Goals:**
- the pair stage on M0's solver stages, generic over every `Ragdoll` (so
  props join when they are bodies);
- the original's group masks, layer masks (living lower legs, feet out),
  order key, immediate push with shares and φ, four-point manifold, pair
  pass (0.01 cm slop, reversed friction, dropped once satisfied), pair
  friction, wake by contact;
- a trace of the contacts and a way to lay a corpse down for in-game
  checks.

**Non-Goals:**
- the touched side's mesh and fitted spheres (C20);
- weapons as parts and their hit events, impacts (C21, C22, M24);
- the placed state, overlap queries, Break Tangle, grips, bites.

## Decisions

- **One stage, `Pairs`, registered by `BodyPlugin`.** It implements
  `detect` (find and push), `pair_pass` (solve), `iterated` (drop the
  records, the original's pair list cleared before detection) and
  `friction`. `Ragdoll::tick` and `solve` keep running without stages, so
  every single-body test is unchanged.
- **Sleepers in detection and friction.** `SolverStage::detect` and
  `friction` take the sleeping bodies as a second slice (`solve_among`;
  `solve_with` passes none). `step_bodies` reaches them with
  `bypass_change_detection` and marks one changed only when a contact
  woke it, so the many sleeping props stay unchanged and are not redrawn.
  A woken sleeper is pushed and rubs this tick and is solved from the
  next, as the original re-adds it to the solver.
- **Bodies known by key.** Records live from one tick's detection to the
  next tick's iterations, while the awake list changes. A body is known by
  its first part's entity; at the first pass each record is mapped to the
  bodies being solved, and one whose body is gone or asleep is skipped.
- **Per-part data in `Part::pairing`.** Group and layer masks (later lanes
  set them: held items all-ones, bites 0x40/0xC0, Break Tangle), "out"
  (a character's feet), the contact mobility, and the sphere set's
  bounding sphere as weights of its centre plus a radius, so a sleeper's
  bound is found without rebuilding its frame. `Ragdoll::new` builds the
  masks from the tree and registers the legs; `go_limp` sets the lower
  legs to 0 (death, as the original's layer setter does).
- **Feet are always out.** The original sets a foot's out-of-pairs state
  at registration and every tick its leg finds ground, and no code clears
  it (no clear of that bit of the state word in `exe.asm`); so a
  character's feet, alive or dead, never meet other parts.
- **Contact mobility 2/3 × part factor.** The original's part mass is the
  bone factor × n/((20·size³ + Σ volume/10⁴)·mass divisor); the volumes
  come from voxelising the part meshes (C20). For a human (18 parts,
  about 70 dm³, size 1, divisor 1) the factor is 18/27. Exarust keeps the
  relative part factors for the links (only ratios matter there) and
  multiplies by 2/3 for contacts, so limbs meet props (whose stored
  mobility is absolute) on the original's scale.
- **Sphere against sphere.** Until C20, the touched side's "mesh" is its
  spheres: a static sphere is pushed off the other sphere along the line
  of centres; a swept one that enters it is pushed off where it ends while
  it ends on the near side, else taken back to where it first touched (as
  the level's swept test takes a crossing sphere back). Each sphere of
  the sphere side is first tested against the other side's bounding
  sphere. The original's box pre-test is only a filter of what the exact
  tests find, and is left out.
- **Reversed pair friction copied.** The pair pass's friction cancels a
  travel over √L and barely damps one under it, the reverse of every other
  friction rule; it is copied as observed (the plan's decision), and the
  pair friction of the 8 passes keeps the plain rule.
- **One manifold rule.** The level's and the pairs' four-point manifolds
  share `contact::evicted`.
- **Touches for later lanes.** Each tick's contacts are kept (`Touch`:
  both sides, hits, mean point, pushes summed and the longest) for the hit
  events and impacts (C21, C22) and the pair trace.

## Open reverse-engineering questions

| question | how it was answered |
|---|---|
| The pair pass's slop and friction, and when a pair is dropped | `exe.asm` of the dynamic pair solve: depth compared (double) with 0.01; L = (0.99·n_y² + 0.01)·μ; `jae` to the scaled branch when L ≥ |r_t|²; a record with no work gets count 0; both bodies restore their edges after work |
| The pair friction's rule and gate | `exe.asm`: v = B's displacement − A's, tangential, scaled by 0.01·L only when L < |v_t|², raw weights, shares m_A/(m_A + m_B) and −m_B/(m_A + m_B); records skipped when μ_A·μ_B < 1.19·10⁻⁷ |
| The immediate push | REA of the per-hit callback and its helpers: weights clamped and normalised by their sum, then divided by their largest (the helper returns the reciprocal: φ); f = φ_A·2/(φ_A + φ_B); A × share × (2 − f), B × −share × f; push length into the 0.1 cm edge-restore accumulator |
| Group masks | REA of the builder: leaf = parent's bit; non-leaf = p | g | next bit (counter & 31); children built with (own bit, p), then OR p | g; checked against the notes' human matrix |
| Layer rule | `exe.asm` of the broad phase: ((a & b & 0x55555550) << 1) & (a | b) |
| Order key | `exe.asm` of add-to-world: 100.0 / mobility, `cvtss2si` (half to even) stored as 16 bits; smaller key is A, tie A = the awake body |
| Feet out while grounded, or always? | setters of the state bit at registration and in the feet routine; no clear of it anywhere in `exe.asm` (no `and` with its complement on the state word, no `btr`): always |
| Living lower legs | the layer setter writes the mask to each foot-contact part and mask \| 0x30 to its first child; registration writes 0x300 and 0x30; death and collapse write 0 |
| Part mass normaliser | REA of the part-mass setter: table[bone] × n/((20·size³ + Σ 1/m)·divisor), link mobility 0.175×, contact mobility 1×; constants 20.0 and the table read from the PE |

## Risks / Trade-offs

- Fat spheres. Exarust's part spheres lie along each part box's long axis
  with the box's thickness; they are larger than the original's fitted
  ones, so a hand rests on its thigh where the original's might clear it
  (the duel: the duellist's hand on its thigh for 52 ticks, under 1.5 mm).
  C20 fits them.
- Cost. A grid of the awake parts' cells picks the sleeping parts to
  test; sphere against sphere is all pairs of two parts' spheres, cheap
  for limbs (1–4 each), heavier for props with hundreds once they are
  bodies.
- The capsules still keep living characters apart until C18; limbs that
  reach past them now meet.
- Corpses walked over are barely moved: the controller carries the living
  legs over them and the pushes go into the floor (the original's walking
  drive comes with C18).

## On props as bodies and the collapse

Props became bodies (M32) and the collapse came (L5) before this change
landed. A rigid body's part takes its stored mobility (over its divisor)
as its contact mobility and its tree's group masks (an articulated child
never meets its parent); its layer is 0. M32 kept the rigid bodies'
meshes as stand-ins that character bodies met and props did not, until
bodies collide: now no body meets a stand-in (the capsules still do), so a
character's part meets a prop once, through the pair, and a prop rests on
a prop. The collapse's start puts the lower legs in layer 0, as the
original's collapse start calls the layer setter (death already did).
`arenao1`'s shoved tableware: 8 shoved, six back on the table, two on the
floor, all asleep after 317–324 ticks, one prop–prop contact (1.9 cm).
