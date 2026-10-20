## Context

The original's pathfinder answers, besides planning, five questions on its
grid (re-read in REA for this change, `context/re/master-movement.md` §22;
formats page: [docs/NAVIGATION.md](../../../docs/NAVIGATION.md), Queries).
Each layer of a cell holds a floor byte and an underside byte (10 cm voxels
over the grid's base); the planner's queries read the pair after moving props
have raised floors.

**Line walk** (the player's look-ahead, its soft stop, the footwork's step
targets, the AI's footwork candidates). The start's exact grid position
(x/30 − ½ + W/2) and the end's cell; the axis the line covers more of is
the major axis; one cell per step along it, the other axis rounded from the
line through the start's exact position (single precision, ties to even).
The running height starts at round((y − base)/10) of the start (the
difference in single precision, the scale in double). Per cell: no lower
floor → passed over; a lower floor more than 4 voxels over the running
height blocks when its underside is less than 10 over it, else it is not
stood on; a reachable lower floor is offered; an upper floor more than 4
over blocks when its underside is less than 10 over it, unless the
underside is 0 and the floor is not below the lower one, and otherwise
passes (only if the lower was stood on); a reachable upper floor offers the
higher of the two; the running height becomes the higher of itself and the
offer. Blocked at the first cell, the end is the start (all three
coordinates); later, the major coordinate steps back one cell, the minor is
the line's there, and the end is the start plus the unit direction (across
the ground) times the dot product of that cell centre's offset with it,
keeping the asked end's height.

**Straight walk** (walkers without walking feet, the race whose record marks
it). The start's layer: the upper one when its height (voxels) lies above
the upper underside less 3 and below the upper floor plus 15; the running
height is that layer's floor at the start cell. As the line walk with 5 and
15, no exception for an open underside, and the running height set to the
offer (following the floors down). It reads the floors before props.

**Obstacle.** Lines whose length across the ground is 40 cm or less: none.
Else as the line walk from the start's height with 7 and 15 (an upper slab
with an open underside passes only when its floor is not below the lower),
the running height set to the floor stood on (the upper when reachable and
higher), and a cell before the line's last two whose penalty on the layer
stood on exceeds 60 is an obstacle.

**Cost.** The cell under the point (no bounds); the layer by the same height
test as the straight walk's start; (clearance, penalty) × 1/255 (double,
stored single).

**Nearest walkable point.** The layer a walker at the point's height stands
on: none without a lower floor or with only open sky; the lower without an
upper floor (open sky counts as none); the upper when the lower floor lies
less than 11 voxels under the upper slab; the lower when the lower slab lies
less than 11 over the upper floor; else the nearer floor. With a layer and
clearance + penalty under 80: the point, its height the floor's. Otherwise a
256 × 256 window centred on the point's cell (round(x/30 + ½) − 1 + W/2, the
window's cell 127); best-first expansion from the start, up to 220 cells
met: a neighbour (eight, in order) is stepped onto by the line walk's
reach (40 cm up, 100 cm under a slab), the upper floor when reachable and
higher, if the step rises less than 5 voxels and drops less than 21 and the
cell layer is new; its key is round(10·√(Δx² + Δz² + Δh²)) from the point;
the cheapest cell met (strictly) is kept; once it costs under 80 after a
cell's expansion, or nothing waits, the answer is its cell centre at its
floor's height. The waiting cell with the least key is expanded next (of
equals, the one met last). Off the floors (no layer, or clearance 255) a
first search takes the cell nearest across the ground that is not open sky
and has more than 10 voxels between its floor and the other layer's slab
(a lower layer without a floor qualifies), within the shrinking distance of
the best, and the flood restarts from it; none → the point unchanged.

**Look-ahead** (exploration steering, move requested, not stepping): goal =
standing position + facing × (6·v² + 75) cm (−70 with backwards or
turn-about); the line walk (the straight walk for the footless race) from
the standing position on the ground to the goal; free walk = not blocked;
blocked and the goal within 100 cm (3-D squared distance) → run and sprint
latch cleared, the brake kept; else (and without turn-about) the brake
cleared. Combat mode's right-button walk: 6·v² + 45, the line walk always,
free walk off when blocked, the same run cut. The soft stop clamps its goal
with the line walk (its answer ignored).

**Spawn.** The behaviour's tick counter counts to 3; on reaching 3, unless
the condition has 0x200, the character frame's origin is moved to the
nearest walkable point and the body re-placed there.

**Footwork step start.** Target = chest + direction × step length × √scale
(the step length is the step reach times the leg factor, × 0.7 crouched,
× 1.5 − 0.3·enc long); a forward step with a target is first clamped by the
target's personal circle; the target is clamped by the line walk from
(chest x/z, ground); the step starts only when the target lies more than
20 cm (3-D) from the chest.

## Goals / Non-Goals

**Goals:** the five queries with the original's rules, numbers and
rounding, as plain functions of `NavGrid` with unit tests on synthetic
grids and a corpus test; the fallback without a grid; the look-ahead, soft
stop, spawn snap and footwork targets through them; the overlay and trace
columns to see it.

**Non-Goals:** planning, footprints, props in the grid (M6, M32), combat
mode's walk (M2), the original's footwork steps (C16), the AI (M7).

## Decisions

