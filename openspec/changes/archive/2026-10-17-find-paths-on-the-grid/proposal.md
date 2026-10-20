## Why

Every level has the original's navigation grid (`build-the-navigation-grid`)
and its queries (`walk-grid-lines`), but nothing can plan on it: the duellist
walks straight at its target and no NPC can find its way round a wall. In the
original every NPC path comes from one planner on that grid, a windowed A\*
whose steps, costs and greedy order give the AI's routes their character
(hugging low-clearance floors, paying for other characters' footprints,
cutting inside corners), rationed to one plan a tick, and kept current by
the rectangles moving props and doors mark.

Phase 6 (AI), the master plan's lane D change M6 (milestone MS4 "working
dungeon"); it unblocks M7 (`steer-npcs-along-paths`), and gives M32 and M12
(props and doors) the grid refresh they mark.

## What Changes

- `exa_world::nav::path`: the planner as functions of the grid (no systems):
  start and goal within 253 cells and 10 cells inside the grid, a 256 × 256
  window; steps by the line walk's reach rising at most 40 cm and dropping
  at most 2 m into cells whose clearance passes the size class
  (180/(size + 1)); cost 10·Δh² + 5 or 7 + twice the clearance + the
  penalty shifted by the goal's shift, refused over 600; best first by cost
  plus ten times the 3-D distance (of equals the last met; a cell met again
  more cheaply takes the new way); success within the stop distance, give
  up at 4096 cells or with nothing waiting; the nearest cell met as the end,
  cut back before the first rough cell (64/(size + 1)) when the search
  failed and the end lies over 100 cm from the goal (or the goal's own test
  rejects it); corners (direction or layer changes) and string pulling with
  the original's cost-bounded line test (+10, penalties shifted once more);
  points at cell centres on their floors. `PathGoal` carries the AI's
  defaults (size class 3, shift 1, stop 10); `Nav` gives no path without a
  grid.
- `exa_world::nav::path::PlanSlot` and the `NavPlanSlot` resource: one plan a
  tick, the slot handed at each tick's end to the longest waiter.
- `exa_world::nav::footprint`: characters' footprints (63 / 32 / 16 on the
  penalty bytes, layer by height, 5 × 5 patch, none near the edge);
  `NavFootprints` keeps them per entity; `exa_game::footprints` stamps NPCs'
  every tick and the player's while it is not walking, lifts them for
  characters that leave.
- `exa_world::nav::dirty`: the dirty-rectangle API for later movers
  (`rect_around`, the 16-entry ring `queue_moved`, the merging `mark_dirty`,
  `refresh` restoring the floors as built, letting movers raise them with
  the original's rule and re-deriving the costs where a floor changed); the
  grid keeps its built floors beside the movers' (the straight walk reads
  the built ones, as the original's); the game refreshes each fixed tick
  (nothing marks a rectangle yet).
- Debug: `exa-tool nav <level> --from x,y,z --to x,y,z [--size --shift
  --stop]` plans, prints and draws a path; `exarust --debug-path x,y,z`
  plans from the player through the slot, logs and draws the path; the nav
  overlay marks penalised cells.
- Docs: NAVIGATION.md (Planning, Footprints, One plan a tick, Moving
  things), DEBUGGING.md, PLAYER.md, PLAN.md, crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and `context/re/exe.asm`, read for this change:
  the A\* (window, layer pick, expansion, cost, heuristic, open list,
  node limit, fallback, pull-back, corner walk, string pulling), its line
  test, the AI planner (slot, own footprint, follow rule), the slot
  hand-over in the simulation step's callbacks, the footprint stamp and
  removal and their callers (player and AI), the AI's default goal, the
  dirty-rectangle slots and the pathfinder's tick refresh; tables and
  constants read from the PE (`context/re/master-movement.md` §23).
- Corpus: arena and exanima01 grids from the installation (`exa-tool nav`,
  `shipped_levels_plan_paths`).
- Exarust in game through `gpu_run.sh` (light run with `--debug-path`).

Out of scope:
- NPCs following paths, re-planning, the busy countdown (M7), and anything
  calling the planner besides the debug probe;
- props and doors marking rectangles and raising floors (M32, M12); the
  original's full refresh after a countdown (its writer was not found);
- the goal tests the AI's states pass (ignoring height for wandering: M10);
- dormant AIs sliding along paths, the AI's activation.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `navigation-grid`: path planning (window, steps, cost and order, end,
  corners), character footprints, one plan a tick, moving things on the
  grid; the straight walk reads the built floors; no path without a grid.
- `asset-inspection-cli`: `exa-tool nav` plans a path between two points.
- `world-viewer`: `--debug-path` and the overlay's penalty marks.

## Impact

- `exa_world` (new `nav::path`, `nav::footprint`, `nav::dirty`; `nav::query`
  shares its step rule and reads built floors for the straight walk;
  `nav::view` gains `NavPlanSlot`, `NavFootprints`, the tick's hand-over and
  refresh, penalty marks)
- `exa_game` (new `footprints`, `path_probe`)
- `exa_tool` (`nav --from --to`; depends on `bevy` for its maths)
- `exarust` (`--debug-path`)
- `docs/NAVIGATION.md`, `docs/DEBUGGING.md`, `docs/PLAYER.md`, `docs/PLAN.md`,
  crate READMEs
