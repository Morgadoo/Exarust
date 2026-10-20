## Context

The original's planner, read in REA for this change
(`context/re/master-movement.md` §23; formats page:
[docs/NAVIGATION.md](../../../docs/NAVIGATION.md), Planning, Footprints, One
plan a tick, Moving things). Units are the grid's: 30 cm cells, 10 cm voxels
over the grid's base, costs as plain integers.

**Window.** A point's cell is cvtss2si(x·(1/30) + ½) (single precision, ties
to even) from the grid's middle; start and goal cells must satisfy |c| ≤
W/2 − 10 (and D/2 − 10) and |goal − start| ≤ 253 per axis, else no plan.
The window's centre cell 127 is start + (goal − start)/2 (integer division
toward zero); its first grid cell is (W − 256)/2 + centre. The goal's height
is round((y − base)/10) (the difference in single precision, the scale in
double). The start's layer: upper when its height lies above the upper
underside less 3 and below the upper floor plus 15.

**Expansion.** Node records (window x, z, layer, clearance byte, cost, total,
next, from) and a map of the node met per window cell and layer. The start
is node 1, expanded first (never on the waiting list). Each neighbour in the
order (dz, dx) = (−1,−1), (−1,0), (−1,1), (0,−1), (0,1), (1,−1), (1,0), (1,1)
(direction costs 7, 5, 7, 5, 5, 7, 5, 7): the step rule of the queries'
flood (lower floor 0 → none; over own + 4 blocks under own + 10, else out of
reach; upper 0 → the lower if in reach; upper over own + 4 blocks under own
+ 10 unless its underside is 0 and it is not below the lower, else the lower
if in reach; upper in reach and higher → the upper), Δ in −20..=4, clearance
≤ 180/(size + 1) (integer division). Cost = 10·Δ² + direction + 2·clearance
+ the current node's cost + (penalty << shift), the shifted penalty compared
signed with 600. A new cell: node count + 1, abort when over 4095 (the
waiting list emptied); its distance h = cvtsd2si(10·√(Δx² + Δz² + Δh²))
to the goal (window cells, voxels from the goal's height); the nearest so
far kept when strictly nearer; total = h + cost; it goes to the list's head.
A met cell: skipped unless the new cost is strictly lower (compared as
unsigned 32-bit against signed 64-bit), then its total moves by the cost's
change and it comes from the current node, expanded or not. After the eight:
nearest h < stop distance → success; empty list → stop; else the list is
scanned from its head for the least total (strictly less: of equals the
one met last) and that node is unlinked and expanded.

**End.** No node met → no plan. Not a success: the nearest node's centre
((window centre + x − 127.5)·30, floor·10 + base) is tested by the goal's
own test, or by its 3-D squared distance to the goal over 10 000 cm²; a
rejected end walks back along `from` to the start, the end moving to a node's
`from` whenever that node's clearance exceeds 64/(size + 1). (A loop that
looks as if it walked back from the end by clearance steps of 4 never
changes the end: its node index is never advanced.)

**Corners.** From the end back to node 1: a node whose direction from the
previous one (window cells) or layer differs starts a new corner, otherwise
it replaces the last; the list ends with the start. Fewer than two corners:
no path. String pulling from the start (the anchor): each corner from the
second after the anchor's to the end is tested against the anchor by the
line test; when it fails the corner before it becomes the anchor and a
point. **Line test**: from the far corner to the anchor, rasterised as the
queries' lines (major axis = z when |dx| ≤ |dz|, step by the sign bit, the
minor rounded from the line through the far corner, single precision);
heights from the far corner's floor (never updated); a cell without a lower
floor blocks; the step rule's blocks block; the cost adds 10·Δ² + 5 or 7 (by
whether the minor moved) + 2·clearance + (penalty << (shift + 1)) and fails
over the search's cost difference + 10.

**AI planner.** Not while down. Only the slot's holder: lifts its own
footprint, plans from its ground point, stamps it again, follows a path of
two points or more when told to or when its ends lie over 150 cm apart
across the ground, notes the tick. Others record their wait (sim ticks since
their last plan) as the longest when strictly longer. The simulation step's
callbacks hand the slot to the longest waiter and reset the wait after the
bodies' ticks. AI defaults: size 3, shift 1, stop 10.

**Footprints.** The stamp moves an existing footprint (removes it first).
Centre cell cvtss2si(W·½ + (x·(1/30) − ½)); outside 5..W−5 (and D) nothing
is stamped and an error is logged once. A 5 × 5 patch: the centre's layer
(lower first) by underside − 3 < h < floor + 25 with h the character's
height; centre 255; its eight neighbours (the four along the axes first) not
the centre: 128, or 255 when already ≥ 128; the next cells along the axes
from those, unmarked: + 64 each, capped at 255; each value / 4 added to the
penalty byte (wrapping); removal subtracts. Callers: every AI behaviour
state (dead and collapsing ones too), the player's exploration (lifted at
its start, stamped at its end unless walking) and combat (the same, also
while getting up).