- **Plain functions of the grid** in `exa_world::nav::query` (methods of
  `NavGrid`, plus `Nav(Option<&NavGrid>)` answering as the fallback); no
  systems or resources, in the original's centimetres and voxels with points
  in metres at the edges (the grid's `centimetres_of` keeps boundary points
  exact). The behaviour's closure turns a walk into `(end, blocked)`, so the
  steering is unchanged in shape and its tests use plain closures.
- **The original's register leftovers** are kept where they cannot matter
  (the offered floor starting as the cell count, read only for a lower floor
  out of reach under a reachable upper one, which an ordered grid never
  has) and left out where they would make the answer arbitrary: the
  off-floor search adds to every distance the square of a register it never
  sets (the same for every cell: the nearest cell still wins); Exarust
  leaves it out. The flood's met-cell map is fresh each call (the original
  clears only the window's middle 33 × 32 cells of a shared buffer).
- **Exact normalisation** where the original uses the processor's
  reciprocal square root estimate (the clamp's direction; within 0.04 %,
  under a millimetre at the look-ahead's lengths).
- **Spawn snap across the ground only**: the original re-places the body on
  the floor's height (10 cm voxels); Exarust moves its capsule across the
  ground and leaves its height to the capsule, which stands on the level's
  collision. Exarust's grid is built off the main thread after the load, so
  the snap counts the behaviour ticks that have a grid (the original's
  exists before its first tick). A living body is carried with its
  character's root, so the capsule's move carries the body.
- **The run cut across the ground**: Exarust's standing position is at the
  feet, the original's at the pelvis, so the original's 3-D compare differs
  only when the first cell blocks (the goal falls back to the ground under
  the standing position: a pelvis 1 m up or more is then not cut); Exarust
  cuts it there.
- **Footwork targets from the feet**: the interim footwork has no chest
  point; its target starts at the feet and its 20 cm test is across the
  ground (blocked at the first cell, the original's step goes ahead with a
  target under the chest, which moves nothing; Exarust's interim footwork
  does not walk either). The interim capsule footwork walks while the
  target of a tap step along its keys lies more than 20 cm away.
- **The capsule meets walls**: the clamped goal can lie within the capsule's
  28 cm radius of a wall's face (in the arena 2.5 cm), so the capsule stops
  against the wall short of its goal, still braking, and the decision stops
  it as braked once its commanded speed drops under 1 cm a tick. This is
  the capsule's (C18 removes it); the rules are the original's.
- **Overlay in `exa_game`**: the behaviour's last line walk is kept on the
  steering (what was asked, where it ended, whether blocked) and drawn by a
  debug plugin when the grid overlay is on; the blocking cell is found by
  walking the same line again.

## Risks / Trade-offs

- [The grid and Exarust's collision disagree] → the look-ahead follows the
  grid, as the original's; the capsule still collides with the level.
- [A start off the floors] → the original's off-floor search can land on a
  floorless cell at the grid's base; the corpus test shows the shipped
  starts are on walkable floors, and the snap moves only across the ground.

## Open Questions (answered)

- Which byte pair the line walk reads, and whether the floor is the high
  byte → REA on the layer builder: pairs (underside, floor), filled from
  the static floors before props; the straight walk reads the static floors.
- "Obstacle within 40 cm" (master plan) → REA: lines of 40 cm or less meet
  none; it walks the whole line (70 cm rise, 150 cm room, penalties over 60
  before the last two cells). Recorded as a correction in the notes.
- "First three ticks only" (master-movement §4.1) → REA: once, on the third
  tick (the counter reaching 3), unless down.
- The rounding of every step (cvtss2si, ties to even; the height scale in
  double) → read in `exe.asm`.
- RR4 (retail: walk into a wall in exanima01, the goal, free walk, brake
  and final gap) → not taken: the retail game was not running.

## Port onto main (2026-10-20)

Finished on 2026-10-17 and left unmerged; ported onto a main whose own
lanes had meanwhile written the line walk, the straight walk and the
obstacle test (`exa_world::nav::query`: `line_walk`, `straight_walk`,
`obstacle`, the same rules branch for branch) and a nearest walkable point
that looked only at the eight neighbours, and had put the player's
look-ahead on the grid (`look-ahead-on-the-grid`). What was ported:

- `exa_world::nav::walkable`: the cost at a point (`cost_at`), the layer
  pick (`layer_at`, now public) and the nearest walkable point as the
  original's flood (220 cells, the planner's steps, the off-floor search
  first), replacing main's one-ring version under the same name; main's
  queries keep their names (`nav::line_walk` and friends), so this
  change's `NavGrid::walk_line`/`walk_straight`/`obstacle_on` and the `Nav`
  wrapper were not ported: callers without a grid pass none (the
  player's line walk falls back to the hip ray, which `look-ahead-on-the-
  grid` keeps).
- The player: the spawn snap on the third behaviour tick, the footwork's
  step targets (none within 20 cm), the soft stop's goal clamped, the
  standing point from the simulated feet (`CharacterBody::feet`), the
  line walk recorded for the overlay, `--trace`'s `goal_x`/`goal_z`.
- The overlay draws the player's line walk but not the blocking cell
  (main's line walk does not return it).
- Not ported: the corpus test over the shipped levels (main's grid build
  has changed since; its numbers were not re-measured here, as no local
  build may run).
