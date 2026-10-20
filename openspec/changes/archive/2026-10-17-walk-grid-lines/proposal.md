## Why

The navigation grid is built for every level (`build-the-navigation-grid`),
but nothing asks it anything. The player's look-ahead is still Exarust's
stand-in, a ray from hip height clamped a capsule radius and a 30 cm cell
short of what it meets: it misses edges lower than the hip, sees nothing a
walker could step onto, and answers differently from the original wherever
the level's collision and its grid disagree. The spawn is not snapped onto
walkable ground, and the interim combat footwork walks into walls.

In the original the pathfinder answers these questions on its grid: a line
walk with its step rules that clamps the end before a blocking cell (the
look-ahead, the soft stop, the footwork's step targets), a straight walk
for walkers without feet, an obstacle test, the cost of a point's cell and
the nearest walkable point (the spawn snap). The look-ahead's clamped goal
switches off free walk, so the body brakes before the wall, and within
100 cm the run is cut.

Phase 3 (Bodies: skeleton, physics, control) and Phase 6's navigation;
the master plan's lane D change M5 (milestone MS2 "standing man").

## What Changes

- `exa_world::nav::query`: the original's grid queries as functions of the
  grid (no systems): the line walk (cell by cell along the major axis from
  the start's height; 40 cm rise, 100 cm under a slab; drops never block;
  the end clamped to where the line passes the last free cell's centre, or
  the start), the straight walk (from the floor under the start, following
  the floors; 50 cm, 150 cm), the obstacle test (lines over 40 cm: a floor
  70 cm up with under 150 cm under it, or a penalty over 60 before the last
  two cells), the cost at a point (clearance and penalty over 255 on the
  layer its height meets), the layer a walker stands on, and the nearest
  walkable point (the point on its floor when its cell costs under 80, else
  a best-first flood by the planner's steps, at most 220 cells); `Nav` for
  the original's fallback without a grid.
- The player's look-ahead walks the grid: the goal 75 + 6·v² cm ahead along
  the facing (−70 backwards; 45 + 6·v² for combat mode's walk, for M2),
  clamped by the line walk from where the character stands; blocked, free
  walk ends and the body brakes to the clamped goal, within 100 cm run and
  latch are cleared and the brake kept. The soft stop's goal is clamped the
  same way. The hip-height ray is gone.
- The spawn snap: on the player's third behaviour tick with a grid (unless
  down), the player moves across the ground onto the nearest walkable
  point.
- Footwork step targets: a step's length along the keys, clamped by the
  line walk; within 20 cm no step. The interim capsule footwork walks only
  while its step's target is farther.
- The steering reads where the feet are in the simulation
  (`CharacterBody::feet`) rather than the drawn transform.
- Debug: the navigation overlay (`--debug-nav`, F6) draws the player's last
  line walk (the way asked, the walk to its end, the blocking cell, the
  goal walked to); `--trace` gains `goal_x`, `goal_z`.
- Docs: NAVIGATION.md (Queries), PLAYER.md (Movement, Combat mode),
  DEBUGGING.md, formats/RSQ.md (trace columns), PLAN.md, crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and `context/re/exe.asm`, re-read for this
  change: the pathfinder's line walk, straight walk, obstacle test, cost
  sample, nearest-walkable search and its layer pick; the layer builder (which
  byte pairs hold floors and undersides); the exploration steering's and the
  combat walk's look-ahead; the footwork step start; the player behaviour
  tick's spawn snap. Constants read from the PE (`context/re/master-movement.md`
  §22).
- Corpus: arena, arenao1 and exanima01 grids from the installation (the
  starts on walkable floors, the walls the line walks meet).
- Exarust in game through `gpu_run.sh` (light runs, traces, the overlay).
- The retail game was not running: RR4 was not taken.

Out of scope:
- the planner (A\*), character footprints and dirty rectangles (M6), and NPC
  path following (M7);
- combat mode's right-button walk, its 30° rule and crouch (M2; the 45 cm
  reach is ready for it), the original's footwork steps (C16), the
  personal-circle clamp of a forward step;
- props raising floors (the original's straight walk reads the floors
  before props, the line walk after; Exarust's grid has no props yet);
- the body walking by its carry (C18): until then the capsule (radius
  28 cm) meets a wall before a goal clamped within its radius of it.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `navigation-grid`: the grid queries (line walk, straight walk, obstacle,
  cost, nearest walkable point) and their fallback.
- `player-movement`: the look-ahead on the grid, the spawn snap, footwork
  step targets; the capsule executor's wall contact.
- `physical-body`: the body trace's goal columns.
- `world-viewer`: the navigation overlay draws the player's line walks.

## Impact

- `exa_world` (new `nav::query`; `nav::Layer` derives `Hash`)
- `exa_game` (`steering`: the look-ahead reach, line-walk records, the snap,
  step targets; `player`: the grid in the behaviour, the snap, the
  footwork clamp, the hip ray removed; new `look_overlay`; `trace`: goal
  columns)
- `exa_physics` (`CharacterBody::feet`, a one-line accessor in a shared
  file)
- `docs/NAVIGATION.md`, `docs/PLAYER.md`, `docs/DEBUGGING.md`,
  `docs/formats/RSQ.md`, `docs/PLAN.md`, crate READMEs