**Movers.** A rectangle from an oriented box: per axis (dim/2) +
round((c ∓ e)·(1/30) ∓ 2.5), clamped to [3, dim − 4]. Moving props: a
16-entry ring (write index mod 16). Others: merged into the first list
rectangle whose union, sides + 6, is smaller in area than the sum, else
appended (skipped while a full refresh is pending). The pathfinder's tick:
the ring drained into the list; each rectangle grown by 3; working floors =
built floors; the scene's prop points in its 6 m rooms rasterised (cell
and voxel rounded as the build's; the raise rule); where working ≠ current
floors anywhere in the rectangle, the floors are copied and the cost passes
run over it; the list cleared.

## Goals / Non-Goals

**Goals:** the planner, footprints, the slot and the refresh exactly as the
original's, as plain functions of the grid with unit tests pinned to the
binary's numbers; a debug probe and a tool to plan on real levels.

**Non-Goals:** following paths (M7); movers marking rectangles (M32, M12);
the AI's goal tests (M10); the full refresh countdown.

## Decisions

- **Plain functions.** `NavGrid::find_path`, `stamp_footprint`,
  `remove_footprint`, `rect_around`, `queue_moved`, `mark_dirty`, `refresh`
  and `PlanSlot` take no Bevy systems or resources (Bevy maths only, as the
  queries), so M7 and the movers call them from their own systems, and
  tests drive them directly.
- **The search's state is per call.** The original's node records and cell
  map are shared globals; Exarust allocates them per plan (one plan a tick
  makes that cheap: 256 KiB of map).
- **The waiting list** is a vector whose end is the list's head, scanned
  from the end with a strict minimum: the original's order of equals.
- **Footprints live in `NavFootprints`** (entity → footprint, cleared with a
  new level's grid); `exa_game::footprints` runs after the character ticks
  (the behaviours' end of tick), so the spawn snap and the planner read last
  tick's footprints as the original's do. Exarust's standing point is the
  feet (the layer test's window, −30 cm to +2.5 m, holds the pelvis's
  height too).
- **Built floors beside the movers' floors.** The grid keeps the floors as
  built (the original's static floors) so a refresh can restore them; the
  straight walk reads them, everything else the movers' floors.
- **Slot and refresh** run in `FixedPostUpdate`, after the fixed tick's body
  ticks, as the original's step callbacks and pathfinder tick do.
- **`PathPlan.reached`** (Exarust's diagnostics: the original keeps its
  success flag to itself) and `cost`, `nodes` for logs and tests.

## Risks / Trade-offs

- **Window edge** → Exarust stops at the window's edge where the original
  wraps its window index and reads an uncleared outer ring; documented,
  reached only by searches spreading 127 cells.
- **Doors in the grid** → exanima01's doors are static level geometry in
  Exarust, so its rooms are islands until M12 (its start room: 2039 cells).
  Whether the original's grid stamps door leaves too is M12's question.
- **Greedy routes** look odd next to shortest ones (many small corners
  where the clearance varies); that is the original's weighting, kept.

## Open questions (answered)

- Which line test does string pulling use, and with what budget? → REA: its
  own (heights from the far corner, penalties shifted once more, no Δ
  limits, no-floor blocks), the search's cost difference + 10.
- Is "one plan a frame" per rendered frame? → REA: the hand-over is a step
  callback of the simulation, before the step counter advances; the waits
  are counted in simulation steps.
- The footprint's patch size and ring rule → REA: 5 × 5 (AI constructor),
  the axis neighbours first, ring 2 along the axes only, unmarked cells,
  + 64 per neighbour.
- Who stamps → REA: every AI state; the player unless walking (exploration
  and combat), lifted at the start of each behaviour tick.
- What sets the full-refresh countdown → not found (its slot is not written
  in the pathfinder's code); left out.

## Port onto main (2026-10-20)

This change was finished on 2026-10-17 but left uncommitted in its
worktree. By the time it was salvaged, main's own lanes had written the
planner's search, the footprint stamp and the movers' rectangles and
refresh in their own shape (`exa_world::nav::path::find_path`,
`nav::footprint::stamp_footprint`/`remove_footprint`, `nav::mover`, and
the path shortcut, end fix-up, budget and layer choice in `exa_ai::path`),
with the same constants. Porting this change's planner, dirty rectangles
and plan slot would have duplicated them, so only what main lacked was
ported:

- **The footprints kept by the characters** (`exa_world::nav::
  NavFootprints`, `exa_game::footprints`): each fixed tick every character
  under its controller moves its footprint to where its feet stand, an
  NPC always, the player unless it walks; a character that leaves takes
  its footprint off; a new grid starts with none. The system runs in the
  behaviour stage (after the player's behaviour, before the character
  ticks), where the original's behaviours lift and stamp, rather than
  after the character ticks as this change had it: the footprint marks
  where the character stood when it decided, as the original's.

Not ported, with what main has instead: `NavGrid::find_path` with its
goal test and its string pulling (main: `nav::find_path`, and the
shortcut in `exa_ai::path`), the plan slot handed over once per
simulation step (none: nothing plans in the running game yet), the
dirty rectangles' refresh in `FixedPostUpdate` (main: `nav::mover`, not
wired), `--debug-path` and `exa-tool nav --from --to`.
